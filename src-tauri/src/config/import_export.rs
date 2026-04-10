use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::tunnel::types::{Connection, ConnectionsFile, TagsFile};

/// Exported configuration (passwords are excluded for security).
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub connections: Vec<Connection>,
    pub tags: TagsFile,
}

/// Export all connection configs + tags to a JSON string.
pub fn export_config(connections: &ConnectionsFile, tags: &TagsFile) -> Result<String> {
    let data = ExportData {
        version: "1.0".to_string(),
        connections: connections.connections.clone(),
        tags: tags.clone(),
    };
    serde_json::to_string_pretty(&data).context("Failed to serialize export data")
}

/// Import connection configs + tags from a JSON string.
/// Returns the parsed data; the caller is responsible for merging/replacing.
pub fn import_config(json: &str) -> Result<ExportData> {
    serde_json::from_str(json).context("Failed to parse import data")
}
