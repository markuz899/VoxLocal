//! L'app in primo piano quando parte un dettato (ARCHITECTURE §12, ActiveAppProbe): finisce nella
//! cronologia e sceglie la modalità con le regole "app → modalità".

/// Nome leggibile dell'app che ha il focus ("Mail", "Slack", "Code"), se il sistema lo dice.
pub fn frontmost_app() -> Option<String> {
    imp::frontmost()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSString;

    pub fn frontmost() -> Option<String> {
        let cls = AnyClass::get(c"NSWorkspace")?;
        // SAFETY: NSWorkspace.sharedWorkspace.frontmostApplication.localizedName, tutte API di sola lettura.
        unsafe {
            let ws: Option<Retained<AnyObject>> = msg_send![cls, sharedWorkspace];
            let app: Option<Retained<AnyObject>> = msg_send![&*ws?, frontmostApplication];
            let name: Option<Retained<NSString>> = msg_send![&*app?, localizedName];
            Some(name?.to_string())
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    /// Il nome dell'eseguibile senza estensione ("slack", "OUTLOOK"), con l'iniziale maiuscola.
    pub fn frontmost() -> Option<String> {
        // SAFETY: chiamate Win32 di sola lettura; l'handle del processo viene sempre chiuso.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            let stem = std::path::Path::new(&path)
                .file_stem()?
                .to_string_lossy()
                .into_owned();
            let mut c = stem.chars();
            Some(c.next()?.to_uppercase().chain(c).collect())
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    // ponytail: su Linux (X11/Wayland) nessun modo portabile; la colonna resta vuota
    pub fn frontmost() -> Option<String> {
        None
    }
}

/// Regola "app → modalità": vince la prima il cui nome è contenuto in quello dell'app (senza maiuscole).
pub fn mode_for_app<'a>(app: &str, rules: &'a [crate::settings::AppMode]) -> Option<&'a str> {
    let app = app.to_lowercase();
    rules
        .iter()
        .find(|r| !r.app.trim().is_empty() && app.contains(&r.app.trim().to_lowercase()))
        .map(|r| r.mode.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppMode;

    #[test]
    fn first_matching_rule_wins_case_insensitive() {
        let rules = vec![
            AppMode {
                app: "slack".into(),
                mode: "message".into(),
            },
            AppMode {
                app: "Mail".into(),
                mode: "email".into(),
            },
            AppMode {
                app: " ".into(),
                mode: "raw".into(),
            },
        ];
        assert_eq!(mode_for_app("Slack", &rules), Some("message"));
        assert_eq!(
            mode_for_app("Microsoft Outlook Mail", &rules),
            Some("email")
        );
        assert_eq!(mode_for_app("Code", &rules), None);
    }

    /// Da lanciare a mano su un desktop: cargo test --lib frontmost -- --ignored --nocapture
    #[test]
    #[ignore]
    fn frontmost_reads_the_focused_app() {
        let app = frontmost_app();
        println!("app in primo piano: {app:?}");
        assert!(app.is_some());
    }
}
