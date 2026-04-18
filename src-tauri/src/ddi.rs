//! Pre-mount the Developer Disk Image so the first call that needs it
//! (e.g. screenshot) isn't slow. Fire-and-forget from the UI.

use crate::error::Result;
use crate::muxd::Transport;

#[tauri::command]
pub async fn prime_ddi(udid: String, transport: Transport) -> Result<()> {
    let _ = transport; // pmd3 helper resolves transport via pair records + mDNS
    // Runs the helper's `mount-ddi` subcommand. Blocking child in a tokio
    // `spawn_blocking` so we don't stall the main Tauri thread on a
    // potentially-large personalized-DDI download.
    let udid = udid.clone();
    tauri::async_runtime::spawn_blocking(move || crate::pmd3::run("mount-ddi", &udid))
        .await
        .map_err(|e| crate::error::LinkdropError::ToolFailed {
            tool: "prime_ddi".into(),
            status: "join".into(),
            stderr: format!("{e:?}"),
        })??;
    Ok(())
}
