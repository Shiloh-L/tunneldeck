use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Forward Rule (one port mapping within a connection) ──────────

/// A single port-forwarding rule: local_port → target_host:target_port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRule {
    pub id: String,
    /// Optional friendly name, e.g. "MySQL", "Redis"
    #[serde(default)]
    pub name: String,
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl ForwardRule {
    pub fn new(name: String, local_port: u16, target_host: String, target_port: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            local_port,
            target_host,
            target_port,
            enabled: true,
        }
    }
}

// ─── Connection (one SSH session with N forward rules) ────────────

/// SSH authentication method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Password,
    Key,
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Password
    }
}

/// An SSH connection configuration persisted in connections.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub auth_method: AuthMethod,
    /// Path to private key file (used when auth_method == Key)
    #[serde(default)]
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub forwards: Vec<ForwardRule>,
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

impl Connection {
    pub fn new(name: String, host: String, port: u16, username: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            host,
            port,
            username,
            auth_method: AuthMethod::Password,
            private_key_path: None,
            forwards: Vec::new(),
            auto_connect: false,
            tag_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Runtime status of a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    WaitingDuo,
    Connected,
    Reconnecting,
    Error,
}

/// Connection info sent to the frontend (config + live status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    #[serde(flatten)]
    pub config: Connection,
    pub status: ConnectionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
    #[serde(default)]
    pub running_forward_ids: Vec<String>,
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
    pub connection_id: String,
    pub connection_name: String,
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

/// File-level wrapper for connections.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionsFile {
    #[serde(default)]
    pub connections: Vec<Connection>,
}

/// File-level wrapper for tags.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagsFile {
    #[serde(default)]
    pub tags: Vec<Tag>,
}
