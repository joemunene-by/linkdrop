<div align="center">

# linkdrop

**Connect your iPhone to Ubuntu. Photos, files, device info, and screen mirror — in one native app.**

Ubuntu's answer to Windows Phone Link, built for iPhone. Tauri + Rust on the backend, React on the front.
No Mac bridge. No jailbreak. No cloud.

![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=flat-square&logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)

</div>

---

## What it does

- **Device info** — name, model, iOS version, serial, battery, storage usage
- **Photos** — mount the iPhone's DCIM folder, browse and copy images/videos
- **Screenshots** — capture the iPhone screen, saved to `~/Pictures/linkdrop`
- **Screen mirror** — one-click AirPlay receiver so you can mirror your iPhone to your desktop

All via USB. iPhone just needs to be plugged in and trusted.

## What it deliberately doesn't do

Apple keeps a few things locked on non-Apple platforms. Rather than fake them, linkdrop is explicit about what's out of scope:

- **iMessage / SMS** — Apple only exposes iMessage through Handoff/Continuity (macOS only) or via a Mac relay (AirMessage). There's no clean Linux path.
- **Outbound calls** — same constraint.
- **Real-time notification push** — `idevicesyslog` can surface *some* events, but it's not reliable enough to ship as a feature.

If these are critical to you, you need a Mac in the loop. If they're not, linkdrop covers the rest nicely.

## Install the backing tools

### Runtime tools (required to use linkdrop)

```bash
sudo apt install libimobiledevice-utils ifuse usbmuxd uxplay
```

- `libimobiledevice-utils` — `ideviceinfo`, `idevice_id`, `idevicescreenshot`
- `ifuse` — FUSE mount of the iPhone's filesystem (for photo browsing)
- `usbmuxd` — daemon that handles USB ↔ iPhone communication
- `uxplay` — AirPlay 2 mirror receiver

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

- **v0.2** — Wi-Fi pairing (via `idevicepair`), drag-drop photo downloads, video thumbnail cache
- **v0.3** — Notification tail via `idevicesyslog` (best-effort, clearly labeled)
- **v0.4** — File manager tab: browse app sandboxes, copy files in/out, quick look
- **v0.5** — Flatpak + AppImage + Snap publishing

## Related projects

Built by [@joemunene-by](https://github.com/joemunene-by). Other recent work:

- [`secure-mcp`](https://github.com/joemunene-by/secure-mcp) — MCP server exposing security tools with policy gates
- [`cyberbench`](https://github.com/joemunene-by/cyberbench) — LLM cybersecurity reasoning benchmark
- [`GhostLM`](https://github.com/joemunene-by/GhostLM) — cybersecurity-focused LLM

## License

MIT. See [LICENSE](./LICENSE).

---

<sub>Not affiliated with Apple Inc. "iPhone" and "AirPlay" are trademarks of Apple Inc.</sub>
