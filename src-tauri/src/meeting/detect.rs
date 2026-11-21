//! Si accorge che è iniziata una chiamata: il microfono è in uso da un'altra app e c'è un'app da
//! chiamata aperta (Zoom, Teams, Webex, FaceTime, Slack...) o un browser in primo piano (Meet).
//! Nessun permesso: CoreAudio dice se il microfono è acceso, NSWorkspace quali app sono aperte.

/// App da chiamata riconosciute: (bundle id, nome mostrato).
#[cfg(target_os = "macos")]
const CALL_APPS: &[(&str, &str)] = &[
    ("us.zoom.xos", "Zoom"),
    ("com.microsoft.teams2", "Microsoft Teams"),
    ("com.microsoft.teams", "Microsoft Teams"),
    ("Cisco-Systems.Spark", "Webex"),
    ("com.webex.meetingmanager", "Webex"),
    ("com.apple.FaceTime", "FaceTime"),
    ("com.tinyspeck.slackmacgap", "Slack"),
    ("com.hnc.Discord", "Discord"),
    ("com.skype.skype", "Skype"),
    ("net.whatsapp.WhatsApp", "WhatsApp"),
];

/// Browser: contano solo se sono in primo piano (una chiamata Meet o Teams nel browser).
#[cfg(target_os = "macos")]
const BROWSERS: &[&str] = &[
    "com.google.Chrome",
    "com.apple.Safari",
    "company.thebrowser.Browser",
    "com.microsoft.edgemac",
    "org.mozilla.firefox",
    "com.brave.Browser",
    "com.operasoftware.Opera",
    "com.vivaldi.Vivaldi",
];

/// Il microfono predefinito è in uso da qualche processo. `None` se non si può sapere (altri sistemi).
pub fn mic_in_use() -> Option<bool> {
    imp::mic_in_use()
}

/// Il nome dell'app con cui probabilmente si sta chiamando, se ce n'è una.
pub fn call_app() -> Option<String> {
    imp::call_app()
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSString;

    use super::{BROWSERS, CALL_APPS};

    #[repr(C)]
    struct PropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    #[link(name = "CoreAudio", kind = "framework")]
    extern "C" {
        fn AudioObjectGetPropertyData(
            object: u32,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier: *const std::ffi::c_void,
            data_size: *mut u32,
            data: *mut std::ffi::c_void,
        ) -> i32;
    }

    const fn fourcc(s: &[u8; 4]) -> u32 {
        ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | s[3] as u32
    }
    const SYSTEM_OBJECT: u32 = 1;
    const SCOPE_GLOBAL: u32 = fourcc(b"glob");
    const DEFAULT_INPUT: u32 = fourcc(b"dIn ");
    const RUNNING_SOMEWHERE: u32 = fourcc(b"gone");

    fn get_u32(object: u32, selector: u32) -> Option<u32> {
        let addr = PropertyAddress {
            selector,
            scope: SCOPE_GLOBAL,
            element: 0,
        };
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: proprietà di sola lettura di 4 byte, buffer della misura giusta.
        let status = unsafe {
            AudioObjectGetPropertyData(
                object,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                (&mut value as *mut u32).cast(),
            )
        };
        (status == 0).then_some(value)
    }

    pub fn mic_in_use() -> Option<bool> {
        let device = get_u32(SYSTEM_OBJECT, DEFAULT_INPUT).filter(|d| *d != 0)?;
        get_u32(device, RUNNING_SOMEWHERE).map(|v| v != 0)
    }

    fn bundle_id(app: &AnyObject) -> Option<String> {
        // SAFETY: NSRunningApplication.bundleIdentifier, sola lettura.
        let id: Option<Retained<NSString>> = unsafe { msg_send![app, bundleIdentifier] };
        id.map(|s| s.to_string())
    }

    pub fn call_app() -> Option<String> {
        let cls = AnyClass::get(c"NSWorkspace")?;
        // SAFETY: NSWorkspace.sharedWorkspace, runningApplications e frontmostApplication, sola lettura.
        unsafe {
            let ws: Option<Retained<AnyObject>> = msg_send![cls, sharedWorkspace];
            let ws = ws?;
            let front: Option<Retained<AnyObject>> = msg_send![&*ws, frontmostApplication];
            let front_id = front.as_deref().and_then(bundle_id);
            // un'app da chiamata in primo piano vince; poi un browser in primo piano; poi una qualsiasi aperta
            if let Some((_, name)) = CALL_APPS
                .iter()
                .find(|(id, _)| front_id.as_deref() == Some(*id))
            {
                return Some(name.to_string());
            }
            if let (Some(f), Some(id)) = (front.as_deref(), front_id.as_deref()) {
                if BROWSERS.contains(&id) {
                    let name: Option<Retained<NSString>> = msg_send![f, localizedName];
                    return name.map(|n| n.to_string());
                }
            }
            let apps: Retained<AnyObject> = msg_send![&*ws, runningApplications];
            let count: usize = msg_send![&*apps, count];
            for i in 0..count {
                let app: Retained<AnyObject> = msg_send![&*apps, objectAtIndex: i];
                if let Some(id) = bundle_id(&app) {
                    if let Some((_, name)) = CALL_APPS.iter().find(|(c, _)| *c == id) {
                        return Some(name.to_string());
                    }
                }
            }
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    // ponytail: Windows e Linux senza rilevazione per ora; si avvia a mano.
    pub fn mic_in_use() -> Option<bool> {
        None
    }
    pub fn call_app() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    /// Sul Mac: `cargo test --lib detect_reads_this_mac -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn detect_reads_this_mac() {
        println!(
            "microfono in uso: {:?}, app da chiamata: {:?}",
            super::mic_in_use(),
            super::call_app()
        );
        assert!(super::mic_in_use().is_some());
        let rec = crate::audio::Recorder::start(None, |_| {}).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(800));
        let during = super::mic_in_use();
        drop(rec.stop());
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!(
            "con il microfono aperto: {during:?}, dopo: {:?}",
            super::mic_in_use()
        );
        assert_eq!(during, Some(true));
    }
}
