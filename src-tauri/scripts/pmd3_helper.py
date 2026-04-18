#!/usr/bin/env python3
"""linkdrop helper: non-interactive pymobiledevice3 bridge for Wi-Fi ops.

`pymobiledevice3`'s CLI `--mobdev2` mode falls through to an interactive
prompt when a device advertises multiple addresses; that doesn't work from
Tauri's non-TTY spawn. This helper uses `get_mobdev2_lockdowns` directly,
which (a) matches the WiFiMACAddress from mDNS against the pair records
under ~/.pymobiledevice3/ and (b) returns fully pair-verified
TcpLockdownClient instances ready for GetValue / SetValue.

Usage:
  pmd3_helper.py list                  # prints [udid, ...] on stdout
  pmd3_helper.py info <udid>           # prints DeviceInfo JSON on stdout
  pmd3_helper.py wifi-enable <udid>    # flips EnableWifiConnections
  pmd3_helper.py screenshot <udid> <path>  # writes PNG; prints {path,bytes}
  pmd3_helper.py apps <udid>           # prints [{bundle_id,name,version}] for user apps
"""

import asyncio
import json
import sys

from pymobiledevice3.lockdown import get_mobdev2_lockdowns
from pymobiledevice3.services.installation_proxy import InstallationProxyService
from pymobiledevice3.services.screenshot import ScreenshotService


LIST_TIMEOUT = 1.5  # keep short — called from list_devices on a 5 s UI poll
INFO_TIMEOUT = 4.0


async def first_lockdown(udid: str):
    """Return the first pair-verified Wi-Fi lockdown client for `udid`."""
    async for _, lockdown in get_mobdev2_lockdowns(
        udid=udid, only_paired=True, timeout=INFO_TIMEOUT
    ):
        return lockdown
    raise SystemExit(f"no paired Wi-Fi device {udid} found on LAN")


async def cmd_list() -> None:
    seen: list[str] = []
    async for ident, lockdown in get_mobdev2_lockdowns(
        only_paired=True, timeout=LIST_TIMEOUT
    ):
        udid = (
            getattr(lockdown, "udid", None)
            or (lockdown.all_values or {}).get("UniqueDeviceID")
            or ident
        )
        if udid and udid not in seen:
            seen.append(udid)
        try:
            await lockdown.close()
        except Exception:
            pass
    print(json.dumps(seen))


async def cmd_info(udid: str) -> None:
    lockdown = await first_lockdown(udid)
    info = lockdown.all_values or {}
    out = {
        "udid": lockdown.udid or udid,
        "name": info.get("DeviceName", ""),
        "model": info.get("ProductName", ""),
        "product_type": info.get("ProductType", ""),
        "ios_version": info.get("ProductVersion", ""),
        "serial": info.get("SerialNumber", ""),
        "battery_percent": None,
        "total_bytes": None,
        "free_bytes": None,
    }
    try:
        battery = await lockdown.get_value(
            domain="com.apple.mobile.battery", key="BatteryCurrentCapacity"
        )
        if battery is not None:
            out["battery_percent"] = int(battery)
    except Exception:
        pass
    try:
        du = await lockdown.get_value(domain="com.apple.disk_usage")
        if isinstance(du, dict):
            out["total_bytes"] = du.get("TotalDiskCapacity")
            out["free_bytes"] = du.get("AmountDataAvailable")
    except Exception:
        pass
    print(json.dumps(out))


async def cmd_wifi_enable(udid: str) -> None:
    lockdown = await first_lockdown(udid)
    await lockdown.set_value(
        domain="com.apple.mobile.wireless_lockdown",
        key="EnableWifiConnections",
        value=True,
    )
    print(json.dumps({"ok": True}))


async def cmd_apps(udid: str) -> None:
    lockdown = await first_lockdown(udid)
    service = InstallationProxyService(lockdown)
    apps = await service.get_apps(application_type="User")
    out = []
    for bundle_id, meta in apps.items():
        out.append(
            {
                "bundle_id": bundle_id,
                "name": meta.get("CFBundleDisplayName") or meta.get("CFBundleName") or bundle_id,
                "version": meta.get("CFBundleShortVersionString") or meta.get("CFBundleVersion") or "",
            }
        )
    out.sort(key=lambda a: a["name"].lower())
    print(json.dumps(out))


async def cmd_screenshot(udid: str, output_path: str) -> None:
    lockdown = await first_lockdown(udid)
    service = ScreenshotService(lockdown)
    data = await service.take_screenshot()
    with open(output_path, "wb") as f:
        f.write(data)
    print(json.dumps({"path": output_path, "bytes": len(data)}))


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: pmd3_helper.py <list|info|wifi-enable> [<udid>]", file=sys.stderr)
        return 2
    op = argv[1]
    udid = argv[2] if len(argv) >= 3 else ""
    if op == "list":
        asyncio.run(cmd_list())
        return 0
    if op == "screenshot":
        if len(argv) < 4:
            print("usage: pmd3_helper.py screenshot <udid> <output-path>", file=sys.stderr)
            return 2
        asyncio.run(cmd_screenshot(udid, argv[3]))
        return 0
    handlers = {"info": cmd_info, "wifi-enable": cmd_wifi_enable, "apps": cmd_apps}
    if op not in handlers:
        print(f"unknown op: {op}", file=sys.stderr)
        return 2
    asyncio.run(handlers[op](udid))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
