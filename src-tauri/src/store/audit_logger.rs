use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::connection::types::AuditEntry;

/// Append-only JSONL audit logger with daily file rotation.
pub struct AuditLogger {
    logs_dir: PathBuf,
    retention_days: u32,
}

impl AuditLogger {
    pub fn new(logs_dir: PathBuf, retention_days: u32) -> Self {
        Self {
            logs_dir,
            retention_days,
        }
    }

    fn today_file(&self) -> PathBuf {
        let date = Local::now().format("%Y-%m-%d");
        self.logs_dir.join(format!("audit_{}.jsonl", date))
    }

    /// Append a single audit entry as one JSONL line.
    pub async fn append(&self, entry: &AuditEntry) -> Result<()> {
        let path = self.today_file();
        let mut line = serde_json::to_string(entry).context("Failed to serialize audit entry")?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .with_context(|| format!("Failed to open audit log {}", path.display()))?;

        file.write_all(line.as_bytes())
            .await
            .context("Failed to write audit log entry")?;
        file.flush().await.context("Failed to flush audit log")?;
        Ok(())
    }

    /// Read all entries from a specific date's log file.
    pub async fn read_date(&self, date: &str) -> Result<Vec<AuditEntry>> {
        let path = self.logs_dir.join(format!("audit_{}.jsonl", date));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let entries: Vec<AuditEntry> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(entries)
    }

    /// Read recent N days of logs.
    pub async fn read_recent(&self, days: u32) -> Result<Vec<AuditEntry>> {
        let mut all_entries = Vec::new();
        let today = Local::now().date_naive();
        for i in 0..days {
            let date = today - chrono::Duration::days(i as i64);
            let date_str = date.format("%Y-%m-%d").to_string();
            let mut entries = self.read_date(&date_str).await?;
            all_entries.append(&mut entries);
        }
        Ok(all_entries)
    }

    /// Clean up log files older than retention_days.
    pub async fn cleanup_old_logs(&self) -> Result<u32> {
        let cutoff = Local::now().date_naive() - chrono::Duration::days(self.retention_days as i64);
        let mut deleted = 0u32;

        let mut dir = tokio::fs::read_dir(&self.logs_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            let filename = entry.file_name();
            let name = filename.to_string_lossy();
            // Parse date from "audit_YYYY-MM-DD.jsonl"
            if let Some(date_str) = name.strip_prefix("audit_").and_then(|s| s.strip_suffix(".jsonl")) {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    if date < cutoff {
                        tokio::fs::remove_file(entry.path()).await?;
                        deleted += 1;
                    }
                }
            }
        }
        Ok(deleted)
    }
}
