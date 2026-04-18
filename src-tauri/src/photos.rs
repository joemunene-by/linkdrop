//! Photo browsing via pymobiledevice3 AFC — works uniformly on USB and
//! Wi-Fi across Linux, macOS, and Windows (no ifuse / FUSE dependency).

use std::path::PathBuf;

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;

#[derive(Debug, Serialize)]
pub struct PhotoEntry {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: &'static str, // "image" | "video"
}

#[derive(Debug, Serialize)]
pub struct MountResult {
    pub mount_point: PathBuf,
}

/// Kept for backwards compat with the UI — no-op now since we stream AFC
/// directly and don't need a local FUSE mount.
#[tauri::command]
pub fn mount_device(udid: String, transport: Transport) -> Result<MountResult> {
    let _ = (udid, transport);
    Ok(MountResult {
        mount_point: PathBuf::from("(afc)"),
    })
}

#[tauri::command]
pub fn unmount_device() -> Result<()> {
    Ok(())
}

#[tauri::command]
pub fn pull_photo(
    udid: String,
    transport: Transport,
    remote: String,
    local: String,
) -> Result<()> {
    let _ = transport;
    crate::pmd3::run_with_args("pull-photo", &[&udid, &remote, &local])?;
    Ok(())
}

#[tauri::command]
pub fn list_photos(
    udid: Option<String>,
    transport: Option<Transport>,
    limit: Option<usize>,
) -> Result<Vec<PhotoEntry>> {
    let _ = transport;
    let udid = udid.ok_or_else(|| LinkdropError::ParseError {
        tool: "list_photos".into(),
        detail: "udid required".into(),
    })?;
    let lim = limit.unwrap_or(200);
    let stdout = crate::pmd3::run_with_args("list-photos", &[&udid, &lim.to_string()])?;

    #[derive(serde::Deserialize)]
    struct WireEntry {
        path: String,
        name: String,
        size_bytes: u64,
        kind: String,
    }

    let wire: Vec<WireEntry> = serde_json::from_str(stdout.trim()).map_err(|e| {
        LinkdropError::ParseError {
            tool: "pmd3_helper list-photos".into(),
            detail: format!("bad JSON: {e}"),
        }
    })?;

    Ok(wire
        .into_iter()
        .map(|w| PhotoEntry {
            path: w.path,
            name: w.name,
            size_bytes: w.size_bytes,
            kind: match w.kind.as_str() {
                "video" => "video",
                _ => "image",
            },
        })
        .collect())
}
