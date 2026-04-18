//! Capture a screenshot from the iPhone via idevicescreenshot.
//! Requires Developer Disk Image mounted (ideviceimagemounter) on recent iOS.

use std::path::PathBuf;

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::{muxd_command, Transport};

#[derive(Debug, Serialize)]
pub struct ScreenshotResult {
    pub path: PathBuf,
}

#[tauri::command]
pub fn take_screenshot(
    udid: String,
    transport: Transport,
    output_dir: String,
) -> Result<ScreenshotResult> {
    let dir = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&dir)?;

    let filename = format!(
        "linkdrop-{}-{}.png",
        &udid[..8.min(udid.len())],
        chrono_like_timestamp()
    );
    let out_path = dir.join(&filename);

    if transport == Transport::Wifi {
        let path_str = out_path.to_string_lossy().into_owned();
        crate::pmd3::run_with_args("screenshot", &[&udid, &path_str])?;
        return Ok(ScreenshotResult { path: out_path });
    }

    let result = muxd_command("idevicescreenshot", transport)
        .args(["-u", &udid, out_path.to_str().unwrap_or("screenshot.png")])
        .output();

    let output = match result {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LinkdropError::MissingTool(
                "idevicescreenshot",
                "libimobiledevice-utils",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: "idevicescreenshot".to_string(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(ScreenshotResult { path: out_path })
}

/// Minimal, dependency-free UTC timestamp (YYYYMMDD-HHMMSS) from system clock.
fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Fold epoch seconds into a readable stamp without external deps.
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    // 1970-01-01 is day 0. Simple incremental calendar walk.
    let mut year = 1970u32;
    let mut remaining = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let yd: u64 = if leap { 366 } else { 365 };
        if remaining < yd {
            break;
        }
        remaining -= yd;
        year += 1;
    }
    let months = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut month = 1u32;
    for (i, &dim) in months.iter().enumerate() {
        let dim = if i == 1 && leap { 29 } else { dim };
        if remaining < dim {
            break;
        }
        remaining -= dim;
        month += 1;
    }
    let day = (remaining + 1) as u32;
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, h, m, s
    )
}
