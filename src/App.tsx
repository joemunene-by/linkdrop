import { useEffect, useState } from "react";
import { api } from "./ipc";
import type { DeviceInfo, DeviceSummary, PhotoEntry, AirPlayStatus } from "./types";

type Tab = "device" | "photos" | "mirror";

const TABS: { key: Tab; label: string }[] = [
  { key: "device", label: "Device" },
  { key: "photos", label: "Photos" },
  { key: "mirror", label: "Screen mirror" },
];

export default function App() {
  const [tab, setTab] = useState<Tab>("device");
  const [devices, setDevices] = useState<DeviceSummary[]>([]);
  const [selectedUdid, setSelectedUdid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <div className="app">
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
                {d.udid.slice(0, 8)}…{d.udid.slice(-4)}
              </option>
            ))}
          </select>
          <button className="btn secondary" onClick={refreshDevices}>
            Refresh
          </button>
        </div>

        {error && <div className="error">{error}</div>}

        {tab === "device" && <DevicePanel udid={selectedUdid} />}
        {tab === "photos" && <PhotosPanel udid={selectedUdid} />}
        {tab === "mirror" && <MirrorPanel />}
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

function DevicePanel({ udid }: { udid: string | null }) {
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!udid) {
      setInfo(null);
      return;
    }
    setLoading(true);
    setError(null);
    api
      .getDeviceInfo(udid)
      .then(setInfo)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [udid]);

  if (!udid) {
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
      const r = await api.takeScreenshot(udid, outDir);
      alert(`Saved: ${r.path}`);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1>Device</h1>
      <p className="sub">
        {info?.name ? `${info.name} — ${info.model}` : "Loading device info…"}
      </p>

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
              <div className="label">iOS</div>
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
      </div>
    </>
  );
}

async function homeDir(): Promise<string> {
  return (
    (globalThis as unknown as { __TAURI_INTERNALS__?: { env?: { HOME?: string } } })
      .__TAURI_INTERNALS__?.env?.HOME || "/home"
  );
}

function PhotosPanel({ udid }: { udid: string | null }) {
  const [photos, setPhotos] = useState<PhotoEntry[]>([]);
  const [mounted, setMounted] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mount = async () => {
    if (!udid) return;
    setLoading(true);
    setError(null);
    try {
      await api.mountDevice(udid);
      setMounted(true);
      const items = await api.listPhotos(200);
      setPhotos(items);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const unmount = async () => {
    try {
      await api.unmountDevice();
    } catch (e) {
      setError(String(e));
    }
    setMounted(false);
    setPhotos([]);
  };

  useEffect(() => {
    // Don't auto-mount; require explicit button press (mounting is visible to the OS).
    return () => {
      if (mounted) {
        api.unmountDevice().catch(() => {});
      }
    };

  }, [mounted]);

  return (
    <>
      <h1>Photos</h1>
      <p className="sub">Browse DCIM from your iPhone's Photos roll via ifuse.</p>

      {error && <div className="error">{error}</div>}

      <div className="card">
        <div className="row">
          <button className="btn" onClick={mount} disabled={!udid || mounted || loading}>
            {mounted ? "Mounted" : "Mount device"}
          </button>
          <button
            className="btn secondary"
            onClick={unmount}
            disabled={!mounted}
          >
            Unmount
          </button>
          {loading && <span style={{ color: "var(--text-dim)" }}>Reading…</span>}
          {mounted && (
            <span className="pill ok">{photos.length} item(s)</span>
          )}
        </div>
      </div>

      {mounted && (
        <div className="card">
          <h2>DCIM</h2>
          {photos.length === 0 ? (
            <div className="empty">No photos found.</div>
          ) : (
            <div className="photo-grid">
              {photos.map((p) => (
                <div key={p.path} className="photo-tile" title={p.path}>
                  <div className="name">{p.name}</div>
                  <div className="kind">{p.kind}</div>
                  <div>{formatBytes(p.size_bytes)}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}

function MirrorPanel() {
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
      setStatus(await api.startAirplay("linkdrop"));
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
        Starts an AirPlay receiver (uxplay). Swipe down from the top-right on your iPhone and
        choose "linkdrop" under Screen Mirroring.
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
          uxplay opens its own window when the iPhone begins mirroring. Requires{" "}
          <code>uxplay</code> on PATH (<code>sudo apt install uxplay</code>).
        </p>
      </div>
    </>
  );
}
