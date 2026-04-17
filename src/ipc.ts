import { invoke } from "@tauri-apps/api/core";
import type {
  AirPlayStatus,
  DeviceInfo,
  DeviceSummary,
  MountResult,
  PhotoEntry,
  ScreenshotResult,
} from "./types";

export const api = {
  listDevices: () => invoke<DeviceSummary[]>("list_devices"),
  getDeviceInfo: (udid: string) => invoke<DeviceInfo>("get_device_info", { udid }),
  mountDevice: (udid: string) => invoke<MountResult>("mount_device", { udid }),
  unmountDevice: () => invoke<void>("unmount_device"),
  listPhotos: (limit = 200) => invoke<PhotoEntry[]>("list_photos", { limit }),
  takeScreenshot: (udid: string, outputDir: string) =>
    invoke<ScreenshotResult>("take_screenshot", { udid, outputDir }),
  startAirplay: (serverName?: string) =>
    invoke<AirPlayStatus>("start_airplay", { serverName: serverName ?? null }),
  stopAirplay: () => invoke<AirPlayStatus>("stop_airplay"),
  airplayStatus: () => invoke<AirPlayStatus>("airplay_status"),
};
