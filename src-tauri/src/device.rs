//! libimobiledevice wrappers: list paired devices and fetch device info.

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::{muxd_command, Transport};

fn run(tool: &'static str, args: &[&str], apt_pkg: &'static str, transport: Transport) -> Result<String> {
    let output = match muxd_command(tool, transport).args(args).output() {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LinkdropError::MissingTool(tool, apt_pkg));
        }
        Err(e) => return Err(e.into()),
    };

    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: tool.to_string(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

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
    // USB list via stock usbmuxd.
    let usb = run("idevice_id", &["-l"], "libimobiledevice-utils", Transport::Usb)
        .unwrap_or_default();

    // Wi-Fi list via netmuxd. If netmuxd is down, skip silently — USB still works.
    let wifi = run("idevice_id", &["-n"], "libimobiledevice-utils", Transport::Wifi)
        .unwrap_or_default();

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for line in usb.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if seen.insert(line.to_string()) {
            out.push(DeviceSummary {
                udid: line.to_string(),
                transport: Transport::Usb,
            });
        }
    }
    for line in wifi.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if seen.insert(line.to_string()) {
            out.push(DeviceSummary {
                udid: line.to_string(),
                transport: Transport::Wifi,
            });
        }
    }

    Ok(out)
}

#[tauri::command]
pub fn get_device_info(udid: String, transport: Transport) -> Result<DeviceInfo> {
    let summary = run(
        "ideviceinfo",
        &["-u", &udid],
        "libimobiledevice-utils",
        transport,
    )?;
    let values = parse_kv(&summary);

    let get = |k: &str| values.get(k).cloned().unwrap_or_default();

    let battery_percent = run(
        "ideviceinfo",
        &["-u", &udid, "-q", "com.apple.mobile.battery", "-k", "BatteryCurrentCapacity"],
        "libimobiledevice-utils",
        transport,
    )
    .ok()
    .and_then(|raw| raw.trim().parse::<u8>().ok());

    let (total_bytes, free_bytes) = read_storage(&udid, transport)?;

    Ok(DeviceInfo {
        udid,
        transport,
        name: get("DeviceName"),
        model: get("ProductName"),
        product_type: get("ProductType"),
        ios_version: get("ProductVersion"),
        serial: get("SerialNumber"),
        battery_percent,
        total_bytes,
        free_bytes,
    })
}

fn parse_kv(raw: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in raw.lines() {
        if let Some((k, v)) = line.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

fn read_storage(udid: &str, transport: Transport) -> Result<(Option<u64>, Option<u64>)> {
    let raw = match run(
        "ideviceinfo",
        &["-u", udid, "-q", "com.apple.disk_usage"],
        "libimobiledevice-utils",
        transport,
    ) {
        Ok(s) => s,
        Err(_) => return Ok((None, None)),
    };
    let kv = parse_kv(&raw);
    let total = kv.get("TotalDiskCapacity").and_then(|s| s.parse::<u64>().ok());
    let free = kv.get("AmountDataAvailable").and_then(|s| s.parse::<u64>().ok());
    Ok((total, free))
}
