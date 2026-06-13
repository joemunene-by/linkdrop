import { test, expect, beforeEach, mock } from "bun:test";

// Mock the Tauri core invoke before importing the module under test, so the
// `api` wrappers bind to our spy instead of the real bridge.
const invoke = mock(() => Promise.resolve(undefined));
mock.module("@tauri-apps/api/core", () => ({ invoke }));

const { api } = await import("./ipc");

beforeEach(() => {
  invoke.mockClear();
});

function lastCall() {
  return invoke.mock.calls[invoke.mock.calls.length - 1];
}

test("listDevices calls list_devices with no args", () => {
  api.listDevices();
  expect(lastCall()).toEqual(["list_devices"]);
});

test("getDeviceInfo forwards udid + transport", () => {
  api.getDeviceInfo("abc123", "usb");
  expect(lastCall()).toEqual(["get_device_info", { udid: "abc123", transport: "usb" }]);
});

test("listPhotos defaults the limit to 200", () => {
  api.listPhotos(null, null);
  expect(lastCall()).toEqual(["list_photos", { udid: null, transport: null, limit: 200 }]);
});

test("listPhotos honors an explicit limit", () => {
  api.listPhotos("u", "wifi", 50);
  expect(lastCall()).toEqual(["list_photos", { udid: "u", transport: "wifi", limit: 50 }]);
});

test("startAirplay coalesces a missing name to null", () => {
  api.startAirplay();
  expect(lastCall()).toEqual(["start_airplay", { serverName: null }]);
});

test("startAirplay passes a provided name", () => {
  api.startAirplay("Living Room");
  expect(lastCall()).toEqual(["start_airplay", { serverName: "Living Room" }]);
});

test("pushAppFile sends camelCase keys Tauri expects", () => {
  api.pushAppFile("u", "usb", "com.example.app", "/local/f", "/Documents/f");
  expect(lastCall()).toEqual([
    "push_app_file",
    {
      udid: "u",
      transport: "usb",
      bundleId: "com.example.app",
      local: "/local/f",
      remote: "/Documents/f",
    },
  ]);
});

test("takeScreenshot sends outputDir as camelCase", () => {
  api.takeScreenshot("u", "usb", "/tmp/shots");
  expect(lastCall()).toEqual([
    "take_screenshot",
    { udid: "u", transport: "usb", outputDir: "/tmp/shots" },
  ]);
});

test("listAppFiles forwards bundleId + path", () => {
  api.listAppFiles("u", "wifi", "com.example.app", "/Documents");
  expect(lastCall()).toEqual([
    "list_app_files",
    { udid: "u", transport: "wifi", bundleId: "com.example.app", path: "/Documents" },
  ]);
});

test("a void command (unmountDevice) invokes with no args", () => {
  api.unmountDevice();
  expect(lastCall()).toEqual(["unmount_device"]);
});

test("every api method is a function", () => {
  for (const [name, fn] of Object.entries(api)) {
    expect(typeof fn, name).toBe("function");
  }
});
