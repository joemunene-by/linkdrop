export type Transport = "usb" | "wifi";

export interface DeviceSummary {
  udid: string;
  transport: Transport;
}

export interface DeviceInfo {
  udid: string;
  transport: Transport;
  name: string;
  model: string;
  product_type: string;
  ios_version: string;
  serial: string;
  battery_percent: number | null;
  total_bytes: number | null;
  free_bytes: number | null;
}

export interface PhotoEntry {
  path: string;
  name: string;
  size_bytes: number;
  kind: "image" | "video";
}

export interface MountResult {
  mount_point: string;
}

export interface ScreenshotResult {
  path: string;
}

export type AirPlayStatus = "Running" | "Stopped";

export interface AppEntry {
  bundle_id: string;
  name: string;
  version: string;
  has_file_sharing: boolean;
}

export interface AppFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size_bytes: number;
}
