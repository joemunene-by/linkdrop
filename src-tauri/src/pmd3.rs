//! Bridge to the pymobiledevice3 helper script for Wi-Fi-transport ops.
//! Runs `scripts/pmd3_helper.py` via the pipx-installed venv's Python.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{LinkdropError, Result};

fn venv_python() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home)
            .join(".local/share/pipx/venvs/pymobiledevice3/bin/python");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("python3")
}

fn helper_script() -> PathBuf {
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
