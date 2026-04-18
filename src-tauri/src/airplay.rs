//! Screen mirror. iPhone goes through an AirPlay receiver (uxplay); Android
//! goes through scrcpy. The UI talks to one pair of start/stop/status
//! commands; this module picks the right tool by platform.

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::error::{LinkdropError, Result};
use crate::platform::DevicePlatform;

#[derive(Default)]
pub struct AirPlayState(pub Mutex<Option<Child>>);

#[derive(Debug, Serialize, Clone, Copy)]
pub enum AirPlayStatus {
    Running,
    Stopped,
}

#[tauri::command]
pub fn start_airplay(
    state: State<AirPlayState>,
    server_name: Option<String>,
    udid: Option<String>,
    platform: Option<DevicePlatform>,
) -> Result<AirPlayStatus> {
    let mut guard = state.0.lock().expect("AirPlayState mutex poisoned");
    if guard.is_some() {
        return Ok(AirPlayStatus::Running);
    }

    // Android → scrcpy, iOS (or unspecified) → uxplay/AirPlay.
    let child = match platform {
        Some(DevicePlatform::Android) => {
            let mut cmd = Command::new("scrcpy");
            if let Some(u) = udid.as_deref() {
                cmd.args(["-s", u]);
            }
            match cmd
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(LinkdropError::MissingTool("scrcpy", "scrcpy"));
                }
                Err(e) => return Err(e.into()),
            }
        }
        _ => {
            let name = server_name.unwrap_or_else(|| "linkdrop".to_string());
            match Command::new("uxplay")
                .args(["-n", &name])
                .env("GST_REGISTRY_FORK", "no")
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(LinkdropError::MissingTool("uxplay", "uxplay"));
                }
                Err(e) => return Err(e.into()),
            }
        }
    };

    *guard = Some(child);
    Ok(AirPlayStatus::Running)
}

#[tauri::command]
pub fn stop_airplay(state: State<AirPlayState>) -> Result<AirPlayStatus> {
    let mut guard = state.0.lock().expect("AirPlayState mutex poisoned");
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(AirPlayStatus::Stopped)
}

#[tauri::command]
pub fn airplay_status(state: State<AirPlayState>) -> Result<AirPlayStatus> {
    let mut guard = state.0.lock().expect("AirPlayState mutex poisoned");
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                // process exited on its own
                *guard = None;
                Ok(AirPlayStatus::Stopped)
            }
            Ok(None) => Ok(AirPlayStatus::Running),
            Err(_) => Ok(AirPlayStatus::Stopped),
        }
    } else {
        Ok(AirPlayStatus::Stopped)
    }
}
