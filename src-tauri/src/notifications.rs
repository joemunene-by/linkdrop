//! Stream iOS syslog via `idevicesyslog` and forward matching lines to the
//! frontend as Tauri events. Best-effort — `idevicesyslog` surfaces many
//! events besides user-facing notifications, so the frontend applies an
//! additional regex filter.

use std::io::{BufRead, BufReader};
use std::process::{Child, Stdio};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::error::{LinkdropError, Result};
use crate::muxd::{muxd_command, Transport};

const EVENT_NAME: &str = "syslog";

#[derive(Default)]
pub struct NotificationsState(pub Mutex<Option<Child>>);

#[tauri::command]
pub fn start_notifications(
    app: AppHandle,
    state: State<NotificationsState>,
    udid: String,
    transport: Transport,
) -> Result<()> {
    let mut guard = state.0.lock().expect("NotificationsState poisoned");
    if guard.is_some() {
        return Ok(());
    }

    let mut cmd = muxd_command("idevicesyslog", transport);
    if transport == Transport::Wifi {
        cmd.arg("-n");
    }
    cmd.args(["-u", &udid]);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LinkdropError::MissingTool("idevicesyslog", "libimobiledevice-utils")
            } else {
                e.into()
            }
        })?;

    let stdout = child.stdout.take().expect("stdout piped above");
    let app_clone = app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(|r| r.ok()) {
            let _ = app_clone.emit(EVENT_NAME, line);
        }
    });

    *guard = Some(child);
    Ok(())
}

#[tauri::command]
pub fn stop_notifications(state: State<NotificationsState>) -> Result<()> {
    let mut guard = state.0.lock().expect("NotificationsState poisoned");
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}
