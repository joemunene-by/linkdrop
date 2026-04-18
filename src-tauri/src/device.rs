//! Device listing + info, unified across iPhone (pmd3) and Android (adb).

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;
use crate::platform::DevicePlatform;

#[derive(Debug, Serialize, Clone)]
pub struct DeviceSummary {
    pub udid: String,
    pub transport: Transport,
    pub platform: DevicePlatform,
}

#[derive(Debug, Serialize, Clone)]
pub struct DeviceInfo {
    pub udid: String,
    pub transport: Transport,
    pub platform: DevicePlatform,
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
    let mut out = Vec::new();

    // iOS via pmd3 (returns [{udid, transport}, ...])
    if let Ok(stdout) = crate::pmd3::run_with_args("list", &[]) {
        #[derive(serde::Deserialize)]
        struct Wire {
            udid: String,
            transport: Transport,
        }
        if let Ok(wire) = serde_json::from_str::<Vec<Wire>>(stdout.trim()) {
            for w in wire {
                out.push(DeviceSummary {
                    udid: w.udid,
                    transport: w.transport,
                    platform: DevicePlatform::Ios,
                });
            }
        }
    }

    // Android via adb (USB only — Wi-Fi ADB is out of scope for v1)
    if let Ok(serials) = crate::adb::list() {
        for s in serials {
            out.push(DeviceSummary {
                udid: s,
                transport: Transport::Usb,
                platform: DevicePlatform::Android,
            });
        }
    }

    Ok(out)
}

#[tauri::command]
pub fn get_device_info(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
) -> Result<DeviceInfo> {
    match platform {
        DevicePlatform::Ios => get_info_ios(&udid, transport),
        DevicePlatform::Android => get_info_android(&udid, transport),
    }
}

fn get_info_ios(udid: &str, transport: Transport) -> Result<DeviceInfo> {
    let stdout = crate::pmd3::run("info", udid)?;
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
        udid: v.get("udid").and_then(|x| x.as_str()).unwrap_or(udid).to_string(),
        transport,
        platform: DevicePlatform::Ios,
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

fn get_info_android(udid: &str, transport: Transport) -> Result<DeviceInfo> {
    let a = crate::adb::info(udid)?;
    Ok(DeviceInfo {
        udid: a.udid,
        transport,
        platform: DevicePlatform::Android,
        name: a.name,
        model: a.model.clone(),
        product_type: a.model,
        ios_version: a.android_version, // UI labels this generically
        serial: a.serial,
        battery_percent: a.battery_percent,
        total_bytes: a.total_bytes,
        free_bytes: a.free_bytes,
    })
}
