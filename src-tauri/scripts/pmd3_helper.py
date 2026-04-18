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
  pmd3_helper.py list-photos <udid> [limit]  # list DCIM entries via AFC
  pmd3_helper.py pull-photo <udid> <remote> <local>  # download one DCIM file
  pmd3_helper.py list-app-files <udid> <bundle_id> [path]  # list app sandbox entries
  pmd3_helper.py pull-app-file <udid> <bundle_id> <remote> <local>  # download from sandbox
"""

import asyncio
import json
import os
import sys
from pathlib import Path

from pymobiledevice3.lockdown import get_mobdev2_lockdowns
from pymobiledevice3.services.afc import AfcService
from pymobiledevice3.services.house_arrest import HouseArrestService
from pymobiledevice3.services.installation_proxy import InstallationProxyService
from pymobiledevice3.services.mobile_image_mounter import DeveloperDiskImageMounter
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


IMAGE_EXTS = {".jpg", ".jpeg", ".png", ".heic", ".heif", ".gif", ".webp"}
VIDEO_EXTS = {".mov", ".mp4", ".m4v"}


async def cmd_list_photos(udid: str, limit: int) -> None:
    lockdown = await first_lockdown(udid)
    out: list[dict] = []
    async with AfcService(lockdown) as afc:
        async for root, _dirs, files in afc.walk("/DCIM"):
            for name in files:
                if len(out) >= limit:
                    break
                ext = os.path.splitext(name)[1].lower()
                if ext in IMAGE_EXTS:
                    kind = "image"
                elif ext in VIDEO_EXTS:
                    kind = "video"
                else:
                    continue
                path = f"{root}/{name}" if not root.endswith("/") else f"{root}{name}"
                try:
                    st = await afc.stat(path)
                    size = int(st.get("st_size", 0))
                except Exception:
                    size = 0
                out.append(
                    {"path": path, "name": name, "size_bytes": size, "kind": kind}
                )
            if len(out) >= limit:
                break
    print(json.dumps(out))


async def cmd_pull_photo(udid: str, remote: str, local: str) -> None:
    lockdown = await first_lockdown(udid)
    async with AfcService(lockdown) as afc:
        await afc.pull(remote, local)
    print(json.dumps({"path": local}))


async def cmd_list_app_files(udid: str, bundle_id: str, remote_path: str) -> None:
    from pymobiledevice3.exceptions import PyMobileDevice3Exception

    lockdown = await first_lockdown(udid)
    try:
        afc = await HouseArrestService.create(
            lockdown, bundle_id=bundle_id, documents_only=True
        )
    except PyMobileDevice3Exception as e:
        if "InstallationLookupFailed" in str(e):
            raise SystemExit(
                f"{bundle_id} doesn't expose its sandbox (no UIFileSharingEnabled in its Info.plist)"
            )
        raise SystemExit(f"house_arrest failed: {e}")
    try:
        try:
            names = await afc.listdir(remote_path)
        except Exception as e:
            raise SystemExit(f"cannot list {remote_path}: {e}")
        out: list[dict] = []
        for name in names:
            if name in (".", ".."):
                continue
            sub = remote_path.rstrip("/") + "/" + name
            try:
                st = await afc.stat(sub)
            except Exception:
                continue
            is_dir = st.get("st_ifmt") == "S_IFDIR"
            out.append(
                {
                    "name": name,
                    "path": sub,
                    "is_dir": is_dir,
                    "size_bytes": 0 if is_dir else int(st.get("st_size", 0)),
                }
            )
        out.sort(key=lambda e: (not e["is_dir"], e["name"].lower()))
        print(json.dumps(out))
    finally:
        try:
            await afc.aclose()
        except Exception:
            pass


async def cmd_pull_app_file(udid: str, bundle_id: str, remote: str, local: str) -> None:
    lockdown = await first_lockdown(udid)
    afc = await HouseArrestService.create(lockdown, bundle_id=bundle_id)
    try:
        await afc.pull(remote, local)
        print(json.dumps({"path": local}))
    finally:
        try:
            await afc.aclose()
        except Exception:
            pass


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
                "has_file_sharing": bool(meta.get("UIFileSharingEnabled", False)),
            }
        )
    out.sort(key=lambda a: (not a["has_file_sharing"], a["name"].lower()))
    print(json.dumps(out))


DDI_IMAGE_TYPE = "Developer"
DDI_DIR_ENV = "LINKDROP_DDI_DIR"


def _default_ddi_dir() -> Path:
    if override := os.environ.get(DDI_DIR_ENV):
        return Path(override)
    return Path.home() / "linkdrop" / "ddi"


async def ensure_ddi_mounted(lockdown) -> None:
    mounter = DeveloperDiskImageMounter(lockdown)
    if await mounter.is_image_mounted(DDI_IMAGE_TYPE):
        return
    ddi_dir = _default_ddi_dir()
    dmg = ddi_dir / "DeveloperDiskImage.dmg"
    sig = ddi_dir / "DeveloperDiskImage.dmg.signature"
    if not dmg.exists() or not sig.exists():
        raise SystemExit(
            f"DDI not found at {ddi_dir}; expected DeveloperDiskImage.dmg + .signature"
        )
    await mounter.mount(dmg, sig)


async def cmd_screenshot(udid: str, output_path: str) -> None:
    lockdown = await first_lockdown(udid)
    await ensure_ddi_mounted(lockdown)
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
    if op == "list-photos":
        limit = int(argv[3]) if len(argv) >= 4 else 200
        asyncio.run(cmd_list_photos(udid, limit))
        return 0
    if op == "pull-photo":
        if len(argv) < 5:
            print("usage: pmd3_helper.py pull-photo <udid> <remote> <local>", file=sys.stderr)
            return 2
        asyncio.run(cmd_pull_photo(udid, argv[3], argv[4]))
        return 0
    if op == "list-app-files":
        if len(argv) < 4:
            print("usage: pmd3_helper.py list-app-files <udid> <bundle_id> [path]", file=sys.stderr)
            return 2
        path = argv[4] if len(argv) >= 5 else "/"
        asyncio.run(cmd_list_app_files(udid, argv[3], path))
        return 0
    if op == "pull-app-file":
        if len(argv) < 6:
            print("usage: pmd3_helper.py pull-app-file <udid> <bundle_id> <remote> <local>", file=sys.stderr)
            return 2
        asyncio.run(cmd_pull_app_file(udid, argv[3], argv[4], argv[5]))
        return 0
    handlers = {"info": cmd_info, "wifi-enable": cmd_wifi_enable, "apps": cmd_apps}
    if op not in handlers:
        print(f"unknown op: {op}", file=sys.stderr)
        return 2
    asyncio.run(handlers[op](udid))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
