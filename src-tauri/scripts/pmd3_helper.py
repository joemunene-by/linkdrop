#!/usr/bin/env python3
"""linkdrop helper: non-interactive pymobiledevice3 bridge for Wi-Fi ops.

The `pymobiledevice3` CLI's `--mobdev2` mode doesn't filter by UDID and
falls through to an interactive prompt when multiple IP addresses are
discovered (IPv4 + IPv6 link-local for the same device). This helper uses
the library API directly: browse mobdev2, pick the endpoint whose lockdown
handshake matches the requested UDID, then run the requested op.

Usage:
  pmd3_helper.py info <udid>                  # prints DeviceInfo-ish JSON on stdout
  pmd3_helper.py wifi-enable <udid>           # flips EnableWifiConnections
"""

import asyncio
import json
import sys
from typing import Optional

from pymobiledevice3.bonjour import browse_mobdev2
from pymobiledevice3.lockdown import create_using_tcp


async def connect_wifi(udid: str):
    """Find the mobdev2 endpoint matching `udid`, return a LockdownClient.

    Prefers IPv4 over IPv6 link-local and deduplicates identical IPs so we
    don't open 8 TLS sessions for a single device.
    """
    services = await browse_mobdev2(timeout=4.0)
    if not services:
        raise SystemExit("no mobdev2 devices on LAN")

    seen = set()
    hosts: list[str] = []
    for svc in services:
        for addr in svc.addresses:
            ip = addr.ip
            if ip in seen:
                continue
            seen.add(ip)
            hosts.append(ip)
    # Prefer IPv4 (shorter) first; IPv6 link-local last.
    hosts.sort(key=lambda h: (":" in h, "%" in h, h))

    tried = []
    for host in hosts:
        tried.append(host)
        try:
            client = await create_using_tcp(hostname=host, autopair=True)
        except Exception as e:
            print(f"  {host}: {type(e).__name__}: {e}", file=sys.stderr)
            continue
        print(f"  {host}: udid={getattr(client, 'udid', None)!r}", file=sys.stderr)
        if getattr(client, "udid", None) == udid:
            return client
        try:
            await client.close()
        except Exception:
            pass

    raise SystemExit(f"no matching device {udid} on LAN; tried {tried}")


async def cmd_info(udid: str) -> None:
    client = await connect_wifi(udid)
    info = await client.all_values()
    out = {
        "udid": client.identifier,
        "name": info.get("DeviceName", ""),
        "model": info.get("ProductName", ""),
        "product_type": info.get("ProductType", ""),
        "ios_version": info.get("ProductVersion", ""),
        "serial": info.get("SerialNumber", ""),
    }
    # battery + storage are on sub-domains
    try:
        battery = await client.get_value(
            domain="com.apple.mobile.battery", key="BatteryCurrentCapacity"
        )
        out["battery_percent"] = int(battery) if battery is not None else None
    except Exception:
        out["battery_percent"] = None
    try:
        du = await client.get_value(domain="com.apple.disk_usage")
        out["total_bytes"] = du.get("TotalDiskCapacity")
        out["free_bytes"] = du.get("AmountDataAvailable")
    except Exception:
        out["total_bytes"] = None
        out["free_bytes"] = None

    print(json.dumps(out))


async def cmd_wifi_enable(udid: str) -> None:
    client = await connect_wifi(udid)
    await client.set_value(
        domain="com.apple.mobile.wireless_lockdown",
        key="EnableWifiConnections",
        value=True,
    )
    print(json.dumps({"ok": True}))


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: pmd3_helper.py <info|wifi-enable> <udid>", file=sys.stderr)
        return 2
    op, udid = argv[1], argv[2]
    if op == "info":
        asyncio.run(cmd_info(udid))
    elif op == "wifi-enable":
        asyncio.run(cmd_wifi_enable(udid))
    else:
        print(f"unknown op: {op}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
