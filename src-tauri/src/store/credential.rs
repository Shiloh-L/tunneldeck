use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Portable file-based credential store.
/// Passwords are base64-encoded in `credentials.json` inside the data directory.
static STORE: Mutex<Option<CredentialStore>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CredentialsFile {
    passwords: HashMap<String, String>,
}

#[derive(Debug)]
struct CredentialStore {
    file_path: PathBuf,
    data: CredentialsFile,
}

impl CredentialStore {
    fn load(file_path: PathBuf) -> Self {
        let data = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            CredentialsFile::default()
        };
        Self { file_path, data }
    }

    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.data)
            .context("Failed to serialize credentials")?;
        let tmp = self.file_path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes())
            .context("Failed to write credentials tmp file")?;
        std::fs::rename(&tmp, &self.file_path)
            .context("Failed to rename credentials file")?;
        Ok(())
    }
}

/// Initialize the credential store with the data directory path.
/// Must be called once at startup.
pub fn init(data_dir: &Path) {
    let file_path = data_dir.join("credentials.json");
    let store = CredentialStore::load(file_path);
    *STORE.lock().unwrap() = Some(store);
}

pub fn save_password(connection_id: &str, password: &str) -> Result<()> {
    let mut guard = STORE.lock().unwrap();
    let store = guard.as_mut().context("Credential store not initialized")?;
    store.data.passwords.insert(
        connection_id.to_string(),
        B64.encode(password.as_bytes()),
    );
    store.save()
}

pub fn load_password(connection_id: &str) -> Result<Option<String>> {
    let guard = STORE.lock().unwrap();
    let store = guard.as_ref().context("Credential store not initialized")?;
    match store.data.passwords.get(connection_id) {
        Some(encoded) => {
            let bytes = B64.decode(encoded)
                .context("Failed to decode stored password")?;
            Ok(Some(String::from_utf8(bytes)
                .context("Stored password is not valid UTF-8")?))
        }
        None => Ok(None),
    }
}

pub fn delete_password(connection_id: &str) -> Result<()> {
    let mut guard = STORE.lock().unwrap();
    let store = guard.as_mut().context("Credential store not initialized")?;
    store.data.passwords.remove(connection_id);
    store.save()
}
