//! Enable Wi-Fi sync on a USB-tethered iPhone by flipping the
//! `EnableWifiConnections` lockdown flag via the pmd3 helper. iOS keeps
//! the flag persistently — it only needs to be set once.

use crate::error::Result;

#[tauri::command]
pub async fn enable_wifi_sync(udid: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || crate::pmd3::run("wifi-enable", &udid))
        .await
        .map_err(|e| crate::error::LinkdropError::ToolFailed {
            tool: "enable_wifi_sync".into(),
            status: "join".into(),
            stderr: format!("{e:?}"),
        })??;
    Ok(())
}
