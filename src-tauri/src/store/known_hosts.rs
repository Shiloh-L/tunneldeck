use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use ssh_key::PublicKey;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// TOFU (Trust On First Use) known hosts store.
/// Stores server public keys indexed by "host:port".
/// - First connection: auto-accept and persist.
/// - Subsequent connections: verify against stored key.
/// - Key mismatch: reject (possible MITM attack).
#[derive(Debug)]
pub struct KnownHostsStore {
    file_path: PathBuf,
    hosts: Mutex<HashMap<String, KnownHost>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownHost {
    pub key_type: String,
    pub key_base64: String,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct KnownHostsFile {
    hosts: HashMap<String, KnownHost>,
}

/// Result of checking a host key.
#[derive(Debug, Clone)]
pub enum HostKeyCheckResult {
    /// First time connecting — key accepted and stored.
    TrustedNew,
    /// Key matches the previously stored key.
    TrustedKnown,
    /// Key does NOT match the stored key — possible MITM!
    Mismatch {
        expected_type: String,
        expected_key: String,
        actual_type: String,
        actual_key: String,
    },
}

impl KnownHostsStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            file_path: data_dir.join("known_hosts.json"),
            hosts: Mutex::new(HashMap::new()),
        }
    }

    /// Load known hosts from disk.
    pub async fn load(&self) -> Result<()> {
        if !self.file_path.exists() {
            debug!("No known_hosts file found, starting fresh");
            return Ok(());
        }
        let content = tokio::fs::read_to_string(&self.file_path)
            .await
            .with_context(|| format!("Failed to read {}", self.file_path.display()))?;
        let file: KnownHostsFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", self.file_path.display()))?;
        *self.hosts.lock().await = file.hosts;
        info!("Loaded {} known hosts", self.hosts.lock().await.len());
        Ok(())
    }

    /// Save known hosts to disk (atomic write).
    async fn save(&self) -> Result<()> {
        let hosts = self.hosts.lock().await;
        let file = KnownHostsFile {
            hosts: hosts.clone(),
        };
        let json = serde_json::to_string_pretty(&file)
            .context("Failed to serialize known_hosts")?;
        let tmp_path = self.file_path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, json.as_bytes())
            .await
            .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
        tokio::fs::rename(&tmp_path, &self.file_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to rename {} -> {}",
                    tmp_path.display(),
                    self.file_path.display()
                )
            })?;
        Ok(())
    }

    /// Check a server's public key. Returns the check result.
    /// On TrustedNew, the key is automatically stored (TOFU).
    pub async fn check_host_key(
        &self,
        host_port: &str,
        server_key: &PublicKey,
    ) -> Result<HostKeyCheckResult> {
        let key_type = server_key.algorithm().as_str().to_string();
        let key_base64 = server_key
            .to_openssh()
            .unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();

        let mut hosts = self.hosts.lock().await;

        if let Some(existing) = hosts.get(host_port) {
            if existing.key_type == key_type && existing.key_base64 == key_base64 {
                // Key matches — update last_seen
                let mut updated = existing.clone();
                updated.last_seen = now;
                hosts.insert(host_port.to_string(), updated);
                drop(hosts);
                let _ = self.save().await;
                debug!("Known host key verified for {}", host_port);
                return Ok(HostKeyCheckResult::TrustedKnown);
            } else {
                // KEY MISMATCH — possible MITM!
                warn!(
                    "HOST KEY MISMATCH for {}! Expected {} {}, got {} {}",
                    host_port, existing.key_type, &existing.key_base64[..20.min(existing.key_base64.len())],
                    key_type, &key_base64[..20.min(key_base64.len())]
                );
                return Ok(HostKeyCheckResult::Mismatch {
                    expected_type: existing.key_type.clone(),
                    expected_key: existing.key_base64.clone(),
                    actual_type: key_type,
                    actual_key: key_base64,
                });
            }
        }

        // New host — TOFU: trust and store
        info!("New host key for {} ({}), trusting on first use", host_port, key_type);
        hosts.insert(
            host_port.to_string(),
            KnownHost {
                key_type,
                key_base64,
                first_seen: now.clone(),
                last_seen: now,
            },
        );
        drop(hosts);
        let _ = self.save().await;

        Ok(HostKeyCheckResult::TrustedNew)
    }
}
