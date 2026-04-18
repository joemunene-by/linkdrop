//! iOS diagnostic pulls — crash reports and sysdiagnose. All via pmd3.

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;

#[tauri::command]
pub fn list_crash_reports(udid: String, transport: Transport) -> Result<Vec<String>> {
    let _ = transport;
    let stdout = crate::pmd3::run("crash-list", &udid)?;
    serde_json::from_str::<Vec<String>>(stdout.trim()).map_err(|e| LinkdropError::ParseError {
        tool: "pmd3_helper crash-list".into(),
        detail: format!("bad JSON: {e}"),
    })
}

#[tauri::command]
pub fn pull_crash_reports(udid: String, transport: Transport, dest_dir: String) -> Result<()> {
    let _ = transport;
    crate::pmd3::run_with_args("crash-pull", &[&udid, &dest_dir])?;
    Ok(())
}

#[tauri::command]
pub async fn create_backup(
    udid: String,
    transport: Transport,
    dest_dir: String,
) -> Result<()> {
    let _ = transport;
    tauri::async_runtime::spawn_blocking(move || {
        crate::pmd3::run_with_args("backup", &[&udid, &dest_dir])
    })
    .await
    .map_err(|e| LinkdropError::ToolFailed {
        tool: "create_backup".into(),
        status: "join".into(),
        stderr: format!("{e:?}"),
    })??;
    Ok(())
}

#[tauri::command]
pub async fn pull_sysdiagnose(
    udid: String,
    transport: Transport,
    dest_dir: String,
) -> Result<()> {
    let _ = transport;
    tauri::async_runtime::spawn_blocking(move || {
        crate::pmd3::run_with_args("sysdiagnose", &[&udid, &dest_dir])
    })
    .await
    .map_err(|e| LinkdropError::ToolFailed {
        tool: "pull_sysdiagnose".into(),
        status: "join".into(),
        stderr: format!("{e:?}"),
    })??;
    Ok(())
}
