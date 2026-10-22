//! Image and drive operations: inspect, checksum, bad-block scan, write,
//! verify, and standalone drive formatting (`Non bootable` mode).
//! Writes and formats refuse anything that is not a checked `WritePlan` /
//! `FormatPlan`; verify re-reads the device byte-for-byte.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use flashcraft_core::{
    Checksums, DriveInfo, Error, FileSystem, FormatPlan, ImageInfo, ImageKind, MultiHasher,
    PartitionScheme, Phase, Progress, Result, WritePlan,
};

const HEADER_LEN: usize = 64 * 1024;
const CHUNK: usize = 4 * 1024 * 1024;
/// ISO 9660 Primary Volume Descriptor lives at sector 16; the volume
/// identifier is 32 space-padded d-characters at offset 40 inside it.
const ISO_PVD_LABEL: u64 = 0x8028;
const ISO_PVD_LABEL_LEN: usize = 32;

/// The label bytes an ISO 9660 PVD accepts: ASCII upper case, digits, `_`,
/// right-padded with spaces to 32 bytes. Everything else becomes `_`.
fn label_bytes(label: &str) -> [u8; ISO_PVD_LABEL_LEN] {
    let mut out = [b' '; ISO_PVD_LABEL_LEN];
    for (i, b) in label.bytes().take(ISO_PVD_LABEL_LEN).enumerate() {
        if let Some(slot) = out.get_mut(i) {
            *slot = match b {
                b'a'..=b'z' => b - 32,
                b'A'..=b'Z' | b'0'..=b'9' | b'_' => b,
                _ => b'_',
            };
        }
    }
    out
}

/// Patch the ISO 9660 volume label inside `buf` (which holds file bytes
/// starting at `buf_off`). Only bytes overlapping the label field change.
fn apply_volume_label(buf: &mut [u8], buf_off: u64, label: &str) {
    let field_off = ISO_PVD_LABEL.saturating_sub(buf_off) as usize;
    let skip = buf_off.saturating_sub(ISO_PVD_LABEL) as usize;
    if field_off >= buf.len() || skip >= ISO_PVD_LABEL_LEN {
        return;
    }
    let padded = label_bytes(label);
    let n = (ISO_PVD_LABEL_LEN - skip).min(buf.len() - field_off);
    if let (Some(dst), Some(src)) = (
        buf.get_mut(field_off..field_off + n),
        padded.get(skip..skip + n),
    ) {
        dst.copy_from_slice(src);
    }
}

/// Rufus "Add fixes for old BIOSes": when writing sector 0 of an MBR image
/// that has no active partition flag (0x80), mark the first non-empty
/// partition entry active so strict legacy BIOSes boot it.
fn apply_old_bios_fix(buf: &mut [u8], buf_off: u64) {
    if buf_off != 0 || buf.len() < 512 {
        return;
    }
    if buf.get(510).copied() != Some(0x55) || buf.get(511).copied() != Some(0xAA) {
        return;
    }
    let offsets = [0x1BEusize, 0x1CE, 0x1DE, 0x1EE];
    let any_active = offsets
        .iter()
        .any(|&off| buf.get(off).copied() == Some(0x80));
    if any_active {
        return;
    }
    for &off in &offsets {
        let ptype = buf.get(off + 4).copied().unwrap_or(0);
        if ptype != 0 {
            if let Some(flag) = buf.get_mut(off) {
                *flag = 0x80;
            }
            break;
        }
    }
}

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

/// Compute MD5, SHA-1 and SHA-256 digests of `path` in a single streaming pass.
pub fn compute_checksums(path: &str, on_progress: &mut dyn FnMut(Progress)) -> Result<Checksums> {
    let meta = fs::metadata(path).map_err(|e| Error::BadImage(format!("{path}: {e}")))?;
    if !meta.is_file() {
        return Err(Error::BadImage(format!("{path}: not a regular file")));
    }
    let total = meta.len();
    let mut f = File::open(path).map_err(|e| Error::BadImage(format!("{path}: {e}")))?;
    let mut hasher = MultiHasher::new();
    let mut buf = vec![0u8; CHUNK];
    let mut done: u64 = 0;
    let start = Instant::now();
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| Error::Io(format!("reading {path}: {e}")))?;
        if n == 0 {
            break;
        }
        if let Some(slice) = buf.get(..n) {
            hasher.update(slice);
        }
        done = done.saturating_add(n as u64);
        let secs = start.elapsed().as_secs_f64();
        on_progress(Progress {
            written: done,
            total,
            bytes_per_sec: if secs > 0.0 {
                (done as f64 / secs) as u64
            } else {
                0
            },
            phase: Phase::Hashing,
        });
    }
    Ok(hasher.finalize())
}

/// Source stream for an image: either a direct file or a decompressor pipe.
enum ImageReader {
    File(File),
    Process(Child),
}

impl Read for ImageReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ImageReader::File(f) => f.read(buf),
            ImageReader::Process(c) => match c.stdout.as_mut() {
                Some(out) => out.read(buf),
                None => Ok(0),
            },
        }
    }
}

impl ImageReader {
    fn finish(self) -> Result<()> {
        match self {
            ImageReader::File(_) => Ok(()),
            ImageReader::Process(mut child) => {
                let status = child
                    .wait()
                    .map_err(|e| Error::Io(format!("decompressor wait: {e}")))?;
                if status.success() {
                    Ok(())
                } else {
                    Err(Error::BadImage(
                        "decompressor exited with an error (corrupt compressed archive?)".into(),
                    ))
                }
            }
        }
    }
}

fn open_image_stream(info: &ImageInfo) -> Result<ImageReader> {
    let Some(comp) = info.compression.as_deref() else {
        let f =
            File::open(&info.path).map_err(|e| Error::BadImage(format!("{}: {e}", info.path)))?;
        return Ok(ImageReader::File(f));
    };
    let (prog, args): (&str, Vec<&str>) = match comp {
        "xz" => ("xz", vec!["-dc", &info.path]),
        "gz" => ("gzip", vec!["-dc", &info.path]),
        "zst" => ("zstd", vec!["-dc", &info.path]),
        "bz2" => ("bzip2", vec!["-dc", &info.path]),
        "zip" => ("unzip", vec!["-p", &info.path]),
        other => {
            return Err(Error::BadImage(format!(
                "unsupported compression format: {other}"
            )))
        }
    };
    let child = Command::new(prog)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            Error::Backend(format!(
                "cannot launch `{prog}` to decompress {}: {e}",
                info.path
            ))
        })?;
    Ok(ImageReader::Process(child))
}

/// Scan a removable device for bad blocks across `passes` (1..=4) passes.
pub fn scan_bad_blocks(
    drive: &DriveInfo,
    passes: u8,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    drive
        .writable()
        .map_err(|reason| Error::BadDrive(format!("{}: {reason}", drive.path)))?;
    let passes = passes.clamp(1, 4) as u64;
    let total = drive.size_bytes.saturating_mul(passes);
    let mut f = File::open(&drive.path)
        .map_err(|e| Error::Io(format!("opening {} for bad-block scan: {e}", drive.path)))?;
    let mut buf = vec![0u8; CHUNK];
    let mut scanned: u64 = 0;
    let start = Instant::now();

    for pass in 0..passes {
        f.seek(SeekFrom::Start(0))
            .map_err(|e| Error::Io(format!("seek {}: {e}", drive.path)))?;
        let mut pass_read: u64 = 0;
        while pass_read < drive.size_bytes {
            let want = (drive.size_bytes - pass_read).min(CHUNK as u64) as usize;
            let n = f.read(&mut buf[..want]).map_err(|e| {
                Error::Io(format!(
                    "bad block on {} at offset {} (pass {}): {e}",
                    drive.path,
                    pass_read,
                    pass + 1
                ))
            })?;
            if n == 0 {
                break;
            }
            pass_read = pass_read.saturating_add(n as u64);
            scanned = scanned.saturating_add(n as u64);
            let secs = start.elapsed().as_secs_f64();
            on_progress(Progress {
                written: scanned,
                total,
                bytes_per_sec: if secs > 0.0 {
                    (scanned as f64 / secs) as u64
                } else {
                    0
                },
                phase: Phase::BadBlocks,
            });
        }
    }
    Ok(())
}

/// Write a checked plan to its device. `on_progress` fires after each chunk.
/// Destructive: every byte of the target is overwritten.
pub fn write_image(plan: &WritePlan, on_progress: &mut dyn FnMut(Progress)) -> Result<()> {
    if plan.bad_block_passes > 0 {
        scan_bad_blocks(&plan.drive, plan.bad_block_passes, on_progress)?;
    }

    let drive_path = plan.drive.path.clone();
    let mut src = open_image_stream(&plan.image)?;

    // Fail fast if we cannot open the device (needs root on most systems).
    let mut dst = fs::OpenOptions::new()
        .write(true)
        .open(&drive_path)
        .map_err(|e| {
            Error::Io(format!(
                "{drive_path}: {e} (a block device írásához rendszergazda jog kell — pkexec/sudo)"
            ))
        })?;

    let compressed = plan.image.compression.is_some();
    let mut total = plan.image.size_bytes;
    let label = match plan.image.kind {
        ImageKind::Iso9660 => plan.volume_label.clone(),
        _ => None,
    };
    let mut buf = vec![0u8; CHUNK];
    let mut written: u64 = 0;
    let start = Instant::now();
    loop {
        let want = if compressed {
            CHUNK
        } else {
            (total.saturating_sub(written)).min(CHUNK as u64) as usize
        };
        if want == 0 {
            break;
        }
        let n = src
            .read(&mut buf[..want])
            .map_err(|e| Error::Io(format!("reading image: {e}")))?;
        if n == 0 {
            break;
        }
        if written.saturating_add(n as u64) > plan.drive.size_bytes {
            return Err(Error::Refused(format!(
                "decompressed image exceeds drive capacity ({} bytes)",
                plan.drive.size_bytes
            )));
        }
        if let Some(slice) = buf.get_mut(..n) {
            if let Some(l) = &label {
                apply_volume_label(slice, written, l);
            }
            if plan.old_bios_fixes {
                apply_old_bios_fix(slice, written);
            }
            dst.write_all(slice)
                .map_err(|e| Error::Io(format!("writing {drive_path}: {e}")))?;
        }
        written = written.saturating_add(n as u64);
        if compressed && written > total {
            total = written;
        }
        let secs = start.elapsed().as_secs_f64();
        on_progress(Progress {
            written,
            total,
            bytes_per_sec: if secs > 0.0 {
                (written as f64 / secs) as u64
            } else {
                0
            },
            phase: Phase::Writing,
        });
    }
    src.finish()?;

    // The bar is full but the kernel still holds buffered pages — signal the
    // sync phase so the UI shows "flushing" instead of sitting on a full bar.
    on_progress(Progress {
        written,
        total: written.max(1),
        bytes_per_sec: 0,
        phase: Phase::Syncing,
    });
    dst.sync_all()
        .map_err(|e| Error::Io(format!("sync: {e}")))?;
    drop(dst);

    if plan.persistence_mib > 0 && plan.image.kind == ImageKind::Iso9660 {
        create_persistent_partition(&drive_path, plan.persistence_mib)?;
    }

    if plan.verify_after {
        verify_image(plan, on_progress)?;
    }

    Ok(())
}

/// Append a `writable` (casper-rw compatible) ext4 persistent partition after
/// a written hybrid ISO when the user configured `persistence_mib > 0`.
fn create_persistent_partition(drive_path: &str, persistence_mib: u64) -> Result<()> {
    let existing = list_partition_nodes(drive_path);
    let script = format!(",{}M,L\n", persistence_mib);
    let mut child = Command::new("sfdisk")
        .args(["--append", "--no-reread", drive_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Backend(format!("sfdisk: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(script.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| Error::Backend(format!("sfdisk wait: {e}")))?;
    if !out.status.success() {
        return Err(Error::Backend(format!(
            "cannot create persistent partition: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    settle_partitions(drive_path);
    let after = list_partition_nodes(drive_path);
    let new_part = after
        .into_iter()
        .find(|p| !existing.contains(p))
        .unwrap_or_else(|| partition_node(drive_path, 3));
    let mkfs = Command::new("mkfs.ext4")
        .args(["-F", "-q", "-L", "writable", &new_part])
        .output()
        .map_err(|e| Error::Backend(format!("mkfs.ext4: {e}")))?;
    if !mkfs.status.success() {
        return Err(Error::Backend(format!(
            "mkfs.ext4 on {new_part}: {}",
            String::from_utf8_lossy(&mkfs.stderr).trim()
        )));
    }
    Ok(())
}

/// Read the device back and compare it byte-for-byte with the image.
pub fn verify_image(plan: &WritePlan, on_progress: &mut dyn FnMut(Progress)) -> Result<()> {
    let mut src = open_image_stream(&plan.image)?;
    let mut dst =
        File::open(&plan.drive.path).map_err(|e| Error::Io(format!("{}: {e}", plan.drive.path)))?;
    dst.seek(SeekFrom::Start(0))
        .map_err(|e| Error::Io(format!("{e}")))?;

    let compressed = plan.image.compression.is_some();
    let mut total = plan.image.size_bytes;
    let label = match plan.image.kind {
        ImageKind::Iso9660 => plan.volume_label.clone(),
        _ => None,
    };
    let mut a = vec![0u8; CHUNK];
    let mut b = vec![0u8; CHUNK];
    let mut read_done: u64 = 0;
    let start = Instant::now();
    loop {
        let want = if compressed {
            CHUNK
        } else {
            (total.saturating_sub(read_done)).min(CHUNK as u64) as usize
        };
        if want == 0 {
            break;
        }
        let na = src
            .read(&mut a[..want])
            .map_err(|e| Error::Io(format!("{e}")))?;
        if na == 0 {
            break;
        }
        if let Some(slice) = a.get_mut(..na) {
            if let Some(l) = &label {
                apply_volume_label(slice, read_done, l);
            }
            if plan.old_bios_fixes {
                apply_old_bios_fix(slice, read_done);
            }
        }
        let mut nb_total = 0usize;
        while nb_total < na {
            let Some(dst_slice) = b.get_mut(nb_total..na) else {
                break;
            };
            let n = dst.read(dst_slice).map_err(|e| Error::Io(format!("{e}")))?;
            if n == 0 {
                break;
            }
            nb_total += n;
        }
        if na != nb_total {
            return Err(Error::Verify(format!(
                "short read at offset {read_done} (image {na}, device {nb_total})"
            )));
        }
        if a.get(..na) != b.get(..nb_total) {
            return Err(Error::Verify(format!(
                "content differs at offset {read_done}"
            )));
        }
        read_done = read_done.saturating_add(na as u64);
        if compressed && read_done > total {
            total = read_done;
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
            phase: Phase::Verifying,
        });
    }
    src.finish()?;
    Ok(())
}

fn partition_node(drive_path: &str, index: u8) -> String {
    let ends_with_digit = drive_path
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_digit());
    if ends_with_digit {
        format!("{drive_path}p{index}")
    } else {
        format!("{drive_path}{index}")
    }
}

fn list_partition_nodes(drive_path: &str) -> Vec<String> {
    let mut out = Vec::new();
    for i in 1..=8 {
        let cand = partition_node(drive_path, i);
        if Path::new(&cand).exists() {
            out.push(cand);
        }
    }
    out
}

fn settle_partitions(drive_path: &str) {
    let _ = Command::new("blockdev")
        .args(["--rereadpt", drive_path])
        .output();
    let _ = Command::new("udevadm").args(["settle"]).output();
    std::thread::sleep(std::time::Duration::from_millis(350));
}

/// Partition and format a removable drive (`Non bootable` mode in Rufus).
pub fn format_drive(plan: &FormatPlan, on_progress: &mut dyn FnMut(Progress)) -> Result<()> {
    plan.drive
        .writable()
        .map_err(|reason| Error::BadDrive(format!("{}: {reason}", plan.drive.path)))?;

    if plan.bad_block_passes > 0 {
        scan_bad_blocks(&plan.drive, plan.bad_block_passes, on_progress)?;
    }

    let drive_path = &plan.drive.path;
    let mut dst = fs::OpenOptions::new()
        .write(true)
        .open(drive_path)
        .map_err(|e| {
            Error::Io(format!(
                "{drive_path}: {e} (a formázáshoz rendszergazda jog kell — pkexec/sudo)"
            ))
        })?;

    let zeros = vec![0u8; CHUNK];
    if plan.quick_format {
        // Wipe the first and last 4 MiB so any leftover ISO 9660 PVD, MBR or
        // backup GPT header is erased cleanly before partitioning.
        on_progress(Progress {
            written: 0,
            total: 100,
            bytes_per_sec: 0,
            phase: Phase::Formatting,
        });
        let head = (CHUNK as u64).min(plan.drive.size_bytes) as usize;
        if let Some(slice) = zeros.get(..head) {
            dst.write_all(slice)
                .map_err(|e| Error::Io(format!("wiping header on {drive_path}: {e}")))?;
        }
        if plan.drive.size_bytes > CHUNK as u64 * 2 {
            let tail_off = plan.drive.size_bytes.saturating_sub(CHUNK as u64);
            if dst.seek(SeekFrom::Start(tail_off)).is_ok() {
                let _ = dst.write_all(&zeros);
            }
        }
        let _ = dst.sync_all();
    } else {
        // Full format: zero every sector on the device with live progress.
        let total = plan.drive.size_bytes;
        let mut written: u64 = 0;
        let start = Instant::now();
        while written < total {
            let want = (total - written).min(CHUNK as u64) as usize;
            if let Some(slice) = zeros.get(..want) {
                dst.write_all(slice)
                    .map_err(|e| Error::Io(format!("zeroing {drive_path}: {e}")))?;
            }
            written = written.saturating_add(want as u64);
            let secs = start.elapsed().as_secs_f64();
            on_progress(Progress {
                written,
                total,
                bytes_per_sec: if secs > 0.0 {
                    (written as f64 / secs) as u64
                } else {
                    0
                },
                phase: Phase::Formatting,
            });
        }
        let _ = dst.sync_all();
    }
    drop(dst);

    on_progress(Progress {
        written: 50,
        total: 100,
        bytes_per_sec: 0,
        phase: Phase::Formatting,
    });

    // Write the partition table via sfdisk.
    let label_kind = match plan.scheme {
        PartitionScheme::Mbr => "dos",
        PartitionScheme::Gpt => "gpt",
    };
    let part_type = match (plan.scheme, plan.fs) {
        (PartitionScheme::Mbr, FileSystem::Fat32) => "0c",
        (PartitionScheme::Mbr, FileSystem::Ntfs | FileSystem::ExFat | FileSystem::Udf) => "07",
        (PartitionScheme::Mbr, FileSystem::Ext2 | FileSystem::Ext3 | FileSystem::Ext4) => "83",
        (
            PartitionScheme::Gpt,
            FileSystem::Fat32 | FileSystem::Ntfs | FileSystem::ExFat | FileSystem::Udf,
        ) => "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7",
        (PartitionScheme::Gpt, FileSystem::Ext2 | FileSystem::Ext3 | FileSystem::Ext4) => "L",
    };
    let bootable = if plan.scheme == PartitionScheme::Mbr && plan.old_bios_fixes {
        ", bootable"
    } else {
        ""
    };
    let sfdisk_script = format!("label: {label_kind}\n1M,,{part_type}{bootable}\n");

    let mut sfdisk = Command::new("sfdisk")
        .args(["--wipe", "always", drive_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Backend(format!("cannot run sfdisk: {e}")))?;
    if let Some(mut stdin) = sfdisk.stdin.take() {
        let _ = stdin.write_all(sfdisk_script.as_bytes());
    }
    let sf_out = sfdisk
        .wait_with_output()
        .map_err(|e| Error::Backend(format!("sfdisk: {e}")))?;
    if !sf_out.status.success() {
        return Err(Error::Backend(format!(
            "sfdisk failed: {}",
            String::from_utf8_lossy(&sf_out.stderr).trim()
        )));
    }

    settle_partitions(drive_path);
    let part = partition_node(drive_path, 1);

    on_progress(Progress {
        written: 75,
        total: 100,
        bytes_per_sec: 0,
        phase: Phase::Formatting,
    });

    run_mkfs(&part, plan)?;

    if plan.extended_label {
        let label = plan.volume_label.as_deref().unwrap_or("FLASHCRAFT");
        let _ = write_extended_label_files(&part, label);
    }

    on_progress(Progress {
        written: 100,
        total: 100,
        bytes_per_sec: 0,
        phase: Phase::Syncing,
    });
    let _ = Command::new("sync").status();
    let _ = Command::new("udevadm").args(["settle"]).output();
    Ok(())
}

fn run_mkfs(part: &str, plan: &FormatPlan) -> Result<()> {
    let label = plan
        .volume_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (prog, mut args): (&str, Vec<String>) = match plan.fs {
        FileSystem::Fat32 => {
            let spc = (plan.cluster_bytes / 512).clamp(1, 128);
            let mut a = vec!["-F".into(), "32".into(), "-s".into(), spc.to_string()];
            if let Some(l) = label {
                let clean: String = l
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .take(11)
                    .collect();
                if !clean.is_empty() {
                    a.push("-n".into());
                    a.push(clean.to_ascii_uppercase());
                }
            }
            ("mkfs.vfat", a)
        }
        FileSystem::ExFat => {
            let mut a = vec!["-c".into(), plan.cluster_bytes.to_string()];
            if let Some(l) = label {
                let clean: String = l.chars().take(15).collect();
                if !clean.is_empty() {
                    a.push("-L".into());
                    a.push(clean);
                }
            }
            ("mkfs.exfat", a)
        }
        FileSystem::Ntfs => {
            let mut a = vec![
                "-f".into(),
                "-c".into(),
                plan.cluster_bytes.clamp(512, 65536).to_string(),
            ];
            if let Some(l) = label {
                let clean: String = l.chars().take(32).collect();
                if !clean.is_empty() {
                    a.push("-L".into());
                    a.push(clean);
                }
            }
            ("mkfs.ntfs", a)
        }
        FileSystem::Udf => {
            let mut a = Vec::new();
            if let Some(l) = label {
                let clean: String = l.chars().take(30).collect();
                if !clean.is_empty() {
                    a.push(format!("--label={clean}"));
                }
            }
            ("mkudffs", a)
        }
        FileSystem::Ext2 | FileSystem::Ext3 | FileSystem::Ext4 => {
            let prog = match plan.fs {
                FileSystem::Ext2 => "mkfs.ext2",
                FileSystem::Ext3 => "mkfs.ext3",
                _ => "mkfs.ext4",
            };
            let blk = plan.cluster_bytes.clamp(1024, 4096);
            let mut a = vec!["-F".into(), "-b".into(), blk.to_string()];
            if let Some(l) = label {
                let clean: String = l.chars().take(16).collect();
                if !clean.is_empty() {
                    a.push("-L".into());
                    a.push(clean);
                }
            }
            (prog, a)
        }
    };
    args.push(part.to_string());

    let out = Command::new(prog)
        .args(&args)
        .output()
        .map_err(|e| Error::Backend(format!("cannot run {prog}: {e}")))?;
    if !out.status.success() {
        return Err(Error::Backend(format!(
            "{prog} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Rufus "Create extended label and icon files": mount the freshly formatted
/// partition briefly and write `autorun.inf` with the full label.
fn write_extended_label_files(part: &str, label: &str) -> Result<()> {
    let mnt = std::env::temp_dir().join(format!("flashcraft-mnt-{}", std::process::id()));
    let _ = fs::create_dir_all(&mnt);
    let mnt_str = mnt.to_string_lossy().to_string();
    let status = Command::new("mount").args([part, &mnt_str]).status();
    if status.is_ok_and(|s| s.success()) {
        let autorun = format!(
            "[Autorun]\r\nlabel = {}\r\nicon = autorun.ico\r\n",
            label.replace(['\r', '\n'], " ")
        );
        let _ = fs::write(mnt.join("autorun.inf"), autorun.as_bytes());
        let _ = Command::new("sync").status();
        let _ = Command::new("umount").arg(&mnt_str).status();
    }
    let _ = fs::remove_dir(&mnt);
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
        WritePlan {
            image,
            drive,
            volume_label: None,
            bad_block_passes: 0,
            persistence_mib: 0,
            old_bios_fixes: false,
            verify_after: false,
            extended_label: false,
        }
    }

    #[test]
    fn labeled_iso_write_and_verify() {
        let mut data = vec![0u8; 0x8050];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        let img = tmpfile("iso.bin", &data);
        let dev = tmpfile("deviso.bin", &vec![0xFF; data.len()]);

        let mut p = plan(&img, &dev, data.len() as u64, data.len() as u64);
        p.image.kind = flashcraft_core::ImageKind::Iso9660;
        p = p.with_label(Some("my disk".to_string()));
        write_image(&p, &mut |_| {}).unwrap();
        verify_image(&p, &mut |_| {}).unwrap();

        let written = fs::read(&dev).unwrap();
        assert_eq!(&written[0x8028..0x8028 + 7], b"MY_DISK");
        assert_eq!(written[0x8028 + 32 - 1], b' ');

        let _ = fs::remove_file(&img);
        let _ = fs::remove_file(&dev);
    }

    #[test]
    fn old_bios_fix_marks_first_partition_active() {
        let mut mbr = vec![0u8; 512];
        mbr[0x1BE + 4] = 0x0c; // FAT32 LBA partition
        mbr[510] = 0x55;
        mbr[511] = 0xAA;
        apply_old_bios_fix(&mut mbr, 0);
        assert_eq!(mbr[0x1BE], 0x80);
    }

    #[test]
    fn checksums_compute_on_file() {
        let p = tmpfile("hash.bin", b"abc");
        let c = compute_checksums(&p, &mut |_| {}).unwrap();
        assert_eq!(c.md5, "900150983cd24fb0d6963f7d28e17f72");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn bad_blocks_scan_reads_passes() {
        let dev_path = tmpfile("bb.bin", &vec![0xAA; 65536]);
        let d = DriveInfo {
            name: "bb".into(),
            path: dev_path.clone(),
            size_bytes: 65536,
            removable: true,
            vendor: None,
            model: None,
            mounted: vec![],
        };
        let mut last_pct = 0;
        scan_bad_blocks(&d, 2, &mut |p| {
            last_pct = p.percent();
        })
        .unwrap();
        assert_eq!(last_pct, 100);
        let _ = fs::remove_file(&dev_path);
    }

    #[test]
    fn label_sanitizes_to_iso_dchars() {
        let bytes = label_bytes("a.b-c(d)");
        assert_eq!(&bytes[..8], b"A_B_C_D_");
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
