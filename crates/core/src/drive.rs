//! Drive model shared by every frontend.

use serde::{Deserialize, Serialize};

/// One candidate target device reported by a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    /// Kernel name, e.g. `sdb`.
    pub name: String,
    /// Device node, e.g. `/dev/sdb`.
    pub path: String,
    pub size_bytes: u64,
    /// Sysfs `removable` flag.
    pub removable: bool,
    pub vendor: Option<String>,
    pub model: Option<String>,
    /// Mounted partition device nodes, e.g. `["/dev/sdb1"]`.
    pub mounted: Vec<String>,
}

impl DriveInfo {
    /// A drive may only be written when it is removable, has a size and is
    /// not mounted. The reason string is for the UI, never an English key.
    pub fn writable(&self) -> std::result::Result<(), &'static str> {
        if !self.removable {
            return Err("device is not removable");
        }
        if self.size_bytes == 0 {
            return Err("device reports zero size");
        }
        if !self.mounted.is_empty() {
            return Err("device has mounted partitions");
        }
        Ok(())
    }
}
