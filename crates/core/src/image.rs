//! Image model and header detection. Header sniffing is a pure function over
//! bytes so it stays wasm-safe and unit-testable; file reading is the
//! backend's job.

use serde::{Deserialize, Serialize};

/// What kind of bootable image we think the file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageKind {
    /// ISO 9660 optical image (`CD001` at sector 16).
    Iso9660,
    /// Raw disk image with an MBR or GPT partition scheme.
    DiskImage,
    /// A compressed image (`.img.xz`, `.img.gz`, `.zip`) that must be
    /// decompressed before writing.
    Compressed,
    /// Anything else; written as a raw stream.
    Raw,
}

impl ImageKind {
    pub fn name(self) -> &'static str {
        match self {
            ImageKind::Iso9660 => "ISO 9660",
            ImageKind::DiskImage => "disk image",
            ImageKind::Compressed => "compressed image",
            ImageKind::Raw => "raw image",
        }
    }
}

/// What we know about a source image file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub path: String,
    pub size_bytes: u64,
    pub kind: ImageKind,
    /// Set when a decompressor must run between the file and the writer.
    pub compression: Option<String>,
}

/// Sniff the first bytes of a file. `header` should be at least the first
/// 33 KiB to detect ISO 9660 (primary volume descriptor at offset 0x8001).
pub fn detect_kind(header: &[u8]) -> ImageKind {
    const ISO_MAGIC: &[u8] = b"CD001";
    const ISO_OFFSET: usize = 0x8001;
    if header.len() >= ISO_OFFSET + ISO_MAGIC.len()
        && &header[ISO_OFFSET..ISO_OFFSET + ISO_MAGIC.len()] == ISO_MAGIC
    {
        return ImageKind::Iso9660;
    }
    // MBR boot signature 0x55AA at the end of sector 0.
    if header.len() >= 512 && header[510] == 0x55 && header[511] == 0xAA {
        return ImageKind::DiskImage;
    }
    ImageKind::Raw
}

/// Compression wrapper implied by the file name, if any.
pub fn compression_from_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    for ext in [".xz", ".gz", ".zst", ".bz2", ".zip"] {
        if lower.ends_with(ext) {
            return Some(ext.trim_start_matches('.').to_string());
        }
    }
    None
}

/// Build an `ImageInfo` from a file name, its size and a header sniff.
pub fn describe(path: &str, size_bytes: u64, header: &[u8]) -> ImageInfo {
    let kind = if compression_from_name(path).is_some() {
        ImageKind::Compressed
    } else {
        detect_kind(header)
    };
    ImageInfo {
        path: path.to_string(),
        size_bytes,
        kind,
        compression: compression_from_name(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_iso9660() {
        let mut header = vec![0u8; 0x8006];
        header[0x8001..0x8006].copy_from_slice(b"CD001");
        assert_eq!(detect_kind(&header), ImageKind::Iso9660);
    }

    #[test]
    fn detects_mbr_disk_image() {
        let mut header = vec![0u8; 512];
        header[510] = 0x55;
        header[511] = 0xAA;
        assert_eq!(detect_kind(&header), ImageKind::DiskImage);
    }

    #[test]
    fn unknown_is_raw() {
        assert_eq!(detect_kind(&[1, 2, 3]), ImageKind::Raw);
        assert_eq!(detect_kind(&[]), ImageKind::Raw);
    }

    #[test]
    fn short_header_never_reads_past_end() {
        // Adversarial: exactly one byte short of the MBR signature window.
        assert_eq!(detect_kind(&vec![0u8; 511]), ImageKind::Raw);
    }

    #[test]
    fn compression_by_extension() {
        assert_eq!(
            compression_from_name("fedora.img.xz").as_deref(),
            Some("xz")
        );
        assert_eq!(compression_from_name("x.ZIP").as_deref(), Some("zip"));
        assert_eq!(compression_from_name("x.iso"), None);
        assert_eq!(
            describe("a.img.gz", 10, &[0u8; 8]).kind,
            ImageKind::Compressed
        );
    }
}
