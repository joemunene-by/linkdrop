<div align="center">

# linkdrop

**Connect your iPhone to Linux, macOS, or Windows. Photos, files, apps, backups, and screen mirror — in one native app.**

A cross-platform Phone-Link-for-iPhone, built around `pymobiledevice3`.
Tauri + Rust on the backend, React on the front. No Mac bridge required
on Linux/Windows. No jailbreak. No cloud.

![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=flat-square&logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)

</div>

---

## What it does

- **Device info** — name, model, iOS version, serial, battery, storage
- **Photos** — browse DCIM via AFC, per-photo download, bulk "Download all"
- **Apps** — installed-app list, file-sharing sandbox browser (Documents/), install `.ipa`, uninstall
- **Screenshots** — one click, DDI auto-mounts behind the scenes
- **Notifications tail** — iOS syslog filtered by regex, savable to file
- **Crash reports + Sysdiagnose** — list, pull, export for Apple Feedback
- **iOS backup** — full MobileBackup2 backup to a folder of your choice
- **Screen mirror** — AirPlay receiver (Linux: `uxplay`; macOS uses built-in)
- **Wi-Fi discovery** — device shows up in the picker once paired, cableless

Every op works on both USB and Wi-Fi. One codepath, all platforms.

## Supported iOS versions

| iOS range | Device info | Photos | Screenshot | Apps / browser | Wi-Fi sync | Mirror |
|---|---|---|---|---|---|---|
| **iOS 12 – 16** (tested: 15.8.7 / iPhone 6s) | ✅ | ✅ | ✅ classic DDI | ✅ | ✅ | ✅ |
| **iOS 17+** | ✅ | ✅ | ✅ Personalized DDI *(first mount auto-downloads ~1.4 GB)* | ✅ | ✅ | ✅ |
| iOS 10 – 11 | ⚠️ USB only (pymobiledevice3 ≥ 12) | ⚠️ USB only | ⚠️ | ⚠️ | ❌ | ✅ |
| iOS ≤ 9 | ❌ (libimobiledevice doesn't pair cleanly) |

Hardware: **iPhone 5s or newer** (A7+), all iPads. USB-C iPhones (15/16) work identically to Lightning.

Screenshot uses a Developer Disk Image that's specific to the iOS major version:
- **iOS < 17:** classic DMG + signature. linkdrop looks first in `~/linkdrop/ddi/` (drop your own files there for offline use); otherwise pymobiledevice3 fetches from its repo into `~/Xcode.app/…/DeviceSupport/<ver>/`.
- **iOS ≥ 17:** Personalized DDI. pymobiledevice3 fetches `Image.dmg` + `BuildManifest.plist` + trustcache once into `~/Xcode_iOS_DDI_Personalized/` and mounts from there.

Either way it's automatic — just click **Take screenshot** and it handles mounting if needed.

## Wi-Fi sync

Click **Enable Wi-Fi sync** in the Device panel while USB-connected (one-time setup — flips iPhone's `EnableWifiConnections` lockdown flag via the [idevice](https://crates.io/crates/idevice) crate). After that, linkdrop talks to the iPhone through `pymobiledevice3` whenever the device is on the same LAN: Device Info, Screenshot, Apps listing, and Photos browsing all work cableless. Developer Disk Image auto-mounts from `~/linkdrop/ddi/` (override with `LINKDROP_DDI_DIR`) whenever it's missing — iOS forgets the mount on reboot, so linkdrop quietly re-uploads it the first time you take a Wi-Fi screenshot after the iPhone restarts.

## What it deliberately doesn't do

Apple keeps a few things locked on non-Apple platforms. Rather than fake them, linkdrop is explicit about what's out of scope:

- **iMessage / SMS** — Apple only exposes iMessage through Handoff/Continuity (macOS only) or via a Mac relay (AirMessage). There's no clean Linux path.
- **Outbound calls** — same constraint.
- **Real-time notification push** — `idevicesyslog` can surface *some* events, but it's not reliable enough to ship as a feature.

If these are critical to you, you need a Mac in the loop. If they're not, linkdrop covers the rest nicely.

## Install the backing tools

### Runtime tools (all platforms)

linkdrop drives a small Python helper that calls `pymobiledevice3` for
everything iPhone-related. One toolchain, three OSes:

**Linux (Debian / Ubuntu)**

```bash
sudo apt install libimobiledevice-utils usbmuxd uxplay pipx
pipx install pymobiledevice3
```

**macOS**

```bash
brew install libimobiledevice ideviceinstaller pipx
pipx install pymobiledevice3
```

(macOS ships its own usbmuxd via Apple Mobile Device Service — nothing else
to start. AirPlay is handled natively by macOS itself, so the Screen Mirror
tab uses an AirPlay receiver only if you have one installed; otherwise the
system picker works fine.)

**Windows**

Install iTunes from Apple (ships Apple Mobile Device Service — linkdrop's
USB path talks to that). Then:

```powershell
winget install Python.Python.3.12
python -m pip install --user pipx
python -m pipx ensurepath
pipx install pymobiledevice3
```

### Optional: Linux-only Wi-Fi discovery via netmuxd

Linux users can drop in [netmuxd](https://github.com/jkcoxson/netmuxd) so
Wi-Fi-only iPhones show up in linkdrop's picker without pymobiledevice3's
per-call Bonjour browse. Not required — linkdrop's own listing already
browses via Bonjour on every platform.

```bash
curl -L -o /tmp/netmuxd https://github.com/jkcoxson/netmuxd/releases/latest/download/netmuxd-x86_64-linux-gnu
sudo install -m 0755 /tmp/netmuxd /usr/local/bin/netmuxd
```

`/etc/systemd/system/netmuxd.service`:

```ini
[Unit]
Description=netmuxd — Wi-Fi iPhone discovery
After=avahi-daemon.service

[Service]
ExecStart=/usr/local/bin/netmuxd --disable-unix --disable-heartbeat --host 127.0.0.1 -p 27015
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

…then `sudo systemctl enable --now netmuxd`.

### Build dependencies (required to compile from source)

Tauri needs GTK + WebKit + a C toolchain on Linux:

```bash
sudo apt install build-essential curl wget file libssl-dev \
  libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

After installing, plug your iPhone in and tap **Trust** when it prompts.

## Build and run

```bash
# 1. Clone and install frontend deps
git clone https://github.com/joemunene-by/linkdrop.git
cd linkdrop
bun install

# 2. Install Rust (one-time, if you don't have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 3. Dev mode (hot-reload frontend + rebuild Rust on save)
bun run tauri dev

# 4. Production bundle (.deb / .AppImage under src-tauri/target/release/bundle)
bun run tauri build
```

## Architecture

```
┌──────────────────────────────────┐
│  React UI (src/)                 │   tabs: Device / Photos / Mirror
│  - device-picker                 │
│  - status cards                  │
│  - photo grid                    │
└───────────────┬──────────────────┘
                │ Tauri IPC (invoke)
┌───────────────▼──────────────────┐
│  Rust backend (src-tauri/src/)   │
│  - device.rs  → ideviceinfo      │
│  - photos.rs  → ifuse mount      │
│  - screenshot.rs → idevicescreen…│
│  - airplay.rs → uxplay (spawn)   │
└───────────────┬──────────────────┘
                │ std::process::Command
┌───────────────▼──────────────────┐
│  libimobiledevice CLI + uxplay   │
└──────────────────────────────────┘
```

Every tool call is wrapped in a single `run()` helper that converts `NotFound` errors into a friendly "install this apt package" message — no cryptic crashes when something's missing.

## Roadmap

- ~~**v0.2** — Wi-Fi pairing + dual-transport device picker~~ ✅
- ~~**v0.3** — Notifications tab tailing `idevicesyslog`~~ ✅
- ~~**v0.4** — Apps tab + per-app sandbox browser via `house_arrest`~~ ✅
- ~~**v0.5** — AppImage + .deb via `bun run tauri build`~~ ✅
- ~~**v0.6** — Wi-Fi-complete ops, DDI auto-mount, photo bulk pull, crash logs, backups, app install/uninstall, macOS + Windows builds, GitHub Actions matrix, Settings + theme, first-run wizard~~ ✅
- **v0.7** — Flatpak / Snap / AUR, inline photo thumbnails, signed macOS + Windows artifacts

## Related projects

Built by [@joemunene-by](https://github.com/joemunene-by). Other recent work:

- [`secure-mcp`](https://github.com/joemunene-by/secure-mcp) — MCP server exposing security tools with policy gates
- [`cyberbench`](https://github.com/joemunene-by/cyberbench) — LLM cybersecurity reasoning benchmark
- [`GhostLM`](https://github.com/joemunene-by/GhostLM) — cybersecurity-focused LLM

## License

MIT. See [LICENSE](./LICENSE).

---

<sub>Not affiliated with Apple Inc. "iPhone" and "AirPlay" are trademarks of Apple Inc.</sub>
