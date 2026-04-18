//! Installed-user-app listing via pymobiledevice3's InstallationProxyService.
//! Transport-agnostic: USB and Wi-Fi both route through the pmd3 helper,
//! which picks the right lockdown path for the given UDID.

use serde::{Deserialize, Serialize};

use crate::error::{LinkdropError, Result};
use crate::muxd::Transport;
use crate::platform::DevicePlatform;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppEntry {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub has_file_sharing: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn list_apps(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
) -> Result<Vec<AppEntry>> {
    let _ = transport;
    match platform {
        DevicePlatform::Android => {
            let apps = crate::adb::list_apps(&udid)?;
            Ok(apps
                .into_iter()
                .map(|a| AppEntry {
                    bundle_id: a.bundle_id,
                    name: a.name,
                    version: a.version,
                    has_file_sharing: a.has_file_sharing,
                })
                .collect())
        }
        DevicePlatform::Ios => {
            let stdout = crate::pmd3::run("apps", &udid)?;
            serde_json::from_str::<Vec<AppEntry>>(stdout.trim()).map_err(|e| {
                LinkdropError::ParseError {
                    tool: "pmd3_helper apps".into(),
                    detail: format!("bad JSON: {e}"),
                }
            })
        }
    }
}

#[tauri::command]
pub fn list_app_files(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
    bundle_id: String,
    path: String,
) -> Result<Vec<AppFileEntry>> {
    let _ = transport;
    if platform == DevicePlatform::Android {
        // Android app "sandbox" equivalent is /sdcard/Android/data/<pkg>/
        let base = format!("/sdcard/Android/data/{}", bundle_id);
        let target = if path == "/" {
            base
        } else {
            format!("{}{}", base, path)
        };
        let files = crate::adb::list_dir(&udid, &target)?;
        return Ok(files
            .into_iter()
            .map(|f| AppFileEntry {
                name: f.name,
                path: f.path,
                is_dir: f.is_dir,
                size_bytes: f.size_bytes,
            })
            .collect());
    }
    let stdout = crate::pmd3::run_with_args("list-app-files", &[&udid, &bundle_id, &path])?;
    serde_json::from_str::<Vec<AppFileEntry>>(stdout.trim()).map_err(|e| {
        LinkdropError::ParseError {
            tool: "pmd3_helper list-app-files".into(),
            detail: format!("bad JSON: {e}"),
        }
    })
}

#[tauri::command]
pub fn push_app_file(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
    bundle_id: String,
    local: String,
    remote: String,
) -> Result<()> {
    let _ = transport;
    match platform {
        DevicePlatform::Android => crate::adb::push(&udid, &local, &remote)?,
        DevicePlatform::Ios => {
            crate::pmd3::run_with_args("push-app-file", &[&udid, &bundle_id, &local, &remote])?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pull_app_file(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
    bundle_id: String,
    remote: String,
    local: String,
) -> Result<()> {
    let _ = transport;
    match platform {
        DevicePlatform::Android => crate::adb::pull(&udid, &remote, &local)?,
        DevicePlatform::Ios => {
            crate::pmd3::run_with_args("pull-app-file", &[&udid, &bundle_id, &remote, &local])?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn install_app(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
    ipa_path: String,
) -> Result<()> {
    let _ = transport;
    tauri::async_runtime::spawn_blocking(move || match platform {
        DevicePlatform::Android => crate::adb::install(&udid, &ipa_path),
        DevicePlatform::Ios => {
            crate::pmd3::run_with_args("install-app", &[&udid, &ipa_path]).map(|_| ())
        }
    })
    .await
    .map_err(|e| LinkdropError::ToolFailed {
        tool: "install_app".into(),
        status: "join".into(),
        stderr: format!("{e:?}"),
    })??;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_app(
    udid: String,
    transport: Transport,
    platform: DevicePlatform,
    bundle_id: String,
) -> Result<()> {
    let _ = transport;
    tauri::async_runtime::spawn_blocking(move || match platform {
        DevicePlatform::Android => crate::adb::uninstall(&udid, &bundle_id),
        DevicePlatform::Ios => {
            crate::pmd3::run_with_args("uninstall-app", &[&udid, &bundle_id]).map(|_| ())
        }
    })
    .await
    .map_err(|e| LinkdropError::ToolFailed {
        tool: "uninstall_app".into(),
        status: "join".into(),
        stderr: format!("{e:?}"),
    })??;
    Ok(())
}
