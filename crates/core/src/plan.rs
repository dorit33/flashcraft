//! Write and format plans plus hostile-input parameter validation.

use serde::{Deserialize, Serialize};

use crate::drive::DriveInfo;
use crate::error::{Error, Result};
use crate::image::ImageInfo;

/// Partition table scheme for drive formatting / partitioning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionScheme {
    #[default]
    Mbr,
    Gpt,
}

impl PartitionScheme {
    pub fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("gpt") {
            Self::Gpt
        } else {
            Self::Mbr
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mbr => "MBR",
            Self::Gpt => "GPT",
        }
    }
}

/// Target filesystem for formatting a partition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileSystem {
    #[default]
    Fat32,
    Ntfs,
    ExFat,
    Udf,
    Ext2,
    Ext3,
    Ext4,
}

impl FileSystem {
    pub fn parse(s: &str) -> Self {
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("ntfs") {
            Self::Ntfs
        } else if lower.starts_with("exfat") {
            Self::ExFat
        } else if lower.starts_with("udf") {
            Self::Udf
        } else if lower.starts_with("ext4") {
            Self::Ext4
        } else if lower.starts_with("ext3") {
            Self::Ext3
        } else if lower.starts_with("ext2") {
            Self::Ext2
        } else {
            Self::Fat32
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fat32 => "FAT32",
            Self::Ntfs => "NTFS",
            Self::ExFat => "exFAT",
            Self::Udf => "UDF",
            Self::Ext2 => "ext2",
            Self::Ext3 => "ext3",
            Self::Ext4 => "ext4",
        }
    }
}

/// Parse a cluster-size UI string (e.g. `"4096 bytes (default)"`, `"16 kilobytes"`)
/// into a byte count.
pub fn parse_cluster_bytes(s: &str) -> u32 {
    let lower = s.to_ascii_lowercase();
    let num: u32 = lower
        .split_whitespace()
        .next()
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(4096);
    if lower.contains("kilo") || lower.ends_with("kb") || lower.ends_with('k') {
        num.saturating_mul(1024).clamp(512, 65536)
    } else {
        num.clamp(512, 65536)
    }
}

/// Supported on-the-fly compression formats.
pub fn is_supported_compression(comp: &str) -> bool {
    matches!(comp, "xz" | "gz" | "zst" | "bz2" | "zip")
}

/// Validated parameters of a `media.write` / `media.verify` call.
#[derive(Debug, Clone, Deserialize)]
pub struct WriteParams {
    pub image: String,
    pub drive: String,
}

/// Parse params from the JSON control form. Adversarial input must produce
/// `Err`, never a panic.
pub fn validate_write_params(value: &serde_json::Value) -> Result<WriteParams> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::InvalidParams("expected an object".into()))?;
    let get_str = |key: &str| -> Result<String> {
        let v = obj
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidParams(format!("missing or non-string `{key}`")))?;
        if v.is_empty() || v.len() > 4096 {
            return Err(Error::InvalidParams(format!("bad `{key}`")));
        }
        Ok(v.to_string())
    };
    Ok(WriteParams {
        image: get_str("image")?,
        drive: get_str("drive")?,
    })
}

/// A checked write plan: the frontend builds this and only then may a
/// backend perform the write.
#[derive(Debug, Clone)]
pub struct WritePlan {
    pub image: ImageInfo,
    pub drive: DriveInfo,
    /// The user-typed volume label; applied to the written media where the
    /// format allows (ISO 9660 PVD, FAT32/NTFS BPB), `None` leaves untouched.
    pub volume_label: Option<String>,
    /// Number of bad-block check passes before writing (0..=4).
    pub bad_block_passes: u8,
    /// Persistent partition size in MiB to append after an ISO write (0 = off).
    pub persistence_mib: u64,
    /// Apply old-BIOS compatibility fixes (ensure active boot flag in MBR).
    pub old_bios_fixes: bool,
    /// Automatically verify media after writing (`Enable runtime UEFI media validation`).
    pub verify_after: bool,
    /// Create extended label (`autorun.inf` + `autorun.ico`) where supported.
    pub extended_label: bool,
}

impl WritePlan {
    /// Refuse every unsafe combination. The checks mirror what the UI shows
    /// so a CLI, the app and an agent all get the same answer.
    pub fn new(image: ImageInfo, drive: DriveInfo) -> Result<Self> {
        drive
            .writable()
            .map_err(|reason| Error::BadDrive(format!("{}: {reason}", drive.path)))?;
        if image.size_bytes == 0 {
            return Err(Error::BadImage("image is empty".into()));
        }
        if let Some(comp) = &image.compression {
            if !is_supported_compression(comp) {
                return Err(Error::BadImage(format!(
                    "unsupported compression format: {comp}"
                )));
            }
        }
        if image.size_bytes > drive.size_bytes {
            return Err(Error::Refused(format!(
                "image ({} bytes) is larger than the drive ({} bytes)",
                image.size_bytes, drive.size_bytes
            )));
        }
        Ok(Self {
            image,
            drive,
            volume_label: None,
            bad_block_passes: 0,
            persistence_mib: 0,
            old_bios_fixes: false,
            verify_after: false,
            extended_label: false,
        })
    }

    /// Attach the volume label (blank → `None`). Rufus applies the typed
    /// label to the target media; we do the same where the format allows.
    pub fn with_label(mut self, label: Option<String>) -> Self {
        self.volume_label = label.filter(|s| !s.trim().is_empty());
        self
    }

    pub fn with_bad_blocks(mut self, passes: u8) -> Self {
        self.bad_block_passes = passes.min(4);
        self
    }

    pub fn with_persistence_mib(mut self, mib: u64) -> Self {
        self.persistence_mib = mib;
        self
    }

    pub fn with_old_bios_fixes(mut self, on: bool) -> Self {
        self.old_bios_fixes = on;
        self
    }

    pub fn with_verify_after(mut self, on: bool) -> Self {
        self.verify_after = on;
        self
    }

    pub fn with_extended_label(mut self, on: bool) -> Self {
        self.extended_label = on;
        self
    }
}

/// A checked format plan (`Non bootable` mode in Rufus): partition + format
/// a removable drive without an image file.
#[derive(Debug, Clone)]
pub struct FormatPlan {
    pub drive: DriveInfo,
    pub scheme: PartitionScheme,
    pub fs: FileSystem,
    pub cluster_bytes: u32,
    pub volume_label: Option<String>,
    pub quick_format: bool,
    pub extended_label: bool,
    pub bad_block_passes: u8,
    pub old_bios_fixes: bool,
}

impl FormatPlan {
    pub fn new(drive: DriveInfo, scheme: PartitionScheme, fs: FileSystem) -> Result<Self> {
        drive
            .writable()
            .map_err(|reason| Error::BadDrive(format!("{}: {reason}", drive.path)))?;
        Ok(Self {
            drive,
            scheme,
            fs,
            cluster_bytes: 4096,
            volume_label: None,
            quick_format: true,
            extended_label: false,
            bad_block_passes: 0,
            old_bios_fixes: false,
        })
    }

    pub fn with_label(mut self, label: Option<String>) -> Self {
        self.volume_label = label.filter(|s| !s.trim().is_empty());
        self
    }

    pub fn with_cluster_bytes(mut self, bytes: u32) -> Self {
        self.cluster_bytes = bytes.clamp(512, 65536);
        self
    }

    pub fn with_quick_format(mut self, quick: bool) -> Self {
        self.quick_format = quick;
        self
    }

    pub fn with_extended_label(mut self, on: bool) -> Self {
        self.extended_label = on;
        self
    }

    pub fn with_bad_blocks(mut self, passes: u8) -> Self {
        self.bad_block_passes = passes.min(4);
        self
    }

    pub fn with_old_bios_fixes(mut self, on: bool) -> Self {
        self.old_bios_fixes = on;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageKind;

    fn drive(removable: bool, size: u64, mounted: Vec<String>) -> DriveInfo {
        DriveInfo {
            name: "sdb".into(),
            path: "/dev/sdb".into(),
            size_bytes: size,
            removable,
            vendor: None,
            model: None,
            mounted,
        }
    }

    fn image(size: u64) -> ImageInfo {
        ImageInfo {
            path: "x.iso".into(),
            size_bytes: size,
            kind: ImageKind::Iso9660,
            compression: None,
        }
    }

    #[test]
    fn refuses_internal_drive() {
        let err = WritePlan::new(image(10), drive(false, 100, vec![]));
        assert!(matches!(err, Err(Error::BadDrive(_))));
        let ferr = FormatPlan::new(
            drive(false, 100, vec![]),
            PartitionScheme::Mbr,
            FileSystem::Fat32,
        );
        assert!(matches!(ferr, Err(Error::BadDrive(_))));
    }

    #[test]
    fn refuses_mounted_drive() {
        let err = WritePlan::new(image(10), drive(true, 100, vec!["/dev/sdb1".into()]));
        assert!(matches!(err, Err(Error::BadDrive(_))));
    }

    #[test]
    fn refuses_oversized_image() {
        let err = WritePlan::new(image(200), drive(true, 100, vec![]));
        assert!(matches!(err, Err(Error::Refused(_))));
    }

    #[test]
    fn accepts_good_plan_and_compressed() {
        assert!(WritePlan::new(image(10), drive(true, 100, vec![])).is_ok());
        let mut comp = image(10);
        comp.kind = ImageKind::Compressed;
        comp.compression = Some("xz".into());
        assert!(WritePlan::new(comp, drive(true, 100, vec![])).is_ok());
    }

    #[test]
    fn cluster_and_fs_parsing() {
        assert_eq!(parse_cluster_bytes("4096 bytes (default)"), 4096);
        assert_eq!(parse_cluster_bytes("16 kilobytes"), 16384);
        assert_eq!(FileSystem::parse("FAT32 (default)"), FileSystem::Fat32);
        assert_eq!(FileSystem::parse("NTFS"), FileSystem::Ntfs);
        assert_eq!(FileSystem::parse("exFAT"), FileSystem::ExFat);
        assert_eq!(FileSystem::parse("ext4"), FileSystem::Ext4);
    }

    #[test]
    fn params_reject_hostile_input() {
        assert!(validate_write_params(&serde_json::json!(null)).is_err());
        assert!(validate_write_params(&serde_json::json!({"image": 3})).is_err());
        assert!(validate_write_params(&serde_json::json!({
            "image": "a.iso",
            "drive": "/dev/sdb"
        }))
        .is_ok());
        assert!(validate_write_params(&serde_json::json!({
            "image": "a.iso",
            "drive": ""
        }))
        .is_err());
    }
}
