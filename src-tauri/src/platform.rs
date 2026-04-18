//! Per-phone-OS abstraction. iPhone goes through `pmd3` + libimobiledevice;
//! Android goes through the `adb` CLI. Each device is tagged with which
//! backend to use so the UI can route commands correctly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DevicePlatform {
    Ios,
    Android,
}
