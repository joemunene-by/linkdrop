//! Transport abstraction: USB goes through stock usbmuxd, Wi-Fi goes through
//! a sidecar netmuxd daemon listening on 127.0.0.1:27015.

use std::process::Command;

use serde::{Deserialize, Serialize};

pub const NETMUXD_HOST: &str = "127.0.0.1:27015";

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Usb,
    Wifi,
}

pub fn muxd_command(tool: &str, transport: Transport) -> Command {
    let mut cmd = Command::new(tool);
    if transport == Transport::Wifi {
        cmd.env("USBMUXD_SOCKET_ADDRESS", NETMUXD_HOST);
    }
    cmd
}
