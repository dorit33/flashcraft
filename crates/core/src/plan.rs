//! Write plans and hostile-input parameter validation.

use serde::Deserialize;

use crate::drive::DriveInfo;
use crate::error::{Error, Result};
use crate::image::ImageInfo;

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
#[derive(Debug)]
pub struct WritePlan {
    pub image: ImageInfo,
    pub drive: DriveInfo,
    /// The user-typed volume label; applied to the written media where the
    /// format allows (ISO 9660 PVD today), `None` leaves the media untouched.
    pub volume_label: Option<String>,
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
        if image.compression.is_some() {
            return Err(Error::BadImage(
                "compressed images need a decompressor backend (not implemented yet)".into(),
            ));
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
        })
    }

    /// Attach the volume label (blank → `None`). Rufus applies the typed
    /// label to the target media; we do the same where the format allows.
    pub fn with_label(mut self, label: Option<String>) -> Self {
        self.volume_label = label.filter(|s| !s.trim().is_empty());
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
    fn accepts_good_plan() {
        assert!(WritePlan::new(image(10), drive(true, 100, vec![])).is_ok());
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
