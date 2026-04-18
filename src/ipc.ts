import { invoke } from "@tauri-apps/api/core";
import type {
  AirPlayStatus,
  AppEntry,
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
};
