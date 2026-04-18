import { invoke } from "@tauri-apps/api/core";
import type {
  AirPlayStatus,
  AppEntry,
  AppFileEntry,
  DeviceInfo,
  DevicePlatform,
  DeviceSummary,
  MountResult,
  PhotoEntry,
  ScreenshotResult,
  Transport,
} from "./types";

export const api = {
  listDevices: () => invoke<DeviceSummary[]>("list_devices"),
  getDeviceInfo: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
  ) => invoke<DeviceInfo>("get_device_info", { udid, transport, platform }),
  mountDevice: (udid: string, transport: Transport) =>
    invoke<MountResult>("mount_device", { udid, transport }),
  unmountDevice: () => invoke<void>("unmount_device"),
  listPhotos: (
    udid: string | null,
    transport: Transport | null,
    platform: DevicePlatform | null,
    limit = 200,
  ) => invoke<PhotoEntry[]>("list_photos", { udid, transport, platform, limit }),
  pullPhoto: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    remote: string,
    local: string,
  ) =>
    invoke<void>("pull_photo", { udid, transport, platform, remote, local }),
  takeScreenshot: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    outputDir: string,
  ) =>
    invoke<ScreenshotResult>("take_screenshot", {
      udid,
      transport,
      platform,
      outputDir,
    }),
  startAirplay: (
    udid?: string,
    platform?: DevicePlatform,
    serverName?: string,
  ) =>
    invoke<AirPlayStatus>("start_airplay", {
      udid: udid ?? null,
      platform: platform ?? null,
      serverName: serverName ?? null,
    }),
  stopAirplay: () => invoke<AirPlayStatus>("stop_airplay"),
  airplayStatus: () => invoke<AirPlayStatus>("airplay_status"),
  enableWifiSync: (udid: string) => invoke<void>("enable_wifi_sync", { udid }),
  startNotifications: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
  ) =>
    invoke<void>("start_notifications", { udid, transport, platform }),
  stopNotifications: () => invoke<void>("stop_notifications"),
  saveSyslogToFile: (path: string, content: string) =>
    invoke<void>("save_syslog_to_file", { path, content }),
  listApps: (udid: string, transport: Transport, platform: DevicePlatform) =>
    invoke<AppEntry[]>("list_apps", { udid, transport, platform }),
  listCrashReports: (udid: string, transport: Transport) =>
    invoke<string[]>("list_crash_reports", { udid, transport }),
  pullCrashReports: (udid: string, transport: Transport, destDir: string) =>
    invoke<void>("pull_crash_reports", { udid, transport, destDir }),
  createBackup: (udid: string, transport: Transport, destDir: string) =>
    invoke<void>("create_backup", { udid, transport, destDir }),
  pullSysdiagnose: (udid: string, transport: Transport, destDir: string) =>
    invoke<void>("pull_sysdiagnose", { udid, transport, destDir }),
  pushAppFile: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    bundleId: string,
    local: string,
    remote: string,
  ) =>
    invoke<void>("push_app_file", {
      udid,
      transport,
      platform,
      bundleId,
      local,
      remote,
    }),
  primeDdi: (udid: string, transport: Transport) =>
    invoke<void>("prime_ddi", { udid, transport }),
  pullAppFile: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    bundleId: string,
    remote: string,
    local: string,
  ) =>
    invoke<void>("pull_app_file", {
      udid,
      transport,
      platform,
      bundleId,
      remote,
      local,
    }),
  installApp: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    ipaPath: string,
  ) => invoke<void>("install_app", { udid, transport, platform, ipaPath }),
  uninstallApp: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    bundleId: string,
  ) =>
    invoke<void>("uninstall_app", { udid, transport, platform, bundleId }),
  listAppFiles: (
    udid: string,
    transport: Transport,
    platform: DevicePlatform,
    bundleId: string,
    path: string,
  ) =>
    invoke<AppFileEntry[]>("list_app_files", {
      udid,
      transport,
      platform,
      bundleId,
      path,
    }),
};
