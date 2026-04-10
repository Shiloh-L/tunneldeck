use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSH tunnel configuration persisted in tunnels.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub jump_host: String,
    #[serde(default = "default_ssh_port")]
    pub jump_port: u16,
    pub username: String,
    pub target_host: String,
    pub target_port: u16,
    pub local_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_ssh_port() -> u16 {
    22
}

impl TunnelConfig {
    pub fn new(
        name: String,
        jump_host: String,
        jump_port: u16,
        username: String,
        target_host: String,
        target_port: u16,
        local_port: u16,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            jump_host,
            jump_port,
            username,
            target_host,
            target_port,
            local_port,
            auto_connect: false,
            tag_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Runtime status of a tunnel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    WaitingDuo,
    Connected,
    Reconnecting,
    Error,
}

/// Tunnel info sent to the frontend (config + live status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    #[serde(flatten)]
    pub config: TunnelConfig,
    pub status: TunnelStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Seconds since connected, if connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
}

/// Tag for organizing tunnels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
}

impl Tag {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
        }
    }
}

/// Audit log entry (one line in JSONL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub tunnel_id: String,
    pub tunnel_name: String,
    pub event: AuditEvent,
    pub message: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditEvent {
    Connected,
    Disconnected,
    Reconnected,
    Error,
    Created,
    Deleted,
    Updated,
}

/// Global application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub api_enabled: bool,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub api_port: u16,
    #[serde(default = "default_true")]
    pub auto_start_tunnels: bool,
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,
    #[serde(default = "default_max_reconnect")]
    pub max_reconnect_attempts: u32,
    #[serde(default = "default_log_retention")]
    pub log_retention_days: u32,
}

fn default_true() -> bool {
    true
}
fn default_health_interval() -> u64 {
    30
}
fn default_max_reconnect() -> u32 {
    10
}
fn default_log_retention() -> u32 {
    30
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_enabled: false,
            api_token: None,
            api_port: 0,
            auto_start_tunnels: true,
            health_check_interval_secs: 30,
            max_reconnect_attempts: 10,
            log_retention_days: 30,
        }
    }
}

/// File-level wrapper for tunnels.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelsFile {
    #[serde(default)]
    pub tunnels: Vec<TunnelConfig>,
}

/// File-level wrapper for tags.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagsFile {
    #[serde(default)]
    pub tags: Vec<Tag>,
}
