//! Device listing and info — uniformly routed through the pymobiledevice3
//! helper so USB and Wi-Fi work identically on Linux, macOS, and Windows.

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;

#[derive(Debug, Serialize, Clone)]
pub struct DeviceSummary {
    pub udid: String,
    pub transport: Transport,
}

#[derive(Debug, Serialize, Clone)]
pub struct DeviceInfo {
    pub udid: String,
    pub transport: Transport,
    pub name: String,
    pub model: String,
    pub product_type: String,
    pub ios_version: String,
    pub serial: String,
    pub battery_percent: Option<u8>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

#[tauri::command]
pub fn list_devices() -> Result<Vec<DeviceSummary>> {
    // Single cross-platform entry point via the pmd3 helper. On Linux we
    // could also shell out to `idevice_id -l` + `-n`, but pymobiledevice3's
    // usbmux + mobdev2 browsing works on Linux, macOS, and Windows with
    // the same invocation, and returns structured JSON.
    let stdout = crate::pmd3::run_with_args("list", &[])?;

    #[derive(serde::Deserialize)]
    struct Wire {
        udid: String,
        transport: Transport,
    }

    let wire: Vec<Wire> =
        serde_json::from_str(stdout.trim()).map_err(|e| LinkdropError::ParseError {
            tool: "pmd3_helper list".into(),
            detail: format!("bad JSON: {e}"),
        })?;

    Ok(wire
        .into_iter()
        .map(|w| DeviceSummary {
            udid: w.udid,
            transport: w.transport,
        })
        .collect())
}

#[tauri::command]
pub fn get_device_info(udid: String, transport: Transport) -> Result<DeviceInfo> {
    // pmd3 helper's `first_lockdown` tries USB then Wi-Fi, so this works
    // for either transport tag and on any OS with a usbmuxd equivalent.
    let stdout = crate::pmd3::run("info", &udid)?;
    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).map_err(|e| LinkdropError::ParseError {
            tool: "pmd3_helper info".into(),
            detail: format!("bad JSON: {e}"),
        })?;

    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let u8_opt =
        |k: &str| v.get(k).and_then(|x| x.as_u64()).and_then(|n| u8::try_from(n).ok());
    let u64_opt = |k: &str| v.get(k).and_then(|x| x.as_u64());

    Ok(DeviceInfo {
        udid: v.get("udid").and_then(|x| x.as_str()).unwrap_or(&udid).to_string(),
        transport,
        name: s("name"),
        model: s("model"),
        product_type: s("product_type"),
        ios_version: s("ios_version"),
        serial: s("serial"),
        battery_percent: u8_opt("battery_percent"),
        total_bytes: u64_opt("total_bytes"),
        free_bytes: u64_opt("free_bytes"),
    })
}

