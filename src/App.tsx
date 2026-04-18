import { Fragment, useEffect, useRef, useState } from "react";
import { homeDir } from "@tauri-apps/api/path";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api } from "./ipc";
import type {
  AppEntry,
  AppFileEntry,
  DeviceInfo,
  DevicePlatform,
  DeviceSummary,
  PhotoEntry,
  AirPlayStatus,
  Transport,
} from "./types";

type Tab =
  | "device"
  | "photos"
  | "apps"
  | "mirror"
  | "notifications"
  | "diagnostics"
  | "settings";

const TABS: { key: Tab; label: string }[] = [
  { key: "device", label: "Device" },
  { key: "photos", label: "Photos" },
  { key: "apps", label: "Apps" },
  { key: "mirror", label: "Screen mirror" },
  { key: "notifications", label: "Notifications" },
  { key: "diagnostics", label: "Diagnostics" },
  { key: "settings", label: "Settings" },
];

type Theme = "system" | "light" | "dark";
const THEME_STORAGE_KEY = "linkdrop.theme";
const WIZARD_STORAGE_KEY = "linkdrop.setupSeen";

function detectOs(): "linux" | "macos" | "windows" {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) return "macos";
  if (ua.includes("win")) return "windows";
  return "linux";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

export default function App() {
  const [tab, setTab] = useState<Tab>("device");
  const [devices, setDevices] = useState<DeviceSummary[]>([]);
  const [selectedUdid, setSelectedUdid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = (localStorage.getItem(THEME_STORAGE_KEY) as Theme | null) ?? "system";
    return ["system", "light", "dark"].includes(saved) ? saved : "system";
  });

  useEffect(() => {
    applyTheme(theme);
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  const [showWizard, setShowWizard] = useState(
    () => localStorage.getItem(WIZARD_STORAGE_KEY) !== "1",
  );
  const dismissWizard = () => {
    localStorage.setItem(WIZARD_STORAGE_KEY, "1");
    setShowWizard(false);
  };

  const selectedDevice = devices.find((d) => d.udid === selectedUdid) ?? null;
  const selectedTransport: Transport | null = selectedDevice?.transport ?? null;
  const selectedPlatform: DevicePlatform | null = selectedDevice?.platform ?? null;

  const refreshDevices = async () => {
    setError(null);
    try {
      const list = await api.listDevices();
      setDevices(list);
      if (list.length > 0 && !selectedUdid) {
        setSelectedUdid(list[0].udid);
      } else if (list.length === 0) {
        setSelectedUdid(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    refreshDevices();
    const t = setInterval(refreshDevices, 5000);
    return () => clearInterval(t);

  }, []);

  // Fire-and-forget DDI prime for iPhones when selected. No-op on Android.
  useEffect(() => {
    if (!selectedUdid || !selectedTransport) return;
    if (selectedPlatform !== "ios") return;
    api.primeDdi(selectedUdid, selectedTransport).catch(() => {});
  }, [selectedUdid, selectedTransport, selectedPlatform]);

  return (
    <div className="app">
      {showWizard && <SetupWizard onDismiss={dismissWizard} />}
      <aside className="sidebar">
        <div className="brand">
          link<span className="dot">·</span>drop
        </div>
        {TABS.map((t) => (
          <button
            key={t.key}
            className={`nav-item ${tab === t.key ? "active" : ""}`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <div style={{ fontSize: 11, color: "var(--text-dim)", padding: "0 8px" }}>
          v0.1.0
        </div>
      </aside>

      <main className="content">
        <div className="device-picker">
          <label style={{ fontSize: 13, color: "var(--text-dim)" }}>Device</label>
          <select
            value={selectedUdid ?? ""}
            onChange={(e) => setSelectedUdid(e.target.value || null)}
          >
            {devices.length === 0 && <option value="">No iPhone detected</option>}
            {devices.map((d) => (
              <option key={d.udid} value={d.udid}>
                {d.udid.slice(0, 8)}…{d.udid.slice(-4)} —{" "}
                {d.platform === "android" ? "Android" : "iPhone"} /{" "}
                {d.transport === "usb" ? "USB" : "Wi-Fi"}
              </option>
            ))}
          </select>
          <button className="btn secondary" onClick={refreshDevices}>
            Refresh
          </button>
        </div>

        {error && <div className="error">{error}</div>}

        {tab === "device" && (
          <DevicePanel
            udid={selectedUdid}
            transport={selectedTransport}
            platform={selectedPlatform}
          />
        )}
        {tab === "photos" && (
          <PhotosPanel
            udid={selectedUdid}
            transport={selectedTransport}
            platform={selectedPlatform}
          />
        )}
        {tab === "apps" && (
          <AppsPanel
            udid={selectedUdid}
            transport={selectedTransport}
            platform={selectedPlatform}
          />
        )}
        {tab === "mirror" && (
          <MirrorPanel udid={selectedUdid} platform={selectedPlatform} />
        )}
        {tab === "notifications" && (
          <NotificationsPanel
            udid={selectedUdid}
            transport={selectedTransport}
            platform={selectedPlatform}
          />
        )}
        {tab === "diagnostics" && (
          <DiagnosticsPanel
            udid={selectedUdid}
            transport={selectedTransport}
            platform={selectedPlatform}
          />
        )}
        {tab === "settings" && <SettingsPanel theme={theme} setTheme={setTheme} />}
      </main>
    </div>
  );
}

function formatBytes(n: number | null): string {
  if (n === null) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = n;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(1)} ${units[i]}`;
}

function DevicePanel({
  udid,
  transport,
  platform,
}: {
  udid: string | null;
  transport: Transport | null;
  platform: DevicePlatform | null;
}) {
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!udid || !transport || !platform) {
      setInfo(null);
      return;
    }
    setLoading(true);
    setError(null);
    api
      .getDeviceInfo(udid, transport, platform)
      .then(setInfo)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [udid, transport, platform]);

  if (!udid || !transport || !platform) {
    return (
      <>
        <h1>Device</h1>
        <p className="sub">Plug in your iPhone via USB and tap "Trust" on the device.</p>
        <div className="card">
          <div className="empty">No device selected.</div>
        </div>
      </>
    );
  }

  const usedBytes =
    info?.total_bytes && info?.free_bytes ? info.total_bytes - info.free_bytes : null;
  const usedPct =
    info?.total_bytes && usedBytes !== null
      ? Math.round((usedBytes / info.total_bytes) * 100)
      : null;

  const takeShot = async () => {
    setError(null);
    try {
      const outDir = `${await homeDir()}/Pictures/linkdrop`;
      const r = await api.takeScreenshot(udid, transport, platform, outDir);
      alert(`Saved: ${r.path}`);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1>
        Device{" "}
        <span className={`pill ok`}>
          {platform === "android" ? "Android" : "iPhone"}
        </span>{" "}
        <span className={`pill ${transport === "usb" ? "ok" : ""}`}>
          {transport === "usb" ? "USB" : "Wi-Fi"}
        </span>
      </h1>
      <p className="sub">
        {info?.name ? `${info.name} — ${info.model}` : "Loading device info…"}
      </p>
      {platform === "ios" && transport === "wifi" && (
        <div
          style={{
            padding: "10px 14px",
            marginBottom: 12,
            background: "rgba(138, 43, 226, 0.08)",
            border: "1px solid rgba(138, 43, 226, 0.35)",
            borderRadius: 6,
            color: "var(--text-dim)",
            fontSize: 12,
          }}
        >
          <strong style={{ color: "var(--text)" }}>Wi-Fi mode:</strong>{" "}
          Device info, screenshot, photos, and apps all go through{" "}
          <code>pymobiledevice3</code>. No cable needed.
        </div>
      )}
      {platform === "android" && (
        <div
          style={{
            padding: "10px 14px",
            marginBottom: 12,
            background: "rgba(56, 142, 60, 0.08)",
            border: "1px solid rgba(56, 142, 60, 0.35)",
            borderRadius: 6,
            color: "var(--text-dim)",
            fontSize: 12,
          }}
        >
          <strong style={{ color: "var(--text)" }}>Android:</strong> routed
          through <code>adb</code>. Make sure USB debugging is on and you've
          accepted the RSA fingerprint prompt on the phone.
        </div>
      )}

      {error && <div className="error">{error}</div>}

      <div className="card">
        <h2>Identity</h2>
        {loading && !info ? (
          <div className="empty">Fetching…</div>
        ) : info ? (
          <div className="info-grid">
            <div>
              <div className="label">Name</div>
              <div className="value">{info.name || "—"}</div>
            </div>
            <div>
              <div className="label">Model</div>
              <div className="value">
                {info.model} <span className="pill">{info.product_type}</span>
              </div>
            </div>
            <div>
              <div className="label">
                {platform === "android" ? "Android" : "iOS"}
              </div>
              <div className="value">{info.ios_version || "—"}</div>
            </div>
            <div>
              <div className="label">Serial</div>
              <div className="value">{info.serial || "—"}</div>
            </div>
            <div style={{ gridColumn: "1 / -1" }}>
              <div className="label">UDID</div>
              <div className="value">{info.udid}</div>
            </div>
          </div>
        ) : null}
      </div>

      {info && (
        <div className="card">
          <h2>Status</h2>
          <div className="info-grid">
            <div>
              <div className="label">Battery</div>
              <div className="value">
                {info.battery_percent !== null ? (
                  <>
                    {info.battery_percent}%{" "}
                    <span
                      className={`pill ${
                        info.battery_percent < 20 ? "warn" : "ok"
                      }`}
                    >
                      {info.battery_percent < 20 ? "low" : "ok"}
                    </span>
                  </>
                ) : (
                  "—"
                )}
              </div>
            </div>
            <div>
              <div className="label">Storage</div>
              <div className="value">
                {formatBytes(usedBytes)} / {formatBytes(info.total_bytes)}
                {usedPct !== null && (
                  <div className="progress">
                    <div className="fill" style={{ width: `${usedPct}%` }} />
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      <div className="card">
        <h2>Actions</h2>
        <div className="row">
          <button className="btn" onClick={takeShot}>
            Take screenshot
          </button>
          <span style={{ fontSize: 12, color: "var(--text-dim)" }}>
            Saves to ~/Pictures/linkdrop/
          </span>
        </div>
        {transport === "usb" && (
          <div className="row" style={{ marginTop: 12 }}>
            <button
              className="btn secondary"
              onClick={async () => {
                setError(null);
                try {
                  await api.enableWifiSync(udid);
                  alert(
                    "Wi-Fi sync enabled. Unplug the iPhone and it should appear under Wi-Fi within ~30s."
                  );
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              Enable Wi-Fi sync
            </button>
            <span style={{ fontSize: 12, color: "var(--text-dim)" }}>
              One-time. Keeps the device reachable without a cable.
            </span>
          </div>
        )}
      </div>
    </>
  );
}

function PhotosPanel({
  udid,
  transport,
  platform,
}: {
  udid: string | null;
  transport: Transport | null;
  platform: DevicePlatform | null;
}) {
  const [photos, setPhotos] = useState<PhotoEntry[]>([]);
  const [mounted, setMounted] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mount = async () => {
    if (!udid || !transport || !platform) return;
    setLoading(true);
    setError(null);
    try {
      const items = await api.listPhotos(udid, transport, platform, 200);
      setPhotos(items);
      setMounted(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const unmount = async () => {
    setMounted(false);
    setPhotos([]);
  };

  useEffect(() => {
    return () => {};
  }, [mounted, transport]);

  return (
    <>
      <h1>Photos</h1>
      <p className="sub">Browse DCIM from your iPhone's Photos roll via ifuse.</p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <div className="row">
          <button
            className="btn"
            onClick={mount}
            disabled={!udid || !transport || !platform || mounted || loading}
          >
            {mounted ? "Loaded" : "Load photos"}
          </button>
          <button
            className="btn secondary"
            onClick={unmount}
            disabled={!mounted}
          >
            Clear
          </button>
          {loading && <span style={{ color: "var(--text-dim)" }}>Reading…</span>}
          {mounted && (
            <span className="pill ok">{photos.length} item(s)</span>
          )}
        </div>
      </div>

      {mounted && (
        <div className="card">
          <div className="row" style={{ marginBottom: 12 }}>
            <h2 style={{ margin: 0 }}>DCIM</h2>
            <div style={{ flex: 1 }} />
            {udid && transport && platform && photos.length > 0 && (
              <PhotoBulkDownload
                udid={udid}
                transport={transport}
                platform={platform}
                photos={photos}
              />
            )}
          </div>
          {photos.length === 0 ? (
            <div className="empty">No photos found.</div>
          ) : (
            <div className="photo-grid">
              {photos.map((p) => (
                <div key={p.path} className="photo-tile" title={p.path}>
                  <div className="name">{p.name}</div>
                  <div className="kind">{p.kind}</div>
                  <div>{formatBytes(p.size_bytes)}</div>
                  {udid && transport && platform && (
                    <button
                      className="btn secondary"
                      style={{
                        marginTop: 6,
                        padding: "2px 8px",
                        fontSize: 11,
                        width: "100%",
                      }}
                      onClick={async () => {
                        try {
                          const dest = `${await homeDir()}/Pictures/linkdrop/${p.name}`;
                          await api.pullPhoto(
                            udid,
                            transport,
                            platform,
                            p.path,
                            dest,
                          );
                          alert(`Saved: ${dest}`);
                        } catch (e) {
                          setError(String(e));
                        }
                      }}
                    >
                      Download
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}

function PhotoBulkDownload({
  udid,
  transport,
  platform,
  photos,
}: {
  udid: string;
  transport: Transport;
  platform: DevicePlatform;
  photos: PhotoEntry[];
}) {
  const [progress, setProgress] = useState<{
    running: boolean;
    done: number;
    total: number;
    errors: number;
  }>({ running: false, done: 0, total: 0, errors: 0 });

  const run = async () => {
    if (progress.running) return;
    setProgress({ running: true, done: 0, total: photos.length, errors: 0 });
    let done = 0;
    let errors = 0;
    const base = `${await homeDir()}/Pictures/linkdrop`;
    for (const p of photos) {
      try {
        await api.pullPhoto(udid, transport, platform, p.path, `${base}/${p.name}`);
      } catch {
        errors++;
      }
      done++;
      setProgress({ running: true, done, total: photos.length, errors });
    }
    setProgress({ running: false, done, total: photos.length, errors });
    alert(
      `Downloaded ${done - errors}/${photos.length} → ~/Pictures/linkdrop/${errors ? ` (${errors} failed)` : ""}`,
    );
  };

  return (
    <div className="row" style={{ gap: 8 }}>
      {progress.running && (
        <span style={{ fontSize: 12, color: "var(--text-dim)" }}>
          {progress.done}/{progress.total}
        </span>
      )}
      <button
        className="btn"
        onClick={run}
        disabled={progress.running || photos.length === 0}
      >
        {progress.running ? "Downloading…" : `Download all (${photos.length})`}
      </button>
    </div>
  );
}

function MirrorPanel({
  udid,
  platform,
}: {
  udid: string | null;
  platform: DevicePlatform | null;
}) {
  const [status, setStatus] = useState<AirPlayStatus>("Stopped");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .airplayStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));
    const t = setInterval(() => {
      api
        .airplayStatus()
        .then(setStatus)
        .catch(() => {});
    }, 2000);
    return () => clearInterval(t);
  }, []);

  const start = async () => {
    setError(null);
    try {
      setStatus(
        await api.startAirplay(
          udid ?? undefined,
          platform ?? undefined,
          "linkdrop",
        ),
      );
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    setError(null);
    try {
      setStatus(await api.stopAirplay());
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1>Screen mirror</h1>
      <p className="sub">
        {platform === "android"
          ? "Starts scrcpy against the selected Android device. Needs `scrcpy` on PATH and USB debugging on the phone."
          : "Starts an AirPlay receiver (uxplay). Swipe down from the top-right on your iPhone and choose \"linkdrop\" under Screen Mirroring."}
      </p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <h2>Receiver</h2>
        <div className="row">
          <span className={`pill ${status === "Running" ? "ok" : ""}`}>
            {status.toLowerCase()}
          </span>
          <button className="btn" onClick={start} disabled={status === "Running"}>
            Start
          </button>
          <button
            className="btn danger"
            onClick={stop}
            disabled={status !== "Running"}
          >
            Stop
          </button>
        </div>
        <p style={{ fontSize: 12, color: "var(--text-dim)", marginTop: 12 }}>
          {platform === "android" ? (
            <>
              scrcpy opens its own window. Install:{" "}
              <code>sudo apt install scrcpy</code> /{" "}
              <code>brew install scrcpy</code> /{" "}
              <code>winget install Genymobile.scrcpy</code>.
            </>
          ) : (
            <>
              uxplay opens its own window when the iPhone begins mirroring.
              Install: <code>sudo apt install uxplay</code> /{" "}
              <code>brew install uxplay</code>.
            </>
          )}
        </p>
      </div>
    </>
  );
}

function AppsPanel({
  udid,
  transport,
  platform,
}: {
  udid: string | null;
  transport: Transport | null;
  platform: DevicePlatform | null;
}) {
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [browsing, setBrowsing] = useState<AppEntry | null>(null);
  const [ipaPath, setIpaPath] = useState("");
  const [installing, setInstalling] = useState(false);

  const install = async () => {
    if (!udid || !transport || !platform || !ipaPath.trim()) return;
    setInstalling(true);
    setError(null);
    try {
      await api.installApp(udid, transport, platform, ipaPath.trim());
      alert(`Installed: ${ipaPath}`);
      setIpaPath("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  };

  const uninstall = async (app: AppEntry) => {
    if (!udid || !transport || !platform) return;
    if (!confirm(`Uninstall ${app.name}?`)) return;
    setError(null);
    try {
      await api.uninstallApp(udid, transport, platform, app.bundle_id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const load = async () => {
    if (!udid || !transport || !platform) return;
    setLoading(true);
    setError(null);
    try {
      const list = await api.listApps(udid, transport, platform);
      setApps(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const q = query.trim().toLowerCase();
  const filtered = q
    ? apps.filter(
        (a) =>
          a.name.toLowerCase().includes(q) ||
          a.bundle_id.toLowerCase().includes(q),
      )
    : apps;

  if (browsing && udid && transport && platform) {
    return (
      <AppBrowser
        udid={udid}
        transport={transport}
        platform={platform}
        app={browsing}
        onBack={() => setBrowsing(null)}
      />
    );
  }

  return (
    <>
      <h1>Apps</h1>
      <p className="sub">
        User-installed apps on the iPhone, via{" "}
        <code>installation_proxy</code>. Apps with File Sharing enabled show a
        Browse button.
      </p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <div className="row">
          <button
            className="btn"
            onClick={load}
            disabled={!udid || !transport || loading}
          >
            {loading ? "Loading…" : apps.length > 0 ? "Reload" : "Load"}
          </button>
          <input
            style={{
              flex: 1,
              padding: "6px 10px",
              background: "var(--bg-deep)",
              color: "var(--text)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              fontSize: 13,
            }}
            placeholder="Filter by name or bundle id"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            disabled={apps.length === 0}
          />
          {apps.length > 0 && (
            <span className="pill ok">
              {filtered.length} / {apps.length}
            </span>
          )}
        </div>
        <div className="row" style={{ marginTop: 10 }}>
          <button
            className="btn secondary"
            onClick={async () => {
              const picked = await openDialog({
                multiple: false,
                directory: false,
                filters: [{ name: "iOS package", extensions: ["ipa"] }],
              });
              if (typeof picked === "string") setIpaPath(picked);
            }}
          >
            Pick .ipa…
          </button>
          <div
            style={{
              flex: 1,
              padding: "6px 10px",
              background: "var(--bg-deep)",
              color: ipaPath ? "var(--text)" : "var(--text-dim)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              fontSize: 12,
              fontFamily: "monospace",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {ipaPath || "(no file selected)"}
          </div>
          <button
            className="btn"
            onClick={install}
            disabled={!udid || !transport || !ipaPath.trim() || installing}
          >
            {installing ? "Installing…" : "Install"}
          </button>
        </div>
      </div>

      {apps.length > 0 && (
        <div className="card">
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr auto auto",
              gap: "6px 16px",
              fontSize: 13,
              alignItems: "center",
            }}
          >
            {filtered.map((a) => (
              <Fragment key={a.bundle_id}>
                <div>
                  <div style={{ color: "var(--text)" }}>{a.name}</div>
                  <div
                    style={{
                      fontSize: 11,
                      color: "var(--text-dim)",
                      fontFamily: "monospace",
                    }}
                  >
                    {a.bundle_id}
                  </div>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {a.version}
                </div>
                <div className="row" style={{ gap: 6 }}>
                  {a.has_file_sharing ? (
                    <button
                      className="btn secondary"
                      style={{ padding: "2px 10px", fontSize: 12 }}
                      onClick={() => setBrowsing(a)}
                    >
                      Browse
                    </button>
                  ) : null}
                  <button
                    className="btn danger"
                    style={{ padding: "2px 10px", fontSize: 12 }}
                    onClick={() => uninstall(a)}
                  >
                    Uninstall
                  </button>
                </div>
              </Fragment>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

function AppBrowser({
  udid,
  transport,
  platform,
  app,
  onBack,
}: {
  udid: string;
  transport: Transport;
  platform: DevicePlatform;
  app: AppEntry;
  onBack: () => void;
}) {
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<AppFileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    api
      .listAppFiles(udid, transport, platform, app.bundle_id, path)
      .then((list) => {
        if (!cancelled) setEntries(list);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [udid, transport, platform, app.bundle_id, path]);

  const openDir = (p: string) => setPath(p);
  const up = () => {
    if (path === "/") return;
    const parts = path.split("/").filter(Boolean);
    parts.pop();
    setPath(parts.length ? "/" + parts.join("/") : "/");
  };

  const download = async (entry: AppFileEntry) => {
    try {
      const home = await homeDir();
      const local = `${home}/Downloads/linkdrop-${app.name}-${entry.name}`;
      await api.pullAppFile(
        udid,
        transport,
        platform,
        app.bundle_id,
        entry.path,
        local,
      );
      alert(`Saved: ${local}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const upload = async () => {
    try {
      const picked = await openDialog({ multiple: false, directory: false });
      if (typeof picked !== "string") return;
      const name = picked.split(/[\\/]/).pop() ?? "upload";
      const remote = path.endsWith("/") ? `${path}${name}` : `${path}/${name}`;
      await api.pushAppFile(
        udid,
        transport,
        platform,
        app.bundle_id,
        picked,
        remote,
      );
      // refresh listing
      const list = await api.listAppFiles(
        udid,
        transport,
        platform,
        app.bundle_id,
        path,
      );
      setEntries(list);
      alert(`Uploaded → ${remote}`);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1>{app.name}</h1>
      <p className="sub" style={{ fontFamily: "monospace", fontSize: 12 }}>
        {app.bundle_id} — Documents{path}
      </p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <div className="row">
          <button className="btn secondary" onClick={onBack}>
            ← Apps
          </button>
          <button
            className="btn secondary"
            onClick={up}
            disabled={path === "/"}
          >
            ↑ Up
          </button>
          <span style={{ fontFamily: "monospace", fontSize: 12, color: "var(--text-dim)" }}>
            {path}
          </span>
          <div style={{ flex: 1 }} />
          <button className="btn secondary" onClick={upload}>
            Upload…
          </button>
          {loading && <span style={{ color: "var(--text-dim)" }}>Loading…</span>}
        </div>
      </div>

      <div className="card">
        {entries.length === 0 && !loading ? (
          <div className="empty">Empty directory.</div>
        ) : (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr auto auto",
              gap: "6px 16px",
              fontSize: 13,
              alignItems: "center",
            }}
          >
            {entries.map((e) => (
              <Fragment key={e.path}>
                <div
                  style={{
                    color: "var(--text)",
                    cursor: e.is_dir ? "pointer" : "default",
                  }}
                  onClick={() => e.is_dir && openDir(e.path)}
                >
                  {e.is_dir ? "📁" : "📄"} {e.name}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {e.is_dir ? "" : formatBytes(e.size_bytes)}
                </div>
                {e.is_dir ? (
                  <span style={{ fontSize: 11, color: "var(--text-dim)" }}>
                    —
                  </span>
                ) : (
                  <button
                    className="btn secondary"
                    style={{ padding: "2px 10px", fontSize: 12 }}
                    onClick={() => download(e)}
                  >
                    Download
                  </button>
                )}
              </Fragment>
            ))}
          </div>
        )}
      </div>
    </>
  );
}

function SetupWizard({ onDismiss }: { onDismiss: () => void }) {
  const os = detectOs();
  const installCmd =
    os === "macos"
      ? "brew install libimobiledevice pipx && pipx install pymobiledevice3"
      : os === "windows"
        ? "winget install libimobiledevice && pipx install pymobiledevice3"
        : "sudo apt install libimobiledevice-utils usbmuxd uxplay pipx && pipx install pymobiledevice3";

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0, 0, 0, 0.55)",
        display: "grid",
        placeItems: "center",
        zIndex: 100,
      }}
    >
      <div
        style={{
          width: "min(560px, 92vw)",
          maxHeight: "92vh",
          overflowY: "auto",
          background: "var(--bg-elev)",
          border: "1px solid var(--border)",
          borderRadius: 10,
          padding: 24,
        }}
      >
        <h1 style={{ marginTop: 0 }}>Welcome to linkdrop</h1>
        <p className="sub">
          A tiny setup before your iPhone shows up. linkdrop detected you're on{" "}
          <strong>{os}</strong>.
        </p>

        <ol style={{ paddingLeft: 18, lineHeight: 1.8 }}>
          <li>
            <strong>Install the runtime bits:</strong>
            <pre
              style={{
                background: "var(--bg-deep)",
                padding: "10px 12px",
                borderRadius: 6,
                fontSize: 12,
                overflowX: "auto",
                marginTop: 6,
              }}
            >
              {installCmd}
            </pre>
          </li>
          <li>
            <strong>Plug your iPhone in</strong> and tap{" "}
            <em>Trust this computer</em> on the phone.
          </li>
          <li>
            <strong>Pair it</strong> with pymobiledevice3 (creates a pair
            record on disk):
            <pre
              style={{
                background: "var(--bg-deep)",
                padding: "10px 12px",
                borderRadius: 6,
                fontSize: 12,
                marginTop: 6,
              }}
            >
              pymobiledevice3 lockdown pair
            </pre>
          </li>
          <li>
            In the Device tab, click <strong>Enable Wi-Fi sync</strong> so you
            can use linkdrop cablelessly afterwards.
          </li>
          <li>
            (iOS &lt; 17 only) drop a matching DeveloperDiskImage into{" "}
            <code>~/linkdrop/ddi/</code>, or let linkdrop auto-download when
            you take a screenshot.
          </li>
        </ol>

        <div className="row" style={{ justifyContent: "flex-end", marginTop: 18 }}>
          <button className="btn secondary" onClick={onDismiss}>
            Don't show again
          </button>
          <button className="btn" onClick={onDismiss}>
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}

function SettingsPanel({
  theme,
  setTheme,
}: {
  theme: Theme;
  setTheme: (t: Theme) => void;
}) {
  return (
    <>
      <h1>Settings</h1>
      <p className="sub">Preferences are stored in browser localStorage.</p>

      <div className="card">
        <h2>Appearance</h2>
        <div className="row" style={{ gap: 10 }}>
          {(["system", "light", "dark"] as const).map((t) => (
            <button
              key={t}
              className={`btn ${theme === t ? "" : "secondary"}`}
              onClick={() => setTheme(t)}
            >
              {t[0].toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Paths</h2>
        <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
          <p>
            Developer Disk Images: <code>~/linkdrop/ddi/</code> (override
            with <code>LINKDROP_DDI_DIR</code>).
          </p>
          <p>
            Cross-platform tip: linkdrop auto-detects the pipx-installed{" "}
            <code>pymobiledevice3</code> at <code>~/.local/share/pipx</code>{" "}
            (Linux), <code>~/.local/pipx</code> (macOS), or{" "}
            <code>%USERPROFILE%/pipx</code> (Windows). Install with{" "}
            <code>pipx install pymobiledevice3</code>.
          </p>
        </div>
      </div>

      <div className="card">
        <h2>About</h2>
        <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
          linkdrop v0.7.0 — built with Tauri, React, and pymobiledevice3.
        </div>
      </div>
    </>
  );
}

function DiagnosticsPanel({
  udid,
  transport,
  platform,
}: {
  udid: string | null;
  transport: Transport | null;
  platform: DevicePlatform | null;
}) {
  const [reports, setReports] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  const load = async () => {
    if (!udid || !transport) return;
    setLoading(true);
    setError(null);
    try {
      const list = await api.listCrashReports(udid, transport);
      setReports(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const pullAll = async () => {
    if (!udid || !transport) return;
    setPulling(true);
    setError(null);
    try {
      const dest = `${await homeDir()}/Downloads/linkdrop-crashes-${Date.now()}`;
      await api.pullCrashReports(udid, transport, dest);
      alert(`Saved to: ${dest}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setPulling(false);
    }
  };

  const q = query.trim().toLowerCase();
  const filtered = q
    ? reports.filter((r) => r.toLowerCase().includes(q))
    : reports;

  return (
    <>
      <h1>Diagnostics</h1>
      <p className="sub">
        Crash reports and analytics (<code>.ips</code>) copied from the device's{" "}
        <code>CrashReports</code> mobile directory.
      </p>

      {error && <div className="error">{error}</div>}

      {platform === "ios" && (
      <div className="card">
        <div className="row">
          <button
            className="btn"
            onClick={load}
            disabled={!udid || !transport || loading}
          >
            {loading ? "Loading…" : reports.length > 0 ? "Reload" : "List crashes"}
          </button>
          <button
            className="btn"
            onClick={pullAll}
            disabled={!udid || !transport || pulling || reports.length === 0}
          >
            {pulling ? "Pulling…" : `Pull all (${reports.length})`}
          </button>
          <input
            style={{
              flex: 1,
              padding: "6px 10px",
              background: "var(--bg-deep)",
              color: "var(--text)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              fontSize: 13,
            }}
            placeholder="Filter by name (e.g. Spotify, Panics)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            disabled={reports.length === 0}
          />
          {reports.length > 0 && (
            <span className="pill ok">
              {filtered.length} / {reports.length}
            </span>
          )}
        </div>
      </div>
      )}

      {platform === "ios" && reports.length > 0 && (
        <div className="card">
          <div
            style={{
              fontFamily: "monospace",
              fontSize: 11,
              maxHeight: 420,
              overflowY: "auto",
              color: "var(--text-dim)",
            }}
          >
            {filtered.map((r) => (
              <div key={r} style={{ padding: "2px 0" }}>
                {r}
              </div>
            ))}
          </div>
        </div>
      )}

      {platform === "ios" && (
        <>
          <BackupCard udid={udid} transport={transport} />
          <SysdiagnoseCard udid={udid} transport={transport} />
        </>
      )}
      {platform === "android" && (
        <div className="card">
          <h2>Android diagnostics</h2>
          <p
            className="sub"
            style={{ marginTop: -4, fontSize: 12 }}
          >
            Backups and sysdiagnose are iOS-only services. For an Android
            bug-report bundle, run{" "}
            <code>adb -s {udid ?? "SERIAL"} bugreport bugreport.zip</code> in
            a terminal.
          </p>
        </div>
      )}
    </>
  );
}

function SysdiagnoseCard({
  udid,
  transport,
}: {
  udid: string | null;
  transport: Transport | null;
}) {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (!udid || !transport) return;
    let dest: string;
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: `${await homeDir()}/Downloads`,
      });
      if (typeof picked !== "string") return;
      dest = picked;
    } catch (e) {
      setError(String(e));
      return;
    }
    setRunning(true);
    setError(null);
    try {
      await api.pullSysdiagnose(udid, transport, dest);
      alert(`Sysdiagnose pulled → ${dest}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="card">
      <h2>Sysdiagnose</h2>
      <p className="sub" style={{ marginTop: -4, marginBottom: 10, fontSize: 12 }}>
        Triggers iOS to generate a fresh sysdiagnose bundle and pulls it. Larger
        than crash reports; useful for reporting iOS issues to Apple or
        debugging kernel panics.
      </p>
      {error && <div className="error">{error}</div>}
      <div className="row">
        <button
          className="btn"
          onClick={run}
          disabled={!udid || !transport || running}
        >
          {running ? "Generating + pulling… (can take minutes)" : "Pull sysdiagnose"}
        </button>
      </div>
    </div>
  );
}

function BackupCard({
  udid,
  transport,
}: {
  udid: string | null;
  transport: Transport | null;
}) {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const backup = async () => {
    if (!udid || !transport) return;
    let baseDir: string;
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: `${await homeDir()}/Backups`,
      });
      if (typeof picked !== "string") return; // cancelled
      baseDir = picked;
    } catch (e) {
      setError(String(e));
      return;
    }
    setRunning(true);
    setError(null);
    try {
      const dest = `${baseDir}/linkdrop-${udid.slice(0, 8)}-${Date.now()}`;
      await api.createBackup(udid, transport, dest);
      alert(`Backup saved: ${dest}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="card">
      <h2>Backup</h2>
      <p
        className="sub"
        style={{ marginTop: -4, marginBottom: 10, fontSize: 12 }}
      >
        Full MobileBackup2 backup. Saves into <code>~/Backups/</code>. Can take
        a while + uses lots of disk.
      </p>
      {error && <div className="error">{error}</div>}
      <div className="row">
        <button
          className="btn"
          onClick={backup}
          disabled={!udid || !transport || running}
        >
          {running ? "Backing up… (don't unplug)" : "Create backup"}
        </button>
      </div>
    </div>
  );
}

function NotificationsPanel({
  udid,
  transport,
  platform,
}: {
  udid: string | null;
  transport: Transport | null;
  platform: DevicePlatform | null;
}) {
  const [running, setRunning] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const [filter, setFilter] = useState(
    "SpringBoard|UNUserNotification|BulletinBoard|UsageTrackingAgent"
  );
  const [error, setError] = useState<string | null>(null);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unsub = listen<string>("syslog", (event) => {
      const line = event.payload;
      let re: RegExp | null = null;
      try {
        re = new RegExp(filter, "i");
      } catch {
        re = null;
      }
      if (re === null || re.test(line)) {
        setLines((prev) => [...prev.slice(-500), line]);
      }
    });
    return () => {
      unsub.then((u) => u()).catch(() => {});
    };
  }, [filter]);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [lines]);

  const start = async () => {
    if (!udid || !transport || !platform) return;
    setError(null);
    try {
      await api.startNotifications(udid, transport, platform);
      setRunning(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    try {
      await api.stopNotifications();
    } catch (e) {
      setError(String(e));
    }
    setRunning(false);
  };

  useEffect(() => {
    return () => {
      api.stopNotifications().catch(() => {});
    };
  }, []);

  return (
    <>
      <h1>Notifications</h1>
      <p className="sub">
        Tails iOS syslog via <code>idevicesyslog</code> and shows matching lines.
        Best-effort — iOS doesn't expose a clean notification stream to
        non-Apple platforms.
      </p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <div className="row">
          <button
            className="btn"
            onClick={start}
            disabled={!udid || !transport || !platform || running}
          >
            {running ? "Streaming" : "Start"}
          </button>
          <button className="btn secondary" onClick={stop} disabled={!running}>
            Stop
          </button>
          <input
            style={{
              flex: 1,
              padding: "6px 10px",
              background: "var(--bg-deep)",
              color: "var(--text)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              fontFamily: "monospace",
              fontSize: 12,
            }}
            placeholder="filter regex"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <button
            className="btn secondary"
            onClick={() => setLines([])}
            disabled={lines.length === 0}
          >
            Clear
          </button>
          <button
            className="btn secondary"
            onClick={async () => {
              if (lines.length === 0) return;
              try {
                const picked = await saveDialog({
                  defaultPath: `syslog-${Date.now()}.txt`,
                  filters: [{ name: "Text", extensions: ["txt", "log"] }],
                });
                if (typeof picked !== "string") return;
                await api.saveSyslogToFile(picked, lines.join("\n") + "\n");
                alert(`Saved ${lines.length} lines → ${picked}`);
              } catch (e) {
                setError(String(e));
              }
            }}
            disabled={lines.length === 0}
          >
            Save…
          </button>
        </div>
      </div>

      <div className="card">
        <div
          ref={logRef}
          style={{
            fontFamily: "monospace",
            fontSize: 11,
            maxHeight: 420,
            overflowY: "auto",
            whiteSpace: "pre-wrap",
            color: "var(--text-dim)",
          }}
        >
          {lines.length === 0 ? (
            <div className="empty">No events yet.</div>
          ) : (
            lines.map((l, i) => <div key={i}>{l}</div>)
          )}
        </div>
      </div>
    </>
  );
}
