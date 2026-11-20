//! Chiavi API nel portachiavi del sistema (Keychain, Credential Manager, Secret Service).
//! Mai su disco in chiaro. Override per sviluppo/CI: variabile `VOXLOCAL_<ID>_KEY`.

use anyhow::Result;

const SERVICE: &str = crate::IDENTIFIER;

fn env_var(id: &str) -> String {
    format!("VOXLOCAL_{}_KEY", id.to_uppercase().replace('-', "_"))
}

pub fn get(id: &str) -> Option<String> {
    if let Ok(v) = std::env::var(env_var(id)) {
        return Some(v).filter(|v| !v.is_empty());
    }
    keyring::Entry::new(SERVICE, id)
        .and_then(|e| e.get_password())
        .ok()
        .filter(|v| !v.is_empty())
}

pub fn set(id: &str, key: &str) -> Result<()> {
    Ok(keyring::Entry::new(SERVICE, id)?.set_password(key.trim())?)
}

pub fn delete(id: &str) -> Result<()> {
    match keyring::Entry::new(SERVICE, id)?.delete_credential() {
        Err(keyring::Error::NoEntry) => Ok(()),
        r => Ok(r?),
    }
}
