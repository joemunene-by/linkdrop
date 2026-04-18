//! Installed-user-app listing via pymobiledevice3's InstallationProxyService.
//! Transport-agnostic: USB and Wi-Fi both route through the pmd3 helper,
//! which picks the right lockdown path for the given UDID.

use serde::{Deserialize, Serialize};

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppEntry {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
}

#[tauri::command]
pub fn list_apps(udid: String, transport: Transport) -> Result<Vec<AppEntry>> {
    let _ = transport; // pmd3 helper resolves transport from pair records + mDNS
    let stdout = crate::pmd3::run("apps", &udid)?;
    serde_json::from_str::<Vec<AppEntry>>(stdout.trim()).map_err(|e| LinkdropError::ParseError {
        tool: "pmd3_helper apps".into(),
        detail: format!("bad JSON: {e}"),
    })
}
