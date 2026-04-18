//! linkdrop backend — Tauri commands that wrap libimobiledevice + uxplay.

mod airplay;
mod device;
mod error;
mod muxd;
mod photos;
mod screenshot;
mod wifi_sync;

use airplay::AirPlayState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AirPlayState::default())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
