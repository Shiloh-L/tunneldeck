use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Atomic JSON file store: read/write JSON with tmp+rename safety.
pub struct JsonStore {
    base_dir: PathBuf,
}

impl JsonStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Ensure the data directory exists.
    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.base_dir)
            .await
            .context("Failed to create data directory")?;
        fs::create_dir_all(self.base_dir.join("logs"))
            .await
            .context("Failed to create logs directory")?;
        Ok(())
    }

    fn file_path(&self, filename: &str) -> PathBuf {
        self.base_dir.join(filename)
    }

    /// Load a JSON file, returns Default if not found.
    pub async fn load<T: DeserializeOwned + Default>(&self, filename: &str) -> Result<T> {
        let path = self.file_path(filename);
        if !path.exists() {
            return Ok(T::default());
        }
        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let data: T = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(data)
    }

    /// Atomic save: write to .tmp then rename to avoid corruption.
    pub async fn save<T: Serialize>(&self, filename: &str, data: &T) -> Result<()> {
        let path = self.file_path(filename);
        let tmp_path = self.file_path(&format!("{}.tmp", filename));
        let json = serde_json::to_string_pretty(data)
            .context("Failed to serialize data")?;
        fs::write(&tmp_path, json.as_bytes())
            .await
            .with_context(|| format!("Failed to write tmp file {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .await
            .with_context(|| format!("Failed to rename {} -> {}", tmp_path.display(), path.display()))?;
        Ok(())
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
