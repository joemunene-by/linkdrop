//! linkdrop backend — Tauri commands that wrap libimobiledevice + uxplay.

mod airplay;
mod apps;
mod device;
mod error;
mod muxd;
mod notifications;
mod photos;
mod pmd3;
mod screenshot;
mod wifi_sync;

use airplay::AirPlayState;
use notifications::NotificationsState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if let Ok(dir) = tauri::Manager::path(app).resource_dir() {
                pmd3::set_resource_dir(dir);
            }
            Ok(())
        })
        .manage(AirPlayState::default())
        .manage(NotificationsState::default())
        .invoke_handler(tauri::generate_handler![
            device::list_devices,
            device::get_device_info,
            photos::mount_device,
            photos::unmount_device,
            photos::list_photos,
            screenshot::take_screenshot,
            airplay::start_airplay,
            airplay::stop_airplay,
            airplay::airplay_status,
            wifi_sync::enable_wifi_sync,
            notifications::start_notifications,
            notifications::stop_notifications,
            apps::list_apps,
            apps::list_app_files,
            apps::pull_app_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
