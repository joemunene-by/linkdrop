//! Enable Wi-Fi sync on a USB-tethered iPhone by flipping the
//! `EnableWifiConnections` lockdown flag in the
//! `com.apple.mobile.wireless_lockdown` domain. Device must be USB-connected
//! at the moment of the call. Once set, netmuxd picks the device up over mDNS.

use idevice::{
    lockdown::LockdownClient,
    provider::IdeviceProvider,
    usbmuxd::{UsbmuxdAddr, UsbmuxdConnection},
    IdeviceService,
};
use plist::Value;

use crate::error::{LinkdropError, Result};

fn fail(step: &'static str, e: impl std::fmt::Debug) -> LinkdropError {
    LinkdropError::ToolFailed {
        tool: "lockdown".into(),
        status: step.into(),
        stderr: format!("{e:?}"),
    }
}

#[tauri::command]
pub async fn enable_wifi_sync(udid: String) -> Result<()> {
    let mut muxer = UsbmuxdConnection::default()
        .await
        .map_err(|e| fail("usbmuxd_connect", e))?;

    let dev = muxer
        .get_device(&udid)
        .await
        .map_err(|e| fail("device_not_found_usb", e))?;

    let addr = UsbmuxdAddr::from_env_var().unwrap_or_default();
    let provider: Box<dyn IdeviceProvider> = Box::new(dev.to_provider(addr, "linkdrop"));

    let mut lockdown = LockdownClient::connect(&*provider)
        .await
        .map_err(|e| fail("lockdown_connect", e))?;

    let pair = provider
        .get_pairing_file()
        .await
        .map_err(|e| fail("pairing_file", e))?;

    lockdown
        .start_session(&pair)
        .await
        .map_err(|e| fail("start_session", e))?;

    lockdown
        .set_value(
            "EnableWifiConnections",
            Value::Boolean(true),
            Some("com.apple.mobile.wireless_lockdown"),
        )
        .await
        .map_err(|e| fail("set_value", e))?;

    Ok(())
}
