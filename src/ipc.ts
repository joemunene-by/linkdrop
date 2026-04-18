import { invoke } from "@tauri-apps/api/core";
import type {
  AirPlayStatus,
  AppEntry,
  AppFileEntry,
  DeviceInfo,
  DeviceSummary,
  MountResult,
  PhotoEntry,
  ScreenshotResult,
  Transport,
} from "./types";

export const api = {
  listDevices: () => invoke<DeviceSummary[]>("list_devices"),
  getDeviceInfo: (udid: string, transport: Transport) =>
    invoke<DeviceInfo>("get_device_info", { udid, transport }),
  mountDevice: (udid: string, transport: Transport) =>
    invoke<MountResult>("mount_device", { udid, transport }),
  unmountDevice: () => invoke<void>("unmount_device"),
  listPhotos: (
    udid: string | null,
    transport: Transport | null,
    limit = 200,
  ) => invoke<PhotoEntry[]>("list_photos", { udid, transport, limit }),
  pullPhoto: (
    udid: string,
    transport: Transport,
    remote: string,
    local: string,
  ) => invoke<void>("pull_photo", { udid, transport, remote, local }),
  takeScreenshot: (udid: string, transport: Transport, outputDir: string) =>
    invoke<ScreenshotResult>("take_screenshot", { udid, transport, outputDir }),
  startAirplay: (serverName?: string) =>
    invoke<AirPlayStatus>("start_airplay", { serverName: serverName ?? null }),
  stopAirplay: () => invoke<AirPlayStatus>("stop_airplay"),
  airplayStatus: () => invoke<AirPlayStatus>("airplay_status"),
  enableWifiSync: (udid: string) => invoke<void>("enable_wifi_sync", { udid }),
  startNotifications: (udid: string, transport: Transport) =>
    invoke<void>("start_notifications", { udid, transport }),
  stopNotifications: () => invoke<void>("stop_notifications"),
  listApps: (udid: string, transport: Transport) =>
    invoke<AppEntry[]>("list_apps", { udid, transport }),
  listCrashReports: (udid: string, transport: Transport) =>
    invoke<string[]>("list_crash_reports", { udid, transport }),
  pullCrashReports: (udid: string, transport: Transport, destDir: string) =>
    invoke<void>("pull_crash_reports", { udid, transport, destDir }),
  createBackup: (udid: string, transport: Transport, destDir: string) =>
    invoke<void>("create_backup", { udid, transport, destDir }),
  installApp: (udid: string, transport: Transport, ipaPath: string) =>
    invoke<void>("install_app", { udid, transport, ipaPath }),
  uninstallApp: (udid: string, transport: Transport, bundleId: string) =>
    invoke<void>("uninstall_app", { udid, transport, bundleId }),
  listAppFiles: (
    udid: string,
    transport: Transport,
    bundleId: string,
    path: string,
  ) =>
    invoke<AppFileEntry[]>("list_app_files", {
      udid,
      transport,
      bundleId,
      path,
    }),
  primeDdi: (udid: string, transport: Transport) =>
    invoke<void>("prime_ddi", { udid, transport }),
  pullAppFile: (
    udid: string,
    transport: Transport,
    bundleId: string,
    remote: string,
    local: string,
  ) =>
    invoke<void>("pull_app_file", {
      udid,
      transport,
      bundleId,
      remote,
      local,
    }),
};
