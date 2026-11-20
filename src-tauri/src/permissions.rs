//! Stato dei permessi per l'onboarding (ARCHITECTURE §9). Fuori da macOS non c'è niente da chiedere
//! all'utente: il microfono si scopre aprendolo, la simulazione dei tasti non richiede permessi.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Granted,
    Denied,
    /// Mai chiesto: il prompt di sistema compare al primo uso.
    Undetermined,
    /// La piattaforma non espone lo stato (Windows, Linux).
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub microphone: Access,
    pub accessibility: Access,
    /// true se la piattaforma richiede il permesso di Accessibilità (solo macOS).
    pub needs_accessibility: bool,
}

pub fn status() -> Status {
    Status {
        microphone: microphone(),
        accessibility: accessibility(),
        needs_accessibility: cfg!(target_os = "macos"),
    }
}

#[cfg(target_os = "macos")]
fn microphone() -> Access {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    use objc2_foundation::NSString;

    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {
        static AVMediaTypeAudio: &'static NSString;
    }

    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return Access::Unknown;
    };
    // AVAuthorizationStatus: 0 non determinato, 1 limitato, 2 negato, 3 concesso.
    let status: isize =
        unsafe { msg_send![cls, authorizationStatusForMediaType: AVMediaTypeAudio] };
    match status {
        0 => Access::Undetermined,
        1 | 2 => Access::Denied,
        3 => Access::Granted,
        _ => Access::Unknown,
    }
}

#[cfg(not(target_os = "macos"))]
fn microphone() -> Access {
    Access::Unknown
}

#[cfg(target_os = "macos")]
fn accessibility() -> Access {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    if unsafe { AXIsProcessTrusted() } {
        Access::Granted
    } else {
        Access::Denied
    }
}

#[cfg(not(target_os = "macos"))]
fn accessibility() -> Access {
    Access::Granted
}

/// Mostra il prompt di sistema per l'Accessibilità (enigo usa AXIsProcessTrustedWithOptions).
pub fn request_accessibility() {
    let settings = enigo::Settings {
        open_prompt_to_get_permissions: true,
        ..Default::default()
    };
    let _ = enigo::Enigo::new(&settings);
}

/// Apre il microfono per un istante: su macOS fa comparire il prompt di sistema.
pub fn request_microphone() -> Access {
    if let Ok(rec) = crate::audio::Recorder::start(None, |_| {}) {
        let _ = rec.stop();
    }
    microphone()
}

/// URL della pagina di Impostazioni di Sistema per il permesso indicato.
pub fn settings_url(which: &str) -> Option<&'static str> {
    match (cfg!(target_os = "macos"), cfg!(windows), which) {
        (true, _, "microphone") => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        }
        (true, _, "accessibility") => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        }
        (_, true, "microphone") => Some("ms-settings:privacy-microphone"),
        _ => None,
    }
}
