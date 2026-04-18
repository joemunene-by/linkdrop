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
    pub has_file_sharing: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
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

#[tauri::command]
pub fn list_app_files(
    udid: String,
    transport: Transport,
    bundle_id: String,
    path: String,
) -> Result<Vec<AppFileEntry>> {
    let _ = transport;
    let stdout = crate::pmd3::run_with_args("list-app-files", &[&udid, &bundle_id, &path])?;
    serde_json::from_str::<Vec<AppFileEntry>>(stdout.trim()).map_err(|e| {
        LinkdropError::ParseError {
            tool: "pmd3_helper list-app-files".into(),
            detail: format!("bad JSON: {e}"),
        }
    })
}

#[tauri::command]
pub fn pull_app_file(
    udid: String,
    transport: Transport,
    bundle_id: String,
    remote: String,
    local: String,
) -> Result<()> {
    let _ = transport;
    crate::pmd3::run_with_args("pull-app-file", &[&udid, &bundle_id, &remote, &local])?;
    Ok(())
}
