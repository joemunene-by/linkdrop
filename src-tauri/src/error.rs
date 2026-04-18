use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkdropError {
    #[error("required tool not found on PATH: {0} — install with `sudo apt install {1}`")]
    MissingTool(&'static str, &'static str),

    #[error("tool `{tool}` exited with status {status}: {stderr}")]
    ToolFailed {
        tool: String,
        status: String,
        stderr: String,
    },

    #[error("no iPhone detected — is the device plugged in and trusted?")]
    NoDevice,

    #[error("unexpected output from `{tool}`: {detail}")]
    ParseError { tool: String, detail: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Serialize for LinkdropError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LinkdropError>;
