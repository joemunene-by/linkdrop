//! Android adapter — thin wrappers around the `adb` CLI. Install on each
//! OS: `sudo apt install adb` / `brew install android-platform-tools` /
//! `winget install Google.AndroidStudio` (or a standalone platform-tools
//! zip). linkdrop surfaces MissingTool errors if adb isn't on PATH.

use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

use crate::error::{LinkdropError, Result};

fn adb() -> Command {
    Command::new("adb")
}

fn run(args: &[&str]) -> Result<String> {
    let output = match adb().args(args).output() {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LinkdropError::MissingTool("adb", "android-tools (adb)"));
        }
        Err(e) => return Err(e.into()),
    };
    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: "adb".into(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// List serial numbers of ADB-connected, fully authorized devices.
pub fn list() -> Result<Vec<String>> {
    // `adb devices` returns lines like:
    //   List of devices attached
    //   ABC1234567\tdevice
    //   DEF8888888\tunauthorized
    //   emulator-5554\toffline
    // We keep only `device`-state entries.
    let raw = run(&["devices"])?;
    let mut out = Vec::new();
    for line in raw.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some(serial), Some("device")) => out.push(serial.to_string()),
            _ => {}
        }
    }
    Ok(out)
}

#[derive(Debug, Serialize)]
pub struct AndroidInfo {
    pub udid: String,
    pub name: String,
    pub model: String,
    pub android_version: String,
    pub serial: String,
    pub battery_percent: Option<u8>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

fn prop(udid: &str, key: &str) -> Option<String> {
    run(&["-s", udid, "shell", "getprop", key])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn info(udid: &str) -> Result<AndroidInfo> {
    let model = prop(udid, "ro.product.model").unwrap_or_default();
    let android_version = prop(udid, "ro.build.version.release").unwrap_or_default();
    let name = prop(udid, "ro.product.brand")
        .map(|b| format!("{b} {model}"))
        .unwrap_or_else(|| model.clone());

    // Battery via `dumpsys battery`
    let battery_percent = run(&["-s", udid, "shell", "dumpsys", "battery"])
        .ok()
        .and_then(|raw| {
            raw.lines()
                .find_map(|l| {
                    let l = l.trim();
                    l.strip_prefix("level:")
                        .or_else(|| l.strip_prefix("level :"))
                        .and_then(|s| s.trim().parse::<u8>().ok())
                })
        });

    // Storage via `df /sdcard` (first line is header, second is data)
    let (total_bytes, free_bytes) = run(&["-s", udid, "shell", "df", "/sdcard"])
        .ok()
        .and_then(|raw| {
            let mut line_iter = raw.lines().skip(1);
            let data = line_iter.next()?;
            let cols: Vec<&str> = data.split_whitespace().collect();
            // Filesystem 1K-blocks Used Available Use% Mounted-on
            let total_k: u64 = cols.get(1)?.parse().ok()?;
            let avail_k: u64 = cols.get(3)?.parse().ok()?;
            Some((Some(total_k * 1024), Some(avail_k * 1024)))
        })
        .unwrap_or((None, None));

    Ok(AndroidInfo {
        udid: udid.to_string(),
        name,
        model,
        android_version,
        serial: udid.to_string(),
        battery_percent,
        total_bytes,
        free_bytes,
    })
}

pub fn screenshot(udid: &str, output_path: &str) -> Result<()> {
    // `adb exec-out screencap -p` streams a PNG on stdout. Capture + write.
    let output = adb()
        .args(["-s", udid, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LinkdropError::MissingTool("adb", "android-tools (adb)")
            } else {
                e.into()
            }
        })?;
    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: "adb screencap".into(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    std::fs::write(output_path, output.stdout).map_err(|e| {
        LinkdropError::ToolFailed {
            tool: "adb screencap".into(),
            status: e.kind().to_string(),
            stderr: format!("write {}: {e}", output_path),
        }
    })?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct AndroidApp {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub has_file_sharing: bool,
}

pub fn list_apps(udid: &str) -> Result<Vec<AndroidApp>> {
    // Third-party packages: `pm list packages -3`
    let raw = run(&["-s", udid, "shell", "pm", "list", "packages", "-3"])?;
    let mut apps = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if let Some(pkg) = line.strip_prefix("package:") {
            let pkg = pkg.trim().to_string();
            // Cheap name = package's last segment, version via `dumpsys` is
            // expensive; skip for v1 and let the UI show package id.
            let name = pkg
                .rsplit('.')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| pkg.clone());
            apps.push(AndroidApp {
                bundle_id: pkg,
                name,
                version: String::new(),
                has_file_sharing: true, // Android exposes /sdcard/Android/data always
            });
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}

#[derive(Debug, Serialize, Clone)]
pub struct AndroidFile {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

pub fn list_photos(udid: &str, limit: usize) -> Result<Vec<AndroidFile>> {
    // `ls -la /sdcard/DCIM/Camera` — columns: perms, links, owner, group, size, date, time, name
    let raw = run(&["-s", udid, "shell", "ls", "-la", "/sdcard/DCIM/Camera"])?;
    let mut out = Vec::new();
    for line in raw.lines() {
        if out.len() >= limit {
            break;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 || cols[0].starts_with('d') || cols[0].starts_with("total") {
            continue;
        }
        let size: u64 = cols[4].parse().unwrap_or(0);
        let name = cols[7..].join(" ");
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        out.push(AndroidFile {
            name: name.clone(),
            path: format!("/sdcard/DCIM/Camera/{name}"),
            is_dir: false,
            size_bytes: size,
        });
    }
    Ok(out)
}

/// `ls -la <remote_path>` parsed into AndroidFile entries. Directories
/// included (caller can descend).
pub fn list_dir(udid: &str, remote_path: &str) -> Result<Vec<AndroidFile>> {
    let raw = run(&["-s", udid, "shell", "ls", "-la", remote_path])?;
    let mut out = Vec::new();
    for line in raw.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 || cols[0].starts_with("total") {
            continue;
        }
        let is_dir = cols[0].starts_with('d');
        let size: u64 = cols[4].parse().unwrap_or(0);
        let name = cols[7..].join(" ");
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        out.push(AndroidFile {
            name: name.clone(),
            path: if remote_path.ends_with('/') {
                format!("{}{}", remote_path, name)
            } else {
                format!("{}/{}", remote_path, name)
            },
            is_dir,
            size_bytes: if is_dir { 0 } else { size },
        });
    }
    Ok(out)
}

pub fn pull(udid: &str, remote: &str, local: &str) -> Result<()> {
    run(&["-s", udid, "pull", remote, local]).map(|_| ())
}

pub fn push(udid: &str, local: &str, remote: &str) -> Result<()> {
    run(&["-s", udid, "push", local, remote]).map(|_| ())
}

pub fn install(udid: &str, apk: &str) -> Result<()> {
    run(&["-s", udid, "install", "-r", apk]).map(|_| ())
}

pub fn uninstall(udid: &str, bundle_id: &str) -> Result<()> {
    run(&["-s", udid, "uninstall", bundle_id]).map(|_| ())
}

pub fn logcat_command(udid: &str) -> Command {
    let mut c = adb();
    c.args(["-s", udid, "logcat", "-v", "time"]);
    c
}

/// Placeholder referenced by Tauri command registry — returns OK if adb is
/// on PATH, otherwise MissingTool.
#[allow(dead_code)]
pub fn ensure_adb() -> Result<PathBuf> {
    match which_adb() {
        Some(p) => Ok(p),
        None => Err(LinkdropError::MissingTool("adb", "android-tools (adb)")),
    }
}

fn which_adb() -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(if cfg!(windows) { "adb.exe" } else { "adb" });
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}
