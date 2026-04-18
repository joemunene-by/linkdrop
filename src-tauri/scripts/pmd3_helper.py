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
  pmd3_helper.py mount-ddi <udid>      # auto-mount DeveloperDiskImage (classic or personalized)
  pmd3_helper.py screenshot <udid> <path>  # writes PNG; prints {path,bytes}
  pmd3_helper.py apps <udid>           # prints [{bundle_id,name,version}] for user apps
  pmd3_helper.py list-photos <udid> [limit]  # list DCIM entries via AFC
  pmd3_helper.py pull-photo <udid> <remote> <local>  # download one DCIM file
  pmd3_helper.py list-app-files <udid> <bundle_id> [path]  # list app sandbox entries
  pmd3_helper.py pull-app-file <udid> <bundle_id> <remote> <local>  # download from sandbox
  pmd3_helper.py crash-list <udid>     # list crash report filenames
  pmd3_helper.py crash-pull <udid> <dest_dir>  # copy all crash reports to local dir
  pmd3_helper.py install-app <udid> <ipa_path>  # install .ipa
  pmd3_helper.py uninstall-app <udid> <bundle_id>  # uninstall by bundle id
  pmd3_helper.py backup <udid> <dest_dir>  # MobileBackup2 full backup
  pmd3_helper.py sysdiagnose <udid> <dest_dir>  # trigger + pull new sysdiagnose
  pmd3_helper.py push-app-file <udid> <bundle_id> <local> <remote>  # upload to sandbox
"""

import asyncio
import contextlib
import io
import json
import os
import sys
import traceback
from pathlib import Path

from pymobiledevice3.lockdown import create_using_usbmux, get_mobdev2_lockdowns
from pymobiledevice3.usbmux import list_devices as usbmux_list_devices
from pymobiledevice3.services.afc import AfcService
from pymobiledevice3.services.crash_reports import CrashReportsManager
from pymobiledevice3.services.house_arrest import HouseArrestService
from pymobiledevice3.services.installation_proxy import InstallationProxyService
from pymobiledevice3.services.mobilebackup2 import Mobilebackup2Service
from pymobiledevice3.services.mobile_image_mounter import (
    DeveloperDiskImageMounter,
    auto_mount,
)
from pymobiledevice3.services.screenshot import ScreenshotService


LIST_TIMEOUT = 2.0  # short — browse runs on every cmd_list
INFO_TIMEOUT = 6.0  # longer — cmd_list's cache is usually a miss, browse must succeed

# Daemon-mode session cache: udid → pair-verified lockdown client. Kept
# open across calls so repeated ops (info, apps, screenshot, ...) reuse
# one TLS handshake instead of reconnecting each time. One-shot CLI mode
# bypasses this (fresh Python interpreter per call) so there's no state
# leak when running pmd3_helper.py as a subprocess.
_LOCKDOWN_CACHE: dict = {}


async def first_lockdown(udid: str):
    """Return a pair-verified lockdown client for `udid`.

    Cache hit → quick health check + return; cache miss → try USB, then
    Wi-Fi via mDNS; stash the result before returning.
    """
    cached = _LOCKDOWN_CACHE.get(udid)
    if cached is not None:
        try:
            # Cheap liveness probe — any already-session'd value works.
            await cached.get_value(key="ProductVersion")
            return cached
        except Exception:
            _LOCKDOWN_CACHE.pop(udid, None)

    # USB path — fastest when the cable is plugged in.
    try:
        client = await create_using_usbmux(serial=udid)
        _LOCKDOWN_CACHE[udid] = client
        return client
    except Exception:
        pass

    # Wi-Fi path via Bonjour.
    async for _, lockdown in get_mobdev2_lockdowns(
        udid=udid, only_paired=True, timeout=INFO_TIMEOUT
    ):
        _LOCKDOWN_CACHE[udid] = lockdown
        return lockdown

    raise SystemExit(f"device {udid} not found on USB or Wi-Fi")


async def cmd_list() -> None:
    """Emit [{udid, transport}, ...] — USB first, Wi-Fi second, dedup'd.

    In daemon mode, Wi-Fi lockdown clients are stashed in
    `_LOCKDOWN_CACHE` so subsequent ops (info, apps, screenshot, ...)
    skip the Bonjour browse entirely.
    """
    out: list[dict] = []
    seen: set[str] = set()

    # USB via usbmuxd / AMDS / native daemon
    try:
        for dev in await usbmux_list_devices():
            udid = getattr(dev, "serial", None) or getattr(dev, "udid", None)
            if udid and udid not in seen:
                seen.add(udid)
                out.append({"udid": udid, "transport": "usb"})
    except Exception:
        pass

    # Wi-Fi via mDNS — keep the returned lockdown clients cached per UDID.
    try:
        async for ident, lockdown in get_mobdev2_lockdowns(
            only_paired=True, timeout=LIST_TIMEOUT
        ):
            udid = (
                getattr(lockdown, "udid", None)
                or (lockdown.all_values or {}).get("UniqueDeviceID")
                or ident
            )
            if udid and udid not in seen:
                seen.add(udid)
                out.append({"udid": udid, "transport": "wifi"})
            # Stash if we don't already have one; otherwise close the fresh one.
            if udid and udid not in _LOCKDOWN_CACHE:
                _LOCKDOWN_CACHE[udid] = lockdown
            else:
                try:
                    await lockdown.close()
                except Exception:
                    pass
    except Exception:
        pass

    print(json.dumps(out))


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


async def cmd_push_app_file(udid: str, bundle_id: str, local: str, remote: str) -> None:
    lockdown = await first_lockdown(udid)
    afc = await HouseArrestService.create(lockdown, bundle_id=bundle_id)
    try:
        await afc.push(local, remote)
        print(json.dumps({"ok": True, "remote": remote}))
    finally:
        try:
            await afc.aclose()
        except Exception:
            pass


async def cmd_sysdiagnose(udid: str, dest_dir: str) -> None:
    os.makedirs(dest_dir, exist_ok=True)
    lockdown = await first_lockdown(udid)
    async with CrashReportsManager(lockdown) as crashes:
        name = await crashes.get_new_sysdiagnose(out=dest_dir)
    print(json.dumps({"ok": True, "file": str(name)}))


async def cmd_crash_list(udid: str) -> None:
    lockdown = await first_lockdown(udid)
    async with CrashReportsManager(lockdown) as crashes:
        entries = await crashes.ls("/")
    # filter out dirs (those look like "ReportedCrashes/" or similar) — keep files
    files = [e for e in entries if not e.endswith("/")]
    print(json.dumps(sorted(files)))


async def cmd_crash_pull(udid: str, dest_dir: str) -> None:
    os.makedirs(dest_dir, exist_ok=True)
    lockdown = await first_lockdown(udid)
    async with CrashReportsManager(lockdown) as crashes:
        await crashes.pull(out=dest_dir, entry="/", erase=False, progress_bar=False)
    print(json.dumps({"dest": dest_dir}))


async def cmd_install_app(udid: str, ipa_path: str) -> None:
    lockdown = await first_lockdown(udid)
    service = InstallationProxyService(lockdown)
    await service.install_from_local(Path(ipa_path))
    print(json.dumps({"ok": True, "installed": ipa_path}))


async def cmd_uninstall_app(udid: str, bundle_id: str) -> None:
    lockdown = await first_lockdown(udid)
    service = InstallationProxyService(lockdown)
    await service.uninstall(bundle_id)
    print(json.dumps({"ok": True, "uninstalled": bundle_id}))


async def cmd_backup(udid: str, dest_dir: str) -> None:
    os.makedirs(dest_dir, exist_ok=True)
    lockdown = await first_lockdown(udid)
    async with Mobilebackup2Service(lockdown) as backup:
        await backup.backup(full=True, backup_directory=dest_dir)
    print(json.dumps({"ok": True, "dest": dest_dir}))


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
    """Make sure the DeveloperDiskImage is mounted for this lockdown session.

    For iOS < 17, prefer a user-supplied DMG/signature at
    ~/linkdrop/ddi/ (override with LINKDROP_DDI_DIR) — that's the offline
    path. Otherwise, fall back to pymobiledevice3's `auto_mount`, which
    dispatches on iOS version: classic DDI for iOS 12-16 (downloaded from
    its own repo into ~/Xcode.app/...), Personalized DDI for iOS 17+
    (downloaded into ~/Xcode_iOS_DDI_Personalized/). Both paths cache after
    the first run so subsequent mounts are fast.
    """
    # Read iOS major version from lockdown's cached all_values.
    try:
        product_version = (lockdown.all_values or {}).get("ProductVersion", "")
        major = int(product_version.split(".")[0]) if product_version else 0
    except Exception:
        major = 0

    # Cheap short-circuit: if the classic Developer image is already on the
    # device (leftover from a previous mount), we're done for iOS < 17.
    if major and major < 17:
        if await DeveloperDiskImageMounter(lockdown).is_image_mounted(DDI_IMAGE_TYPE):
            return
        # Offline path: use ~/linkdrop/ddi/ if present.
        ddi_dir = _default_ddi_dir()
        dmg = ddi_dir / "DeveloperDiskImage.dmg"
        sig = ddi_dir / "DeveloperDiskImage.dmg.signature"
        if dmg.exists() and sig.exists():
            await DeveloperDiskImageMounter(lockdown).mount(dmg, sig)
            return

    # Online fallback — works for both iOS < 17 (classic) and iOS 17+
    # (personalized). First run downloads a ~20 MB classic image, or a
    # ~1.4 GB personalized image. After that, cached locally.
    from pymobiledevice3.exceptions import AlreadyMountedError

    try:
        await auto_mount(lockdown)
    except AlreadyMountedError:
        pass


async def cmd_mount_ddi(udid: str) -> None:
    lockdown = await first_lockdown(udid)
    await ensure_ddi_mounted(lockdown)
    product = (lockdown.all_values or {}).get("ProductVersion", "unknown")
    print(json.dumps({"ok": True, "ios": product}))


async def cmd_screenshot(udid: str, output_path: str) -> None:
    lockdown = await first_lockdown(udid)
    await ensure_ddi_mounted(lockdown)
    service = ScreenshotService(lockdown)
    data = await service.take_screenshot()
    with open(output_path, "wb") as f:
        f.write(data)
    print(json.dumps({"path": output_path, "bytes": len(data)}))


DAEMON_HANDLERS = {
    "list":         ("cmd_list", 0, False),
    "info":         ("cmd_info", 1, True),
    "wifi-enable":  ("cmd_wifi_enable", 1, True),
    "apps":         ("cmd_apps", 1, True),
    "mount-ddi":    ("cmd_mount_ddi", 1, True),
    "screenshot":   ("cmd_screenshot", 2, True),
    "list-photos":  ("cmd_list_photos", 2, True),  # udid, limit (int)
    "pull-photo":   ("cmd_pull_photo", 3, True),
    "list-app-files":("cmd_list_app_files", 3, True),
    "pull-app-file":("cmd_pull_app_file", 4, True),
    "push-app-file":("cmd_push_app_file", 4, True),
    "crash-list":   ("cmd_crash_list", 1, True),
    "crash-pull":   ("cmd_crash_pull", 2, True),
    "install-app":  ("cmd_install_app", 2, True),
    "uninstall-app":("cmd_uninstall_app", 2, True),
    "backup":       ("cmd_backup", 2, True),
    "sysdiagnose":  ("cmd_sysdiagnose", 2, True),
}


def _send(msg: dict) -> None:
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def run_daemon() -> int:
    """Long-running mode: one JSON-per-line request, one JSON response.

    Request:  {"id": <any>, "op": "info", "args": ["<udid>"]}
    Response: {"id": <same>, "ok": true, "data": <value>}
           or {"id": <same>, "ok": false, "error": "<msg>"}

    Python imports and lockdown sessions stay warm across requests, so
    per-call latency drops from the subprocess-cold-start ~1s to ~0.1s
    for cached flows.
    """
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    # Heartbeat-like banner so Rust knows the daemon is ready.
    _send({"ok": True, "event": "ready", "pid": os.getpid()})
    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            rid = None
            try:
                req = json.loads(line)
                rid = req.get("id")
                op = req["op"]
                args = req.get("args", [])
                entry = DAEMON_HANDLERS.get(op)
                if entry is None:
                    _send({"id": rid, "ok": False, "error": f"unknown op: {op}"})
                    continue
                fn_name, nargs, _needs_udid = entry
                if nargs and len(args) < nargs:
                    _send({
                        "id": rid,
                        "ok": False,
                        "error": f"{op} expects {nargs} arg(s), got {len(args)}",
                    })
                    continue
                # list-photos has an int limit
                if op == "list-photos" and len(args) >= 2:
                    try:
                        args = [args[0], int(args[1])]
                    except Exception:
                        args = [args[0], 200]
                fn = globals()[fn_name]
                buf = io.StringIO()
                with contextlib.redirect_stdout(buf):
                    loop.run_until_complete(fn(*args[:nargs]))
                raw = buf.getvalue().strip()
                data = json.loads(raw) if raw else None
                _send({"id": rid, "ok": True, "data": data})
            except SystemExit as e:
                _send({"id": rid, "ok": False, "error": str(e)})
            except Exception as e:
                _send({
                    "id": rid,
                    "ok": False,
                    "error": f"{type(e).__name__}: {e}",
                    "trace": traceback.format_exc(),
                })
    finally:
        loop.close()
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: pmd3_helper.py <daemon|list|info|wifi-enable|...> [args...]", file=sys.stderr)
        return 2
    op = argv[1]
    if op == "daemon":
        return run_daemon()
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
    if op == "crash-list":
        asyncio.run(cmd_crash_list(udid))
        return 0
    if op == "crash-pull":
        if len(argv) < 4:
            print("usage: pmd3_helper.py crash-pull <udid> <dest_dir>", file=sys.stderr)
            return 2
        asyncio.run(cmd_crash_pull(udid, argv[3]))
        return 0
    if op == "install-app":
        if len(argv) < 4:
            print("usage: pmd3_helper.py install-app <udid> <ipa_path>", file=sys.stderr)
            return 2
        asyncio.run(cmd_install_app(udid, argv[3]))
        return 0
    if op == "uninstall-app":
        if len(argv) < 4:
            print("usage: pmd3_helper.py uninstall-app <udid> <bundle_id>", file=sys.stderr)
            return 2
        asyncio.run(cmd_uninstall_app(udid, argv[3]))
        return 0
    if op == "backup":
        if len(argv) < 4:
            print("usage: pmd3_helper.py backup <udid> <dest_dir>", file=sys.stderr)
            return 2
        asyncio.run(cmd_backup(udid, argv[3]))
        return 0
    if op == "sysdiagnose":
        if len(argv) < 4:
            print("usage: pmd3_helper.py sysdiagnose <udid> <dest_dir>", file=sys.stderr)
            return 2
        asyncio.run(cmd_sysdiagnose(udid, argv[3]))
        return 0
    if op == "push-app-file":
        if len(argv) < 6:
            print("usage: pmd3_helper.py push-app-file <udid> <bundle_id> <local> <remote>", file=sys.stderr)
            return 2
        asyncio.run(cmd_push_app_file(udid, argv[3], argv[4], argv[5]))
        return 0
    handlers = {
        "info": cmd_info,
        "wifi-enable": cmd_wifi_enable,
        "apps": cmd_apps,
        "mount-ddi": cmd_mount_ddi,
    }
    if op not in handlers:
        print(f"unknown op: {op}", file=sys.stderr)
        return 2
    asyncio.run(handlers[op](udid))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
