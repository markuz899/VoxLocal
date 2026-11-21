//! Audio delle app (Zoom, Meet, Teams...) su macOS con ScreenCaptureKit (macOS 13+).
//! Si cattura il display principale a 2x2 px, solo per avere l'audio: float32 non interleaved a 48 kHz.

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass};
use objc2_core_media::CMSampleBuffer;
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{
    SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamOutput,
    SCStreamOutputType,
};

pub const RATE: u32 = 48_000;

type Sink = Box<dyn FnMut(&[f32]) + Send>;

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "VoxLocalMeetingAudioOutput"]
    #[ivars = Mutex<Sink>]
    struct AudioOut;

    unsafe impl NSObjectProtocol for AudioOut {}

    unsafe impl SCStreamOutput for AudioOut {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn did_output(&self, _stream: &SCStream, sb: &CMSampleBuffer, kind: SCStreamOutputType) {
            if kind != SCStreamOutputType::Audio {
                return;
            }
            // SAFETY: il buffer è valido per la durata della callback; si copiano solo i byte del primo canale.
            unsafe {
                let frames = sb.num_samples().max(0) as usize;
                let Some(block) = sb.data_buffer() else { return };
                if frames == 0 || block.data_length() < frames * 4 {
                    return;
                }
                // non interleaved: i primi `frames` float sono il canale sinistro
                let mut left = vec![0f32; frames];
                let Some(dst) = NonNull::new(left.as_mut_ptr().cast()) else { return };
                if block.copy_data_bytes(0, frames * 4, dst) == 0 {
                    (self.ivars().lock().unwrap())(&left);
                }
            }
        }
    }
);

impl AudioOut {
    fn new(sink: Sink) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Mutex::new(sink));
        unsafe { msg_send![super(this), init] }
    }
}

pub struct SystemAudio {
    stream: Retained<SCStream>,
    _output: Retained<AudioOut>,
    stop: Arc<AtomicBool>,
}

// SAFETY: SCStream è thread-safe per start/stop/update (documentazione Apple); lo usiamo solo da lì.
unsafe impl Send for SystemAudio {}

fn shareable_content() -> Result<Retained<SCShareableContent>> {
    let (tx, rx) = mpsc::channel();
    let handler = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
        let c = unsafe { Retained::retain(content) };
        let msg = unsafe { err.as_ref() }.map(|e| e.localizedDescription().to_string());
        let _ = tx.send(c.ok_or_else(|| msg.unwrap_or_else(|| "nessun contenuto".into())));
    });
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&handler) };
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| anyhow!("ScreenCaptureKit non risponde"))?
        .map_err(|e| anyhow!("audio delle app non disponibile: {e}"))
}

/// Filtro sul display principale: include tutte le app aperte in questo momento.
fn display_filter() -> Result<Retained<SCContentFilter>> {
    let content = shareable_content()?;
    let display = unsafe { content.displays() }
        .firstObject()
        .ok_or_else(|| anyhow!("nessuno schermo da cui catturare l'audio"))?;
    Ok(unsafe {
        SCContentFilter::initWithDisplay_excludingWindows(
            SCContentFilter::alloc(),
            &display,
            &NSArray::new(),
        )
    })
}

impl SystemAudio {
    pub fn start(sink: impl FnMut(&[f32]) + Send + 'static) -> Result<Self> {
        if !permission(false) {
            permission(true);
            anyhow::bail!(
                "serve il permesso Registrazione schermo: Impostazioni di Sistema → Privacy e sicurezza → Registrazione schermo, attiva VoxLocal"
            );
        }
        let filter = display_filter()?;
        let config = unsafe { SCStreamConfiguration::new() };
        unsafe {
            config.setWidth(2);
            config.setHeight(2);
            config.setCapturesAudio(true);
            config.setSampleRate(RATE as isize);
            config.setChannelCount(2);
            // la voce di VoxLocal stessa (nessuna, oggi) resta fuori
            config.setExcludesCurrentProcessAudio(true);
        }
        let stream = unsafe {
            SCStream::initWithFilter_configuration_delegate(
                SCStream::alloc(),
                &filter,
                &config,
                None,
            )
        };
        let output = AudioOut::new(Box::new(sink));
        let proto: &ProtocolObject<dyn SCStreamOutput> = ProtocolObject::from_ref(&*output);
        let mut err: *mut NSError = std::ptr::null_mut();
        // `addStreamOutput:type:sampleHandlerQueue:error:` non è esposto senza dispatch2: coda nil = quella di default
        let ok: bool = unsafe {
            msg_send![&stream, addStreamOutput: proto, type: SCStreamOutputType::Audio, sampleHandlerQueue: std::ptr::null_mut::<AnyObject>(), error: &mut err]
        };
        anyhow::ensure!(ok, "impossibile ricevere l'audio delle app");

        let (tx, rx) = mpsc::channel();
        let started = RcBlock::new(move |e: *mut NSError| {
            let _ = tx.send(unsafe { e.as_ref() }.map(|e| e.localizedDescription().to_string()));
        });
        unsafe { stream.startCaptureWithCompletionHandler(Some(&started)) };
        if let Some(e) = rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| anyhow!("l'audio delle app non parte"))?
        {
            anyhow::bail!("l'audio delle app non parte: {e}");
        }

        // il filtro vede solo le app aperte all'avvio: lo si rinnova, così un'app aperta dopo (la chiamata) entra
        let stop = Arc::new(AtomicBool::new(false));
        let (s, st) = (stream.clone(), stop.clone());
        let s = SendStream(s);
        std::thread::Builder::new()
            .name("voxlocal-meeting-filter".into())
            .spawn(move || {
                let s = s;
                while !st.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(10));
                    if st.load(Ordering::Relaxed) {
                        break;
                    }
                    match display_filter() {
                        Ok(f) => unsafe { s.0.updateContentFilter_completionHandler(&f, None) },
                        Err(e) => log::warn!("rinnovo del filtro audio: {e:#}"),
                    }
                }
            })?;
        log::info!("audio delle app: cattura avviata");
        Ok(Self {
            stream,
            _output: output,
            stop,
        })
    }

    pub fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        let done = RcBlock::new(move |_e: *mut NSError| {
            let _ = tx.send(());
        });
        unsafe { self.stream.stopCaptureWithCompletionHandler(Some(&done)) };
        let _ = rx.recv_timeout(Duration::from_secs(3));
        log::info!("audio delle app: cattura fermata");
    }
}

struct SendStream(Retained<SCStream>);
// SAFETY: vedi `SystemAudio`.
unsafe impl Send for SendStream {}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// `request = true` apre la richiesta di sistema (una volta sola: poi si attiva dalle Impostazioni).
pub fn permission(request: bool) -> bool {
    unsafe {
        if request {
            CGRequestScreenCaptureAccess()
        } else {
            CGPreflightScreenCaptureAccess()
        }
    }
}
