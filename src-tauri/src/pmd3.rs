//! Bridge to the pymobiledevice3 helper script for Wi-Fi-transport ops.
//! Runs `scripts/pmd3_helper.py` via the pipx-installed venv's Python.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use crate::error::{LinkdropError, Result};

/// Set by `lib::run`'s setup hook once at startup, using the Tauri app's
/// bundled resource directory (e.g. `/usr/lib/linkdrop/resources` for a
/// .deb install, `/tmp/.mount_*/usr/lib/linkdrop/resources` for an AppImage).
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
    // pipx-installed pymobiledevice3 venv — locations vary by OS and pipx
    // version. Fall through to whatever `python3` / `python` resolves to on
    // PATH if the venv isn't where we expect.
    let candidates: Vec<PathBuf> = if let Some(home) = home_dir() {
        #[cfg(windows)]
        {
            vec![
                // pipx ≥ 1.3 default
                home.join("pipx")
                    .join("venvs")
                    .join("pymobiledevice3")
                    .join("Scripts")
                    .join("python.exe"),
                // pipx < 1.3 (XDG-ish)
                home.join("AppData")
                    .join("Local")
                    .join("pipx")
                    .join("pipx")
                    .join("venvs")
                    .join("pymobiledevice3")
                    .join("Scripts")
                    .join("python.exe"),
            ]
        }
        #[cfg(target_os = "macos")]
        {
            vec![
                home.join(".local/pipx/venvs/pymobiledevice3/bin/python"),
                home.join(".local/share/pipx/venvs/pymobiledevice3/bin/python"),
                // Homebrew Python fallback
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

pub fn run(op: &str, udid: &str) -> Result<String> {
    run_with_args(op, &[udid])
}

pub fn run_with_args(op: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(venv_python())
        .arg(helper_script())
        .arg(op)
        .args(args)
        .output()
        .map_err(|e| LinkdropError::ToolFailed {
            tool: "pmd3_helper".into(),
            status: e.kind().to_string(),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(LinkdropError::ToolFailed {
            tool: "pmd3_helper".into(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
