//! Photo browsing via ifuse-mounted DCIM directory.
//! ifuse requires the device to be trusted on the host.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::{LinkdropError, Result};
use crate::muxd::{muxd_command, Transport};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "heic", "heif", "gif", "webp"];
const VIDEO_EXTENSIONS: &[&str] = &["mov", "mp4", "m4v"];

#[derive(Debug, Serialize)]
pub struct PhotoEntry {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: &'static str, // "image" | "video"
}

#[derive(Debug, Serialize)]
pub struct MountResult {
    pub mount_point: PathBuf,
}

fn ensure_mount_point() -> Result<PathBuf> {
    let mut base = dirs_cache_home()?;
    base.push("linkdrop");
    base.push("mount");
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

fn dirs_cache_home() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(xdg));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".cache"));
    }
    Err(LinkdropError::ParseError {
        tool: "env".into(),
        detail: "neither XDG_CACHE_HOME nor HOME is set".into(),
    })
}

#[tauri::command]
pub fn mount_device(udid: String, transport: Transport) -> Result<MountResult> {
    let mount_point = ensure_mount_point()?;

    // Unmount anything stale first (ignore errors).
    let _ = Command::new("fusermount")
        .args(["-u", mount_point.to_str().unwrap_or("")])
        .output();

    let output = match muxd_command("ifuse", transport)
        .args(["-u", &udid, mount_point.to_str().unwrap_or("")])
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LinkdropError::MissingTool("ifuse", "ifuse"));
        }
        Err(e) => return Err(e.into()),
    };

    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: "ifuse".into(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(MountResult { mount_point })
}

#[tauri::command]
pub fn unmount_device() -> Result<()> {
    let mount_point = ensure_mount_point()?;
    let _ = Command::new("fusermount")
        .args(["-u", mount_point.to_str().unwrap_or("")])
        .output();
    Ok(())
}

#[tauri::command]
pub fn list_photos(
    udid: Option<String>,
    transport: Option<Transport>,
    limit: Option<usize>,
) -> Result<Vec<PhotoEntry>> {
    if matches!(transport, Some(Transport::Wifi)) {
        let udid = udid.ok_or_else(|| LinkdropError::ParseError {
            tool: "list_photos".into(),
            detail: "udid required for Wi-Fi transport".into(),
        })?;
        return list_photos_wifi(&udid, limit.unwrap_or(200));
    }
    list_photos_usb(limit.unwrap_or(500))
}

fn list_photos_usb(limit: usize) -> Result<Vec<PhotoEntry>> {
    let mount_point = ensure_mount_point()?;
    let dcim = mount_point.join("DCIM");
    if !dcim.exists() {
        return Err(LinkdropError::ParseError {
            tool: "ifuse".into(),
            detail: "DCIM not found — is the device mounted?".into(),
        });
    }

    let mut entries: Vec<PhotoEntry> = Vec::new();

    walk_dir(&dcim, &mut |p| {
        if entries.len() >= limit {
            return;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let kind = match ext.as_deref() {
            Some(e) if IMAGE_EXTENSIONS.contains(&e) => "image",
            Some(e) if VIDEO_EXTENSIONS.contains(&e) => "video",
            _ => return,
        };
        let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        entries.push(PhotoEntry {
            path: p.to_string_lossy().into_owned(),
            name: p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            size_bytes: size,
            kind,
        });
    });

    Ok(entries)
}

fn list_photos_wifi(udid: &str, limit: usize) -> Result<Vec<PhotoEntry>> {
    let stdout = crate::pmd3::run_with_args("list-photos", &[udid, &limit.to_string()])?;

    #[derive(serde::Deserialize)]
    struct WireEntry {
        path: String,
        name: String,
        size_bytes: u64,
        kind: String,
    }

    let wire: Vec<WireEntry> = serde_json::from_str(stdout.trim()).map_err(|e| {
        LinkdropError::ParseError {
            tool: "pmd3_helper list-photos".into(),
            detail: format!("bad JSON: {e}"),
        }
    })?;

    Ok(wire
        .into_iter()
        .map(|w| PhotoEntry {
            path: w.path,
            name: w.name,
            size_bytes: w.size_bytes,
            kind: match w.kind.as_str() {
                "video" => "video",
                _ => "image",
            },
        })
        .collect())
}

fn walk_dir(root: &Path, visit: &mut impl FnMut(&Path)) {
    let read = match std::fs::read_dir(root) {
        Ok(r) => r,
        Err(_) => return,
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, visit);
        } else if path.is_file() {
            visit(&path);
        }
    }
}
