//! Bridge to the pymobiledevice3 helper. Spawns the helper once in
//! `daemon` mode and talks to it over stdin/stdout — that keeps the
//! Python import and any live lockdown sessions warm so per-call
//! latency drops from ~1-5s (subprocess cold-start + mDNS browse) to
//! ~50-500ms for cached flows.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::error::{LinkdropError, Result};

/// Resource dir from Tauri's AppHandle — set once at startup.
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn venv_python() -> PathBuf {
    let candidates: Vec<PathBuf> = if let Some(home) = home_dir() {
        #[cfg(windows)]
        {
            vec![
                home.join("pipx/venvs/pymobiledevice3/Scripts/python.exe"),
                home.join("AppData/Local/pipx/pipx/venvs/pymobiledevice3/Scripts/python.exe"),
            ]
        }
        #[cfg(target_os = "macos")]
        {
            vec![
                home.join(".local/pipx/venvs/pymobiledevice3/bin/python"),
                home.join(".local/share/pipx/venvs/pymobiledevice3/bin/python"),
                PathBuf::from("/opt/homebrew/bin/python3"),
                PathBuf::from("/usr/local/bin/python3"),
            ]
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            vec![
                home.join(".local/share/pipx/venvs/pymobiledevice3/bin/python"),
                home.join(".local/pipx/venvs/pymobiledevice3/bin/python"),
            ]
        }
    } else {
        Vec::new()
    };
    for p in candidates {
        if p.exists() {
            return p;
        }
    }
    #[cfg(windows)]
    {
        PathBuf::from("python.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("python3")
    }
}

fn helper_script() -> PathBuf {
    if let Some(resources) = RESOURCE_DIR.get() {
        let p = resources.join("scripts/pmd3_helper.py");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/pmd3_helper.py")
}

// ─── Daemon manager ────────────────────────────────────────────────────────

struct Daemon {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

static DAEMON: OnceLock<Mutex<Option<Daemon>>> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn daemon_lock() -> &'static Mutex<Option<Daemon>> {
    DAEMON.get_or_init(|| Mutex::new(None))
}

fn spawn_daemon() -> Result<Daemon> {
    let mut child = Command::new(venv_python())
        .arg(helper_script())
        .arg("daemon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| LinkdropError::ToolFailed {
            tool: "pmd3_helper daemon".into(),
            status: e.kind().to_string(),
            stderr: e.to_string(),
        })?;

    let stdin = child.stdin.take().ok_or_else(|| LinkdropError::ToolFailed {
        tool: "pmd3_helper daemon".into(),
        status: "no stdin".into(),
        stderr: "".into(),
    })?;
    let stdout = child.stdout.take().ok_or_else(|| LinkdropError::ToolFailed {
        tool: "pmd3_helper daemon".into(),
        status: "no stdout".into(),
        stderr: "".into(),
    })?;
    let mut daemon = Daemon {
        child,
        stdin,
        stdout: BufReader::new(stdout),
    };

    // Consume the `ready` banner.
    let mut line = String::new();
    daemon
        .stdout
        .read_line(&mut line)
        .map_err(|e| LinkdropError::ToolFailed {
            tool: "pmd3_helper daemon".into(),
            status: "ready read".into(),
            stderr: e.to_string(),
        })?;
    Ok(daemon)
}

fn call_daemon(op: &str, args: &[&str]) -> Result<serde_json::Value> {
    let guard = daemon_lock();
    let mut slot = guard.lock().expect("daemon mutex poisoned");

    // Lazy spawn + restart-if-dead.
    if slot.as_mut().map(|d| d.child.try_wait().ok().flatten().is_some()).unwrap_or(true) {
        *slot = Some(spawn_daemon()?);
    }
    let daemon = slot.as_mut().expect("just-spawned");

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let req = serde_json::json!({
        "id": id,
        "op": op,
        "args": args,
    });
    let serialized = serde_json::to_string(&req).expect("json serialize");
    writeln!(daemon.stdin, "{}", serialized).map_err(|e| {
        // Write failure usually means the daemon died. Drop it so next call respawns.
        *slot = None;
        LinkdropError::ToolFailed {
            tool: "pmd3_helper daemon".into(),
            status: "write".into(),
            stderr: e.to_string(),
        }
    })?;

    let daemon = slot.as_mut().expect("still-alive");
    let mut line = String::new();
    loop {
        line.clear();
        let n = daemon.stdout.read_line(&mut line).map_err(|e| {
            LinkdropError::ToolFailed {
                tool: "pmd3_helper daemon".into(),
                status: "read".into(),
                stderr: e.to_string(),
            }
        })?;
        if n == 0 {
            *slot = None;
            return Err(LinkdropError::ToolFailed {
                tool: "pmd3_helper daemon".into(),
                status: "eof".into(),
                stderr: "daemon closed stdout".into(),
            });
        }
        let v: serde_json::Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(_) => continue, // ignore non-JSON debug lines
        };
        // Ignore unsolicited events (`{"event":"ready"}` etc.) — match on id.
        let got_id = v.get("id").and_then(|x| x.as_u64());
        if got_id != Some(id) {
            continue;
        }
        let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        if !ok {
            let err = v
                .get("error")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(LinkdropError::ToolFailed {
                tool: format!("pmd3 {}", op),
                status: "error".into(),
                stderr: err,
            });
        }
        return Ok(v.get("data").cloned().unwrap_or(serde_json::Value::Null));
    }
}

// ─── Public API: callers stay the same ─────────────────────────────────────

pub fn run(op: &str, udid: &str) -> Result<String> {
    run_with_args(op, &[udid])
}

pub fn run_with_args(op: &str, args: &[&str]) -> Result<String> {
    let v = call_daemon(op, args)?;
    // Serialize back to a JSON string so existing call-sites (which
    // parse the output with serde_json::from_str) keep working.
    Ok(serde_json::to_string(&v).unwrap_or_else(|_| "null".into()))
}
