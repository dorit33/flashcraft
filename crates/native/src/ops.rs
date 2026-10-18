//! Image operations: inspect, write, verify. Writes refuse anything that is
//! not a checked `WritePlan`; verify re-reads the device byte-for-byte.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Instant;

use flashcraft_core::{Error, ImageInfo, Progress, Result, WritePlan};

const HEADER_LEN: usize = 64 * 1024;
const CHUNK: usize = 4 * 1024 * 1024;

/// Read the header and report what the file looks like.
pub fn inspect_image(path: &str) -> Result<ImageInfo> {
    let meta = fs::metadata(path).map_err(|e| Error::BadImage(format!("{path}: {e}")))?;
    if !meta.is_file() {
        return Err(Error::BadImage(format!("{path}: not a regular file")));
    }
    let mut header = vec![0u8; HEADER_LEN];
    let mut f = File::open(path).map_err(|e| Error::BadImage(format!("{path}: {e}")))?;
    let n = f.read(&mut header).map_err(|e| Error::Io(format!("{e}")))?;
    header.truncate(n);
    Ok(flashcraft_core::image::describe(path, meta.len(), &header))
}

/// Write a checked plan to its device. `on_progress` fires after each chunk.
/// Destructive: every byte of the target is overwritten.
pub fn write_image(plan: &WritePlan, on_progress: &mut dyn FnMut(Progress)) -> Result<()> {
    let image_path = plan.image.path.clone();
    let drive_path = plan.drive.path.clone();

    let mut src =
        File::open(&image_path).map_err(|e| Error::BadImage(format!("{image_path}: {e}")))?;
    // Fail fast if we cannot open the device (needs root on most systems).
    let mut dst = fs::OpenOptions::new()
        .write(true)
        .open(&drive_path)
        .map_err(|e| {
            Error::Io(format!(
                "{drive_path}: {e} (a block device írásához rendszergazda jog kell — pkexec/sudo)"
            ))
        })?;

    let total = plan.image.size_bytes;
    let mut buf = vec![0u8; CHUNK];
    let mut written: u64 = 0;
    let start = Instant::now();
    loop {
        let want = (total - written).min(CHUNK as u64) as usize;
        if want == 0 {
            break;
        }
        let n = src
            .read(&mut buf[..want])
            .map_err(|e| Error::Io(format!("reading image: {e}")))?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])
            .map_err(|e| Error::Io(format!("writing {drive_path}: {e}")))?;
        written += n as u64;
        let secs = start.elapsed().as_secs_f64();
        on_progress(Progress {
            written,
            total,
            bytes_per_sec: if secs > 0.0 {
                (written as f64 / secs) as u64
            } else {
                0
            },
        });
    }
    dst.sync_all()
        .map_err(|e| Error::Io(format!("sync: {e}")))?;
    Ok(())
}

/// Read the device back and compare it byte-for-byte with the image.
pub fn verify_image(plan: &WritePlan, on_progress: &mut dyn FnMut(Progress)) -> Result<()> {
    let mut src = File::open(&plan.image.path)
        .map_err(|e| Error::BadImage(format!("{}: {e}", plan.image.path)))?;
    let mut dst =
        File::open(&plan.drive.path).map_err(|e| Error::Io(format!("{}: {e}", plan.drive.path)))?;
    // Verification only covers the image length; trailing drive bytes are
    // allowed to differ.
    dst.seek(SeekFrom::Start(0))
        .map_err(|e| Error::Io(format!("{e}")))?;

    let total = plan.image.size_bytes;
    let mut a = vec![0u8; CHUNK];
    let mut b = vec![0u8; CHUNK];
    let mut read_done: u64 = 0;
    let start = Instant::now();
    while read_done < total {
        let want = (total - read_done).min(CHUNK as u64) as usize;
        let na = src
            .read(&mut a[..want])
            .map_err(|e| Error::Io(format!("{e}")))?;
        let nb = dst
            .read(&mut b[..want])
            .map_err(|e| Error::Io(format!("{e}")))?;
        if na != nb {
            return Err(Error::Verify(format!(
                "short read at offset {read_done} (image {na}, device {nb})"
            )));
        }
        if a[..na] != b[..nb] {
            return Err(Error::Verify(format!(
                "content differs at offset {read_done}"
            )));
        }
        read_done += na as u64;
        if na == 0 {
            break;
        }
        let secs = start.elapsed().as_secs_f64();
        on_progress(Progress {
            written: read_done,
            total,
            bytes_per_sec: if secs > 0.0 {
                (read_done as f64 / secs) as u64
            } else {
                0
            },
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashcraft_core::{DriveInfo, ImageKind};

    fn tmpfile(name: &str, data: &[u8]) -> String {
        let p = std::env::temp_dir().join(format!("flashcraft-{}-{name}", std::process::id()));
        fs::write(&p, data).ok();
        p.to_string_lossy().to_string()
    }

    fn plan(image_path: &str, drive_path: &str, image_size: u64, drive_size: u64) -> WritePlan {
        let image = ImageInfo {
            path: image_path.to_string(),
            size_bytes: image_size,
            kind: ImageKind::Iso9660,
            compression: None,
        };
        let drive = DriveInfo {
            name: "test".into(),
            path: drive_path.to_string(),
            size_bytes: drive_size,
            removable: true,
            vendor: None,
            model: None,
            mounted: vec![],
        };
        // WritePlan::new refuses non-device sizes only via fields we control;
        // for tests we drive the ops directly on regular files.
        WritePlan { image, drive }
    }

    #[test]
    fn write_then_verify_roundtrip() {
        let data: Vec<u8> = (0..1_000_000u32).map(|i| (i % 251) as u8).collect();
        let img = tmpfile("img.bin", &data);
        let dev = tmpfile("dev.bin", &vec![0u8; data.len() + 100]);

        let p = plan(&img, &dev, data.len() as u64, data.len() as u64 + 100);
        write_image(&p, &mut |_| {}).unwrap();
        verify_image(&p, &mut |_| {}).unwrap();

        let _ = fs::remove_file(&img);
        let _ = fs::remove_file(&dev);
    }

    #[test]
    fn verify_detects_corruption() {
        let data = vec![7u8; 10_000];
        let img = tmpfile("img2.bin", &data);
        let mut corrupt = vec![7u8; 10_000];
        corrupt[5_000] = 9;
        let dev = tmpfile("dev2.bin", &corrupt);

        // Pretend the image was already written: verify only compares bytes.
        let p = plan(&img, &dev, data.len() as u64, data.len() as u64);
        assert!(matches!(
            verify_image(&p, &mut |_| {}),
            Err(Error::Verify(_))
        ));

        let _ = fs::remove_file(&img);
        let _ = fs::remove_file(&dev);
    }

    #[test]
    fn inspect_reports_iso() {
        let mut data = vec![0u8; 0x8006];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        let img = tmpfile("x.iso", &data);
        let info = inspect_image(&img).unwrap();
        assert_eq!(info.kind, ImageKind::Iso9660);
        let _ = fs::remove_file(&img);
    }
}
