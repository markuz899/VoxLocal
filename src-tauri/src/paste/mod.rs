//! Incolla nell'app in primo piano: clipboard + Cmd/Ctrl+V simulato, poi ripristino.

use std::time::Duration;

use anyhow::{anyhow, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Scrive `text` nella clipboard e restituisce il testo che c'era prima (se era testo).
pub fn set_clipboard(text: &str) -> Result<Option<String>> {
    let mut cb = arboard::Clipboard::new()?;
    let previous = cb.get_text().ok();
    cb.set_text(text)?;
    Ok(previous)
}

/// Simula la scorciatoia di incolla. Su macOS va chiamata dal main thread (DECISIONS D9).
pub fn send_paste_keys() -> Result<()> {
    send_shortcut('v')
}

/// Simula la scorciatoia di copia (per il comando sulla selezione). Main thread su macOS.
pub fn send_copy_keys() -> Result<()> {
    send_shortcut('c')
}

/// ⌘ + lettera su macOS, Ctrl + lettera altrove. La sorgente degli eventi è privata (enigo
/// `independent_of_keyboard_state`), quindi i tasti della scorciatoia tenuti premuti non si sommano.
fn send_shortcut(letter: char) -> Result<()> {
    if !can_simulate_keys() {
        return Err(anyhow!("simulazione dei tasti non disponibile (Wayland)"));
    }
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow!("{e:?}"))?;
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    enigo.key(modifier, Direction::Press)?;
    let res = enigo.key(Key::Unicode(letter), Direction::Click);
    enigo.key(modifier, Direction::Release)?;
    Ok(res?)
}

/// Il testo negli appunti, se ce n'è.
pub fn get_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Dopo `delay` rimette il testo precedente, ma solo se nella clipboard c'è ancora
/// il nostro: se nel frattempo l'utente ha copiato altro, non lo tocchiamo.
// ponytail: ripristina solo testo; immagini/file in clipboard vanno persi (OPEN_QUESTIONS Q4)
pub fn restore_later(previous: Option<String>, ours: String, delay: Duration) {
    let Some(previous) = previous else { return };
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if cb.get_text().ok().as_deref() == Some(ours.as_str()) {
                let _ = cb.set_text(previous);
            }
        }
    });
}

fn can_simulate_keys() -> bool {
    !(cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some())
}
