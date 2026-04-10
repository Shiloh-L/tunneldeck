use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::tunnel::types::{TagsFile, TunnelConfig, TunnelsFile};

/// Exported configuration (passwords are excluded for security).
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub tunnels: Vec<TunnelConfig>,
    pub tags: crate::tunnel::types::TagsFile,
}

/// Export all tunnel configs + tags to a JSON string.
pub fn export_config(tunnels: &TunnelsFile, tags: &TagsFile) -> Result<String> {
    let data = ExportData {
        version: "1.0".to_string(),
        tunnels: tunnels.tunnels.clone(),
        tags: tags.clone(),
    };
    serde_json::to_string_pretty(&data).context("Failed to serialize export data")
}

/// Import tunnel configs + tags from a JSON string.
/// Returns the parsed data; the caller is responsible for merging/replacing.
pub fn import_config(json: &str) -> Result<ExportData> {
    serde_json::from_str(json).context("Failed to parse import data")
}
