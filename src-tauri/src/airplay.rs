//! AirPlay screen/audio mirror via uxplay. The receiver opens its own window
//! (SDL-based) when mirroring begins on the iPhone.

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::error::{LinkdropError, Result};

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
) -> Result<AirPlayStatus> {
    let mut guard = state.0.lock().expect("AirPlayState mutex poisoned");
    if guard.is_some() {
        return Ok(AirPlayStatus::Running);
    }

    let name = server_name.unwrap_or_else(|| "linkdrop".to_string());

    let child = match Command::new("uxplay")
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
