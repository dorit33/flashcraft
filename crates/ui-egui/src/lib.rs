//! FlashCraft shared egui UI — laid out to match Rufus: Drive Properties,
//! Format Options, Status bar with overlaid text, Checksums (`#`), Log,
//! About, and START / Verify / Cancel.
//! Thin by rule: view state and command dispatch only; device I/O goes
//! through `flashcraft-native` (desktop only). Every user-facing string goes
//! through `i18n`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod i18n;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc,
};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use egui::{Color32, RichText};
use i18n::{t, tf, Lang, Resolved};

use flashcraft_core as core;

const FILE_SYSTEMS: &[&str] = &[
    "FAT32 (default)",
    "NTFS",
    "exFAT",
    "UDF",
    "ext4",
    "ext3",
    "ext2",
];
const CLUSTER_SIZES: &[&str] = &[
    "4096 bytes (default)",
    "8192 bytes",
    "16 kilobytes",
    "32 kilobytes",
    "64 kilobytes",
];
const PARTITION_SCHEMES: &[&str] = &["MBR", "GPT"];
const TARGET_SYSTEMS: &[&str] = &["BIOS or UEFI", "UEFI (non CSM)"];
const IMAGE_OPTIONS: &[&str] = &["Standard installation", "DD Image (raw sector copy)"];
const BAD_BLOCK_PASSES: &[&str] = &["1 Pass", "2 Passes", "3 Passes", "4 Passes"];

/// Boot selection mode in Rufus's top combo box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootSelection {
    /// Write a disk or ISO image file (`media.write`).
    Image,
    /// Format the drive as non-bootable storage (`media.format`).
    NonBootable,
}

/// Which operation is running or awaiting confirmation.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Write,
    Verify,
    Format,
}

/// Message from the background write/verify/format/checksum job to the UI.
#[cfg(not(target_arch = "wasm32"))]
enum JobMsg {
    Progress(core::Progress),
    /// A localized status line keyed by an i18n catalog key.
    Status(&'static str),
    Checksums(core::Checksums),
    Done(Result<&'static str, String>),
}

/// The sibling `flashcraft-cli` binary: same directory as the running exe
/// (installed layout, portable zip, target dir), PATH as fallback.
#[cfg(not(target_arch = "wasm32"))]
fn cli_binary() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("flashcraft-cli");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    std::path::PathBuf::from("flashcraft-cli")
}

/// Elevated job — the Rufus UAC flow: the GUI runs unprivileged and polkit
/// asks for the password when the work actually starts. `pkexec` runs the
/// CLI (which unmounts, validates and writes/verifies/formats as root) and
/// streams JSON progress lines on stdout; stderr carries human messages/errors.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn run_elevated(
    kind: JobKind,
    image_path: &str,
    drive_path: &str,
    volume_label: &str,
    scheme: &str,
    file_system: &str,
    cluster_size: &str,
    quick_format: bool,
    extended_label: bool,
    bad_blocks: u8,
    persistence_mib: u64,
    old_bios_fixes: bool,
    verify_after: bool,
    cancel: &AtomicBool,
    tx: &Sender<JobMsg>,
    ctx: &egui::Context,
) -> core::Result<()> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    let mut cmd = Command::new("pkexec");
    cmd.arg(cli_binary());
    match kind {
        JobKind::Write => {
            cmd.arg("write")
                .arg(image_path)
                .arg(drive_path)
                .arg("--yes")
                .arg("--progress");
            if !volume_label.trim().is_empty() {
                cmd.arg("--label").arg(volume_label);
            }
            if bad_blocks > 0 {
                cmd.arg("--bad-blocks").arg(bad_blocks.to_string());
            }
            if persistence_mib > 0 {
                cmd.arg("--persistence").arg(persistence_mib.to_string());
            }
            if old_bios_fixes {
                cmd.arg("--old-bios-fixes");
            }
            if verify_after {
                cmd.arg("--verify");
            }
            if extended_label {
                cmd.arg("--extended-label");
            }
        }
        JobKind::Verify => {
            cmd.arg("verify")
                .arg(image_path)
                .arg(drive_path)
                .arg("--progress");
            if !volume_label.trim().is_empty() {
                cmd.arg("--label").arg(volume_label);
            }
            if old_bios_fixes {
                cmd.arg("--old-bios-fixes");
            }
        }
        JobKind::Format => {
            cmd.arg("format")
                .arg(drive_path)
                .arg("--yes")
                .arg("--progress")
                .arg("--scheme")
                .arg(scheme)
                .arg("--fs")
                .arg(file_system)
                .arg("--cluster")
                .arg(cluster_size);
            if !volume_label.trim().is_empty() {
                cmd.arg("--label").arg(volume_label);
            }
            if !quick_format {
                cmd.arg("--no-quick");
            }
            if extended_label {
                cmd.arg("--extended-label");
            }
            if bad_blocks > 0 {
                cmd.arg("--bad-blocks").arg(bad_blocks.to_string());
            }
            if old_bios_fixes {
                cmd.arg("--old-bios-fixes");
            }
        }
    }

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            core::Error::Backend(format!(
                "cannot elevate via pkexec: {e} (install polkit or run as root)"
            ))
        })?;
    let Some(stdout) = child.stdout.take() else {
        return Err(core::Error::Backend(
            "pkexec: could not capture stdout".into(),
        ));
    };
    for line in std::io::BufReader::new(stdout).lines() {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(core::Error::Refused("aborted by user".into()));
        }
        let Ok(line) = line else { break };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        if v.get("status").and_then(|s| s.as_str()) == Some("unmounting") {
            let _ = tx.send(JobMsg::Status("Unmounting mounted partitions"));
        } else if let Ok(p) = serde_json::from_value::<core::Progress>(v) {
            let _ = tx.send(JobMsg::Progress(p));
        }
        ctx.request_repaint();
    }
    let output = child
        .wait_with_output()
        .map_err(|e| core::Error::Io(format!("pkexec: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(core::Error::Refused("aborted by user".into()));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let msg = stderr
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("elevated command failed")
        .trim_start_matches("error: ")
        .to_string();
    Err(core::Error::Io(msg))
}

/// FlashCraft's view state — mirrors the Rufus dialog, field for field.
pub struct FlashcraftUi {
    // Settings & dialogs
    settings_open: bool,
    show_about: bool,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    show_checksums: bool,
    lang: Lang,
    // Drive Properties
    drives: Vec<core::DriveInfo>,
    selected_drive: Option<String>,
    boot_selection: BootSelection,
    image_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    image_info: Option<core::ImageInfo>,
    #[cfg(not(target_arch = "wasm32"))]
    checksums: Option<core::Checksums>,
    // Image & Partition Options
    image_option: String,
    persistence_mib: u64,
    partition_scheme: String,
    target_system: String,
    // Format Options
    volume_label: String,
    file_system: String,
    cluster_size: String,
    // Advanced sections
    show_adv_drive: bool,
    list_usb_hard_drives: bool,
    old_bios_fixes: bool,
    uefi_media_validation: bool,
    show_adv_format: bool,
    quick_format: bool,
    extended_label: bool,
    bad_blocks: bool,
    bad_block_passes: usize,
    // Run state
    status: String,
    status_err: bool,
    #[cfg(not(target_arch = "wasm32"))]
    progress: Option<core::Progress>,
    #[cfg(not(target_arch = "wasm32"))]
    job_started: Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel_flag: Arc<AtomicBool>,
    log: Vec<String>,
    show_log: bool,
    pending_confirm: Option<JobKind>,
    drives_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    job_rx: Option<Receiver<JobMsg>>,
}

impl Default for FlashcraftUi {
    fn default() -> Self {
        Self {
            settings_open: false,
            show_about: false,
            show_checksums: false,
            lang: Lang::Auto,
            drives: Vec::new(),
            selected_drive: None,
            boot_selection: BootSelection::Image,
            image_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            image_info: None,
            #[cfg(not(target_arch = "wasm32"))]
            checksums: None,
            image_option: IMAGE_OPTIONS[0].to_string(),
            persistence_mib: 0,
            partition_scheme: PARTITION_SCHEMES[0].to_string(),
            target_system: TARGET_SYSTEMS[0].to_string(),
            volume_label: String::new(),
            file_system: FILE_SYSTEMS[0].to_string(),
            cluster_size: CLUSTER_SIZES[0].to_string(),
            show_adv_drive: false,
            list_usb_hard_drives: false,
            old_bios_fixes: false,
            uefi_media_validation: false,
            show_adv_format: false,
            quick_format: true,
            extended_label: true,
            bad_blocks: false,
            bad_block_passes: 0,
            status: "READY".to_string(),
            status_err: false,
            #[cfg(not(target_arch = "wasm32"))]
            progress: None,
            #[cfg(not(target_arch = "wasm32"))]
            job_started: None,
            #[cfg(not(target_arch = "wasm32"))]
            cancel_flag: Arc::new(AtomicBool::new(false)),
            log: Vec::new(),
            show_log: false,
            pending_confirm: None,
            drives_error: None,
            #[cfg(not(target_arch = "wasm32"))]
            job_rx: None,
        }
    }
}

impl FlashcraftUi {
    fn say(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.status = msg.clone();
        self.log.push(msg);
    }

    /// Refresh the device list; honours the "List USB Hard Drives" toggle.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn refresh_drives(&mut self) {
        match flashcraft_native::linux::list_drives_live() {
            Ok(drives) => {
                self.drives = if self.list_usb_hard_drives {
                    drives
                } else {
                    drives.into_iter().filter(|d| d.removable).collect()
                };
                if self.selected_drive.is_none() {
                    self.selected_drive = self.drives.first().map(|d| d.path.clone());
                } else if let Some(sel) = &self.selected_drive {
                    if !self.drives.iter().any(|d| &d.path == sel) {
                        self.selected_drive = self.drives.first().map(|d| d.path.clone());
                    }
                }
                self.drives_error = None;
            }
            Err(e) => {
                self.drives.clear();
                self.drives_error = Some(e.to_string());
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn refresh_drives(&mut self) {
        self.drives.clear();
        self.drives_error = Some(
            t(
                "device access is not available in the browser",
                Resolved::En,
            )
            .to_string(),
        );
    }

    fn gib(bytes: u64) -> String {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    }

    fn drive_label(d: &core::DriveInfo) -> String {
        format!(
            "{} [{}] ({})",
            d.model.clone().unwrap_or_else(|| d.name.clone()),
            Self::gib(d.size_bytes),
            d.path
        )
    }

    /// Rufus autofills the volume label from the image name on SELECT.
    #[cfg(not(target_arch = "wasm32"))]
    fn on_image_picked(&mut self, path: String, lang: Resolved) {
        if let Some(stem) = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
        {
            self.volume_label = stem
                .trim_end_matches(".iso")
                .trim_end_matches(".img")
                .chars()
                .take(16)
                .collect();
        }
        self.boot_selection = BootSelection::Image;
        self.image_path = path;
        self.checksums = None;
        match flashcraft_native::inspect_image(&self.image_path) {
            Ok(info) => {
                self.status_err = false;
                self.say(tf(
                    "Selected '{0}'",
                    lang,
                    &format!(
                        "{} ({}, {})",
                        info.path,
                        Self::gib(info.size_bytes),
                        info.kind.name()
                    ),
                ));
                self.image_info = Some(info);
            }
            Err(e) => {
                self.image_info = None;
                self.say(e.to_string());
                self.status_err = true;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn browse_image(&mut self, lang: Resolved) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "All images",
                &[
                    "iso", "img", "bin", "raw", "xz", "gz", "zst", "bz2", "zip", "vhd", "ffu",
                ],
            )
            .pick_file()
        {
            self.on_image_picked(path.to_string_lossy().to_string(), lang);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build_plan(&mut self) -> Option<core::WritePlan> {
        let drive_path = self.selected_drive.clone()?;
        let info = match self.image_info.clone() {
            Some(i) => i,
            None => match flashcraft_native::inspect_image(&self.image_path) {
                Ok(i) => i,
                Err(e) => {
                    self.say(e.to_string());
                    self.status_err = true;
                    return None;
                }
            },
        };
        let dev = self.drives.iter().find(|d| d.path == drive_path)?.clone();
        match core::WritePlan::new(info, dev) {
            Ok(p) => Some(p),
            Err(e) => {
                self.say(e.to_string());
                self.status_err = true;
                None
            }
        }
    }

    /// The drive's *only* refusal is that partitions are mounted — the image
    /// side already validates. Rufus proceeds here and unmounts during the
    /// write; so do we (the unmount happens after the warning is confirmed).
    #[cfg(not(target_arch = "wasm32"))]
    fn blocked_only_by_mounts(&self) -> bool {
        let Some(path) = self.selected_drive.as_deref() else {
            return false;
        };
        let Some(dev) = self.drives.iter().find(|d| d.path == path) else {
            return false;
        };
        if self.boot_selection == BootSelection::NonBootable {
            return dev.removable
                && dev.size_bytes > 0
                && dev.writable() == Err("device has mounted partitions");
        }
        let Some(img) = &self.image_info else {
            return false;
        };
        let comp_ok = img
            .compression
            .as_deref()
            .map(core::is_supported_compression)
            .unwrap_or(true);
        dev.writable() == Err("device has mounted partitions")
            && img.size_bytes > 0
            && comp_ok
            && img.size_bytes <= dev.size_bytes
    }

    /// START pressed: validate inputs, then pop the destruction warning.
    #[cfg(not(target_arch = "wasm32"))]
    fn start_pressed(&mut self, lang: Resolved) {
        if self.job_rx.is_some() {
            return;
        }
        if self.selected_drive.is_none() {
            self.status_err = true;
            self.say(t("Please select a device", lang));
            return;
        }
        if self.boot_selection == BootSelection::NonBootable {
            let Some(path) = self.selected_drive.as_deref() else {
                return;
            };
            let Some(dev) = self.drives.iter().find(|d| d.path == path) else {
                return;
            };
            if dev.writable().is_ok() || self.blocked_only_by_mounts() {
                self.pending_confirm = Some(JobKind::Format);
            } else if let Err(reason) = dev.writable() {
                self.status_err = true;
                self.say(format!("{}: {}", dev.path, t(reason, lang)));
            }
            return;
        }
        if self.image_path.is_empty() {
            self.status_err = true;
            self.say(t("Please select an image (SELECT)", lang));
            return;
        }
        if self.blocked_only_by_mounts() || self.build_plan().is_some() {
            self.pending_confirm = Some(JobKind::Write);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_checksum_job(&mut self, ctx: &egui::Context, lang: Resolved) {
        if self.job_rx.is_some() {
            return;
        }
        if self.image_path.is_empty() {
            self.status_err = true;
            self.say(t("Please select an image (SELECT)", lang));
            return;
        }
        self.show_checksums = true;
        if self.checksums.is_some() {
            return;
        }
        let image_path = self.image_path.clone();
        let (tx, rx) = channel::<JobMsg>();
        let ctx = ctx.clone();
        self.status_err = false;
        self.progress = Some(core::Progress {
            phase: core::Phase::Hashing,
            ..Default::default()
        });
        self.job_started = Some(Instant::now());
        self.cancel_flag.store(false, Ordering::Relaxed);
        let cancel = self.cancel_flag.clone();
        self.job_rx = Some(rx);
        self.say(tf("Computing checksums {0}%", lang, "0"));
        thread::spawn(move || {
            let res = flashcraft_native::compute_checksums(&image_path, &mut |p| {
                if !cancel.load(Ordering::Relaxed) {
                    let _ = tx.send(JobMsg::Progress(p));
                    ctx.request_repaint();
                }
            });
            match res {
                Ok(sums) => {
                    let _ = tx.send(JobMsg::Checksums(sums));
                    let _ = tx.send(JobMsg::Done(Ok("READY")));
                }
                Err(e) => {
                    let _ = tx.send(JobMsg::Done(Err(e.to_string())));
                }
            }
            ctx.request_repaint();
        });
    }

    /// Warning accepted (or Verify clicked): run the job on a thread.
    #[cfg(not(target_arch = "wasm32"))]
    fn start_job(&mut self, kind: JobKind, ctx: &egui::Context, lang: Resolved) {
        let Some(drive_path) = self.selected_drive.clone() else {
            return;
        };
        let Some(dev) = self.drives.iter().find(|d| d.path == drive_path).cloned() else {
            return;
        };
        let image_path = self.image_path.clone();
        let volume_label = self.volume_label.clone();
        let scheme = self.partition_scheme.clone();
        let file_system = self.file_system.clone();
        let cluster_size = self.cluster_size.clone();
        let quick_format = self.quick_format;
        let extended_label = self.extended_label;
        let bad_blocks = if self.bad_blocks {
            (self.bad_block_passes as u8).saturating_add(1).min(4)
        } else {
            0
        };
        let is_dd = self.image_option.starts_with("DD");
        let persistence_mib = if is_dd { 0 } else { self.persistence_mib };
        let old_bios_fixes = self.old_bios_fixes;
        let verify_after = self.uefi_media_validation;

        let (tx, rx) = channel::<JobMsg>();
        let ctx = ctx.clone();
        let start_msg = match kind {
            JobKind::Verify => tf(
                "verify started",
                lang,
                &format!("{image_path} -> {drive_path}"),
            ),
            JobKind::Format => tf(
                "format started",
                lang,
                &format!("{drive_path} ({scheme}, {file_system})"),
            ),
            JobKind::Write => tf(
                "write started",
                lang,
                &format!("{image_path} -> {drive_path}"),
            ),
        };
        self.say(start_msg);
        self.status_err = false;
        self.progress = Some(core::Progress::default());
        self.job_started = Some(Instant::now());
        self.cancel_flag.store(false, Ordering::Relaxed);
        let cancel = self.cancel_flag.clone();
        self.job_rx = Some(rx);

        thread::spawn(move || {
            let txp = tx.clone();
            let result = if flashcraft_native::linux::effective_uid() != Some(0) {
                let _ = txp.send(JobMsg::Status("Requesting administrator rights"));
                ctx.request_repaint();
                run_elevated(
                    kind,
                    &image_path,
                    &drive_path,
                    &volume_label,
                    &scheme,
                    &file_system,
                    &cluster_size,
                    quick_format,
                    extended_label,
                    bad_blocks,
                    persistence_mib,
                    old_bios_fixes,
                    verify_after,
                    &cancel,
                    &txp,
                    &ctx,
                )
            } else {
                (|| {
                    let mut dev = dev;
                    if kind != JobKind::Verify && !dev.mounted.is_empty() {
                        let _ = txp.send(JobMsg::Status("Unmounting mounted partitions"));
                        ctx.request_repaint();
                        flashcraft_native::linux::unmount_partitions(&mut dev)?;
                    }
                    match kind {
                        JobKind::Format => {
                            let plan = core::FormatPlan::new(
                                dev,
                                core::PartitionScheme::parse(&scheme),
                                core::FileSystem::parse(&file_system),
                            )?
                            .with_label(Some(volume_label))
                            .with_cluster_bytes(core::parse_cluster_bytes(&cluster_size))
                            .with_quick_format(quick_format)
                            .with_extended_label(extended_label)
                            .with_bad_blocks(bad_blocks)
                            .with_old_bios_fixes(old_bios_fixes);
                            flashcraft_native::format_drive(&plan, &mut |p| {
                                let _ = txp.send(JobMsg::Progress(p));
                                ctx.request_repaint();
                            })
                        }
                        JobKind::Verify => {
                            let info = flashcraft_native::inspect_image(&image_path)?;
                            let plan = core::WritePlan::new(info, dev)?
                                .with_label(if is_dd { None } else { Some(volume_label) })
                                .with_old_bios_fixes(old_bios_fixes);
                            flashcraft_native::verify_image(&plan, &mut |p| {
                                let _ = txp.send(JobMsg::Progress(p));
                                ctx.request_repaint();
                            })
                        }
                        JobKind::Write => {
                            let info = flashcraft_native::inspect_image(&image_path)?;
                            let plan = core::WritePlan::new(info, dev)?
                                .with_label(if is_dd { None } else { Some(volume_label) })
                                .with_bad_blocks(bad_blocks)
                                .with_persistence_mib(persistence_mib)
                                .with_old_bios_fixes(old_bios_fixes)
                                .with_verify_after(verify_after)
                                .with_extended_label(extended_label);
                            flashcraft_native::write_image(&plan, &mut |p| {
                                let _ = txp.send(JobMsg::Progress(p));
                                ctx.request_repaint();
                            })
                        }
                    }
                })()
            };
            let done = match result {
                Ok(()) => Ok(match kind {
                    JobKind::Verify => "VERIFY OK",
                    JobKind::Format => "FORMAT DONE",
                    JobKind::Write => "WRITE DONE",
                }),
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(JobMsg::Done(done));
            ctx.request_repaint();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_job(&mut self, lang: Resolved) {
        let Some(rx) = &self.job_rx else {
            return;
        };
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        let mut done = None;
        for msg in msgs {
            match msg {
                JobMsg::Progress(p) => {
                    self.progress = Some(p);
                    let speed_suffix = if p.bytes_per_sec > 0 {
                        format!(" ({:.1} MB/s)", p.bytes_per_sec as f64 / 1_000_000.0)
                    } else {
                        String::new()
                    };
                    let base = match p.phase {
                        core::Phase::Syncing => t("Syncing to device…", lang).to_string(),
                        core::Phase::Verifying => {
                            tf("Verifying {0}%", lang, &p.percent().to_string())
                        }
                        core::Phase::Formatting => {
                            tf("Formatting {0}%", lang, &p.percent().to_string())
                        }
                        core::Phase::BadBlocks => {
                            tf("Scanning bad blocks {0}%", lang, &p.percent().to_string())
                        }
                        core::Phase::Hashing => {
                            tf("Computing checksums {0}%", lang, &p.percent().to_string())
                        }
                        core::Phase::Writing => tf("Writing {0}%", lang, &p.percent().to_string()),
                    };
                    self.status = format!("{base}{speed_suffix}");
                }
                JobMsg::Status(key) => self.say(t(key, lang).to_string()),
                JobMsg::Checksums(sums) => {
                    self.log.push(format!("MD5:     {}", sums.md5));
                    self.log.push(format!("SHA-1:   {}", sums.sha1));
                    self.log.push(format!("SHA-256: {}", sums.sha256));
                    self.checksums = Some(sums);
                }
                JobMsg::Done(result) => done = Some(result),
            }
        }
        if let Some(result) = done {
            self.job_rx = None;
            self.refresh_drives();
            match result {
                Ok("READY") => {
                    self.status_err = false;
                    self.status = "READY".to_string();
                }
                Ok(msg) => {
                    self.status_err = false;
                    self.say(format!("{} — {}", t(msg, lang), t("READY", lang)));
                }
                Err(e) => {
                    self.status_err = true;
                    self.say(e);
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_job(&mut self, _lang: Resolved) {}

    /// Draw a miniature Craft-style FlashCraft badge (rounded red-amber tile,
    /// warm-paper USB stick + golden lightning bolt) in code.
    fn paint_app_badge(painter: &egui::Painter, rect: egui::Rect) {
        let r = rect.width().min(rect.height());
        let tile = egui::Rect::from_center_size(rect.center(), egui::vec2(r, r));
        painter.rect_filled(
            tile,
            egui::CornerRadius::same((r * 0.22) as u8),
            Color32::from_rgb(0xd9, 0x38, 0x1e),
        );
        let plug = egui::Rect::from_center_size(
            tile.center() + egui::vec2(0.0, -r * 0.25),
            egui::vec2(r * 0.34, r * 0.22),
        );
        let body = egui::Rect::from_center_size(
            tile.center() + egui::vec2(0.0, r * 0.12),
            egui::vec2(r * 0.50, r * 0.54),
        );
        let cream = Color32::from_rgb(0xef, 0xe9, 0xdc);
        let ink = Color32::from_rgb(0x0b, 0x0b, 0x0c);
        let gold = Color32::from_rgb(0xf5, 0x9f, 0x00);
        painter.rect_filled(plug, egui::CornerRadius::same(2), cream);
        painter.rect_stroke(
            plug,
            egui::CornerRadius::same(2),
            egui::Stroke::new(1.4_f32, ink),
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(body, egui::CornerRadius::same(4), cream);
        painter.rect_stroke(
            body,
            egui::CornerRadius::same(4),
            egui::Stroke::new(1.5_f32, ink),
            egui::StrokeKind::Inside,
        );
        let c = body.center();
        let bolt = vec![
            c + egui::vec2(r * 0.05, -r * 0.19),
            c + egui::vec2(-r * 0.12, r * 0.02),
            c + egui::vec2(r * 0.01, r * 0.02),
            c + egui::vec2(-r * 0.05, r * 0.20),
            c + egui::vec2(r * 0.13, -r * 0.02),
            c + egui::vec2(r * 0.0, -r * 0.02),
        ];
        painter.add(egui::Shape::convex_polygon(
            bolt,
            gold,
            egui::Stroke::new(1.0_f32, ink),
        ));
    }

    /// Rufus-style section header: bold title followed by a thin horizontal line.
    fn section_header(ui: &mut egui::Ui, title: &str) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .strong()
                    .size(13.5)
                    .color(Color32::from_rgb(0xf0, 0xf0, 0xf0)),
            );
            let avail = ui.available_width();
            if avail > 8.0 {
                let (_, rect) = ui.allocate_space(egui::vec2(avail, 14.0));
                let y = rect.center().y + 1.0;
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x + 4.0, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(0x4a, 0x4a, 0x4a)),
                );
            }
        });
        ui.add_space(2.0);
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let lang = self.lang.resolve();
        self.poll_job(lang);

        #[cfg(not(target_arch = "wasm32"))]
        let busy = self.job_rx.is_some();
        #[cfg(target_arch = "wasm32")]
        let busy = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 5.0);

                // ── Top header strip: Craft badge + title + language & About ──
                ui.horizontal(|ui| {
                    let (badge_rect, _) =
                        ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    Self::paint_app_badge(ui.painter(), badge_rect);
                    ui.label(
                        RichText::new(format!("FlashCraft {}", env!("CARGO_PKG_VERSION")))
                            .strong()
                            .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("ℹ")
                            .on_hover_text(t("About", lang).as_ref())
                            .clicked()
                        {
                            self.show_about = !self.show_about;
                        }
                        if ui
                            .button(format!("🌐 {}", self.lang.label()))
                            .on_hover_text(t("Language", lang).as_ref())
                            .clicked()
                        {
                            self.settings_open = !self.settings_open;
                        }
                    });
                });

                if self.settings_open {
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(t("Language", lang).as_ref());
                                let current = self.lang;
                                egui::ComboBox::from_id_salt("lang")
                                    .width(200.0)
                                    .selected_text(current.label())
                                    .show_ui(ui, |ui| {
                                        for l in Lang::ALL {
                                            ui.selectable_value(&mut self.lang, *l, l.label());
                                        }
                                    });
                            });
                        });
                }

                // ── Drive Properties ─────────────────────────────────────────
                Self::section_header(ui, t("Drive Properties", lang).as_ref());

                ui.label(t("Device", lang).as_ref());
                let chosen = self
                    .drives
                    .iter()
                    .find(|d| Some(&d.path) == self.selected_drive.as_ref())
                    .cloned();
                let chosen_size_bytes = chosen.as_ref().map(|d| d.size_bytes);
                let chosen_text = chosen
                    .as_ref()
                    .map(|d| match d.writable() {
                        Ok(()) => Self::drive_label(d),
                        Err("device has mounted partitions") => format!(
                            "{}  ({})",
                            Self::drive_label(d),
                            t("mounted — will unmount automatically", lang)
                        ),
                        Err(reason) => format!("{}  ({})", Self::drive_label(d), t(reason, lang)),
                    })
                    .unwrap_or_else(|| t("no drive selected", lang).to_string());

                ui.horizontal(|ui| {
                    let refresh_w = 30.0;
                    let combo_w = (ui.available_width() - refresh_w - 8.0).max(120.0);
                    ui.add_enabled_ui(!busy, |ui| {
                        egui::ComboBox::from_id_salt("device")
                            .selected_text(chosen_text)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                for d in &self.drives {
                                    let label = match d.writable() {
                                        Ok(()) => Self::drive_label(d),
                                        Err("device has mounted partitions") => format!(
                                            "{}  ({})",
                                            Self::drive_label(d),
                                            t("mounted — will unmount automatically", lang)
                                        ),
                                        Err(reason) => format!(
                                            "{}  ({})",
                                            Self::drive_label(d),
                                            t(reason, lang)
                                        ),
                                    };
                                    ui.selectable_value(
                                        &mut self.selected_drive,
                                        Some(d.path.clone()),
                                        label,
                                    );
                                }
                            });
                    });
                    let refresh = ui.add_enabled(
                        !busy,
                        egui::Button::new("").min_size(egui::vec2(refresh_w, 22.0)),
                    );
                    {
                        let c = refresh.rect.center();
                        let rad = refresh.rect.height() * 0.28;
                        let color = ui.visuals().widgets.inactive.fg_stroke.color;
                        let stroke = egui::Stroke::new(1.4_f32, color);
                        let pts: Vec<egui::Pos2> = (0..=12)
                            .map(|i| {
                                let a = -1.0 + (i as f32 / 12.0) * 4.9;
                                c + egui::vec2(a.cos() * rad, a.sin() * rad)
                            })
                            .collect();
                        let tip = *pts.last().unwrap_or(&c);
                        let dir = pts[pts.len().saturating_sub(2)] - tip;
                        let n = egui::vec2(-dir.y, dir.x).normalized() * 3.0;
                        ui.painter().add(egui::Shape::line(pts, stroke));
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![tip + egui::vec2(dir.x * 1.2, dir.y * 1.2), tip + n, tip - n],
                            color,
                            egui::Stroke::NONE,
                        ));
                    }
                    if refresh.on_hover_text(t("Refresh", lang)).clicked() {
                        self.refresh_drives();
                        self.say(t("Device list refreshed", lang));
                    }
                });

                // Boot selection + [#] Checksums + SELECT
                ui.add_space(2.0);
                ui.label(t("Boot selection", lang).as_ref());
                let image_name = self
                    .image_path
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let boot_label = match self.boot_selection {
                    BootSelection::NonBootable => t("Non bootable", lang).to_string(),
                    BootSelection::Image => {
                        if image_name.is_empty() {
                            t("Disk or ISO image (Please select)", lang).to_string()
                        } else {
                            image_name.clone()
                        }
                    }
                };
                ui.horizontal(|ui| {
                    let select_w = 110.0;
                    let hash_w = 28.0;
                    let combo_w = (ui.available_width() - select_w - hash_w - 16.0).max(120.0);
                    ui.add_enabled_ui(!busy, |ui| {
                        egui::ComboBox::from_id_salt("boot")
                            .selected_text(boot_label)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                let img_item = if image_name.is_empty() {
                                    t("Disk or ISO image (Please select)", lang).to_string()
                                } else {
                                    format!(
                                        "{} ({})",
                                        image_name,
                                        t("Disk or ISO image (Please select)", lang)
                                    )
                                };
                                if ui
                                    .selectable_label(
                                        self.boot_selection == BootSelection::Image,
                                        img_item,
                                    )
                                    .clicked()
                                {
                                    self.boot_selection = BootSelection::Image;
                                }
                                if ui
                                    .selectable_label(
                                        self.boot_selection == BootSelection::NonBootable,
                                        t("Non bootable", lang).as_ref(),
                                    )
                                    .clicked()
                                {
                                    self.boot_selection = BootSelection::NonBootable;
                                    if self.volume_label.is_empty() {
                                        self.volume_label = "FLASHCRAFT".to_string();
                                    }
                                }
                            });
                    });
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let can_hash = !busy
                            && self.boot_selection == BootSelection::Image
                            && !self.image_path.is_empty();
                        if ui
                            .add_enabled(
                                can_hash,
                                egui::Button::new(RichText::new("#").strong())
                                    .min_size(egui::vec2(hash_w, 22.0)),
                            )
                            .on_hover_text(t("Checksums", lang).as_ref())
                            .clicked()
                        {
                            self.start_checksum_job(ctx, lang);
                        }
                        if ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(t("SELECT", lang).as_ref())
                                    .min_size(egui::vec2(select_w, 22.0)),
                            )
                            .clicked()
                        {
                            self.browse_image(lang);
                        }
                    }
                });

                // Image option + Persistent partition size (when in Image mode)
                let is_image_mode = self.boot_selection == BootSelection::Image;
                ui.add_space(2.0);
                ui.columns(2, |cols| {
                    if let Some(c0) = cols.first_mut() {
                        c0.label(t("Image option", lang).as_ref());
                        let w = c0.available_width();
                        c0.add_enabled_ui(!busy && is_image_mode, |ui| {
                            egui::ComboBox::from_id_salt("imgopt")
                                .selected_text(t(&self.image_option, lang).as_ref())
                                .width(w)
                                .show_ui(ui, |ui| {
                                    for &opt in IMAGE_OPTIONS {
                                        ui.selectable_value(
                                            &mut self.image_option,
                                            opt.to_string(),
                                            t(opt, lang).as_ref(),
                                        );
                                    }
                                });
                        });
                    }
                    if let Some(c1) = cols.get_mut(1) {
                        c1.label(t("Persistent partition size", lang).as_ref());
                        let can_persist =
                            !busy && is_image_mode && !self.image_option.starts_with("DD");
                        let max_mib: u64 = chosen_size_bytes
                            .map(|sz| (sz / 1_048_576).saturating_sub(4096))
                            .unwrap_or(0)
                            .min(32768);
                        c1.add_enabled_ui(can_persist && max_mib >= 256, |ui| {
                            let upper = max_mib.max(256);
                            ui.add(
                                egui::Slider::new(&mut self.persistence_mib, 0..=upper)
                                    .step_by(256.0)
                                    .suffix(" MB"),
                            );
                        });
                    }
                });

                // Partition scheme & Target system (two columns, linked like Rufus)
                ui.add_space(2.0);
                ui.columns(2, |cols| {
                    let mut scheme_changed = false;
                    let mut target_changed = false;
                    if let Some(c0) = cols.first_mut() {
                        c0.label(t("Partition scheme", lang).as_ref());
                        let w = c0.available_width();
                        c0.add_enabled_ui(!busy, |ui| {
                            egui::ComboBox::from_id_salt("scheme")
                                .selected_text(self.partition_scheme.as_str())
                                .width(w)
                                .show_ui(ui, |ui| {
                                    for &s in PARTITION_SCHEMES {
                                        if ui
                                            .selectable_value(
                                                &mut self.partition_scheme,
                                                s.to_string(),
                                                s,
                                            )
                                            .changed()
                                        {
                                            scheme_changed = true;
                                        }
                                    }
                                });
                        });
                    }
                    if let Some(c1) = cols.get_mut(1) {
                        c1.label(t("Target system", lang).as_ref());
                        let w = c1.available_width();
                        c1.add_enabled_ui(!busy, |ui| {
                            egui::ComboBox::from_id_salt("target")
                                .selected_text(t(&self.target_system, lang).as_ref())
                                .width(w)
                                .show_ui(ui, |ui| {
                                    for &ts in TARGET_SYSTEMS {
                                        if ui
                                            .selectable_value(
                                                &mut self.target_system,
                                                ts.to_string(),
                                                t(ts, lang).as_ref(),
                                            )
                                            .changed()
                                        {
                                            target_changed = true;
                                        }
                                    }
                                });
                        });
                    }
                    if scheme_changed {
                        self.target_system = if self.partition_scheme == "GPT" {
                            TARGET_SYSTEMS[1].to_string()
                        } else {
                            TARGET_SYSTEMS[0].to_string()
                        };
                    } else if target_changed {
                        self.partition_scheme = if self.target_system == TARGET_SYSTEMS[1] {
                            "GPT".to_string()
                        } else {
                            "MBR".to_string()
                        };
                    }
                });

                // Advanced drive properties toggle
                let adv_drive_label = if self.show_adv_drive {
                    format!("▲ {}", t("Hide advanced drive properties", lang))
                } else {
                    format!("▼ {}", t("Show advanced drive properties", lang))
                };
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(adv_drive_label)
                                .color(Color32::from_rgb(0x6c, 0xb8, 0xf6))
                                .size(12.0),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    self.show_adv_drive = !self.show_adv_drive;
                }
                if self.show_adv_drive {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.add_enabled_ui(!busy, |ui| {
                                if ui
                                    .checkbox(
                                        &mut self.list_usb_hard_drives,
                                        t("List USB Hard Drives", lang).as_ref(),
                                    )
                                    .changed()
                                {
                                    self.refresh_drives();
                                }
                                ui.checkbox(
                                    &mut self.old_bios_fixes,
                                    t(
                                        "Add fixes for old BIOSes (extra partition, align, etc.)",
                                        lang,
                                    )
                                    .as_ref(),
                                );
                                ui.checkbox(
                                    &mut self.uefi_media_validation,
                                    t("Enable runtime UEFI media validation", lang).as_ref(),
                                );
                            });
                        });
                }

                // ── Format Options ───────────────────────────────────────────
                Self::section_header(ui, t("Format Options", lang).as_ref());

                ui.label(t("Volume label", lang).as_ref());
                ui.add_enabled_ui(!busy, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.volume_label)
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.add_space(2.0);
                ui.columns(2, |cols| {
                    if let Some(c0) = cols.first_mut() {
                        c0.label(t("File system", lang).as_ref());
                        let w = c0.available_width();
                        c0.add_enabled_ui(!busy, |ui| {
                            egui::ComboBox::from_id_salt("fs")
                                .selected_text(t(&self.file_system, lang).as_ref())
                                .width(w)
                                .show_ui(ui, |ui| {
                                    for &fs in FILE_SYSTEMS {
                                        ui.selectable_value(
                                            &mut self.file_system,
                                            fs.to_string(),
                                            t(fs, lang).as_ref(),
                                        );
                                    }
                                });
                        });
                    }
                    if let Some(c1) = cols.get_mut(1) {
                        c1.label(t("Cluster size", lang).as_ref());
                        let w = c1.available_width();
                        c1.add_enabled_ui(!busy, |ui| {
                            egui::ComboBox::from_id_salt("cluster")
                                .selected_text(t(&self.cluster_size, lang).as_ref())
                                .width(w)
                                .show_ui(ui, |ui| {
                                    for &cs in CLUSTER_SIZES {
                                        ui.selectable_value(
                                            &mut self.cluster_size,
                                            cs.to_string(),
                                            t(cs, lang).as_ref(),
                                        );
                                    }
                                });
                        });
                    }
                });

                let adv_fmt_label = if self.show_adv_format {
                    format!("▲ {}", t("Hide advanced format options", lang))
                } else {
                    format!("▼ {}", t("Show advanced format options", lang))
                };
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(adv_fmt_label)
                                .color(Color32::from_rgb(0x6c, 0xb8, 0xf6))
                                .size(12.0),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    self.show_adv_format = !self.show_adv_format;
                }
                if self.show_adv_format {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.add_enabled_ui(!busy, |ui| {
                                ui.checkbox(
                                    &mut self.quick_format,
                                    t("Quick format", lang).as_ref(),
                                );
                                ui.checkbox(
                                    &mut self.extended_label,
                                    t("Create extended label and icon files", lang).as_ref(),
                                );
                                ui.horizontal(|ui| {
                                    ui.checkbox(
                                        &mut self.bad_blocks,
                                        t("Check device for bad blocks", lang).as_ref(),
                                    );
                                    let current_pass = BAD_BLOCK_PASSES
                                        .get(self.bad_block_passes)
                                        .copied()
                                        .unwrap_or(BAD_BLOCK_PASSES[0]);
                                    ui.add_enabled_ui(self.bad_blocks, |ui| {
                                        egui::ComboBox::from_id_salt("badblocks")
                                            .selected_text(t(current_pass, lang).as_ref())
                                            .width(120.0)
                                            .show_ui(ui, |ui| {
                                                for (idx, &p) in BAD_BLOCK_PASSES.iter().enumerate()
                                                {
                                                    ui.selectable_value(
                                                        &mut self.bad_block_passes,
                                                        idx,
                                                        t(p, lang).as_ref(),
                                                    );
                                                }
                                            });
                                    });
                                });
                            });
                        });
                }

                // ── Status & Progress Bar ────────────────────────────────────
                Self::section_header(ui, t("Status:", lang).trim_end_matches(':'));

                #[cfg(not(target_arch = "wasm32"))]
                let frac = self
                    .progress
                    .as_ref()
                    .map(|p| p.percent() as f32 / 100.0)
                    .unwrap_or(0.0);
                #[cfg(target_arch = "wasm32")]
                let frac = 0.0_f32;

                let shown_status = if self.status == "READY" {
                    t("READY", lang).to_string()
                } else {
                    self.status.clone()
                };

                // Rufus progress bar: full width, green fill, centered status text inside!
                let bar_w = ui.available_width();
                let (bar, _) =
                    ui.allocate_exact_size(egui::vec2(bar_w, 24.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(bar, 0.0, ui.visuals().extreme_bg_color);
                if frac > 0.0 {
                    let w = (bar.width() - 2.0) * frac.clamp(0.0, 1.0);
                    let fill_col = if self.status_err {
                        Color32::from_rgb(0x9e, 0x2a, 0x2b)
                    } else {
                        Color32::from_rgb(0x06, 0x96, 0x20)
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            bar.min + egui::vec2(1.0, 1.0),
                            egui::vec2(w, bar.height() - 2.0),
                        ),
                        0.0,
                        fill_col,
                    );
                }
                ui.painter().rect_stroke(
                    bar,
                    0.0,
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(0x55, 0x55, 0x55)),
                    egui::StrokeKind::Inside,
                );
                let text_col = if self.status_err {
                    Color32::from_rgb(0xff, 0x85, 0x85)
                } else {
                    Color32::WHITE
                };
                ui.painter().text(
                    bar.center(),
                    egui::Align2::CENTER_CENTER,
                    &shown_status,
                    egui::FontId::proportional(12.5),
                    text_col,
                );

                if let Some(err) = &self.drives_error {
                    ui.label(
                        RichText::new(err)
                            .color(Color32::from_rgb(0xe0, 0xb0, 0x30))
                            .small(),
                    );
                }

                // ── Bottom button bar ────────────────────────────────────────
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(t("Log", lang).as_ref())
                                .min_size(egui::vec2(68.0, 26.0)),
                        )
                        .clicked()
                    {
                        self.show_log = !self.show_log;
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(started) = self.job_started {
                        if busy {
                            let secs = started.elapsed().as_secs();
                            let elapsed = format!(
                                "{:02}:{:02}:{:02}",
                                secs / 3600,
                                (secs / 60) % 60,
                                secs % 60
                            );
                            ui.label(RichText::new(elapsed).monospace().weak());
                            ctx.request_repaint_after(std::time::Duration::from_millis(250));
                        }
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if busy {
                            if ui
                                .add(
                                    egui::Button::new(t("Cancel", lang).as_ref())
                                        .min_size(egui::vec2(86.0, 26.0)),
                                )
                                .clicked()
                            {
                                self.cancel_flag.store(true, Ordering::Relaxed);
                                self.say(t("Cancelling operation…", lang));
                            }
                        } else if ui
                            .add(
                                egui::Button::new(t("Close", lang).as_ref())
                                    .min_size(egui::vec2(86.0, 26.0)),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        let can_verify = !busy
                            && self.boot_selection == BootSelection::Image
                            && !self.image_path.is_empty()
                            && self.selected_drive.is_some();
                        if ui
                            .add_enabled(
                                can_verify,
                                egui::Button::new(t("Verify", lang).as_ref())
                                    .min_size(egui::vec2(86.0, 26.0)),
                            )
                            .clicked()
                        {
                            self.start_job(JobKind::Verify, ctx, lang);
                        }

                        if ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(
                                    RichText::new(t("START", lang).as_ref()).strong(),
                                )
                                .min_size(egui::vec2(96.0, 26.0)),
                            )
                            .clicked()
                        {
                            self.start_pressed(lang);
                        }
                    });
                });
            });

        // ── Checksums window (`#` button) ────────────────────────────────────
        #[cfg(not(target_arch = "wasm32"))]
        if self.show_checksums {
            let mut open = self.show_checksums;
            egui::Window::new(t("Checksums", lang).as_ref())
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([460.0, 170.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new(&self.image_path).small().weak());
                    ui.separator();
                    if let Some(sums) = &self.checksums.clone() {
                        egui::Grid::new("checksums_grid")
                            .num_columns(3)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                for (name, val) in [
                                    ("MD5:", sums.md5.as_str()),
                                    ("SHA-1:", sums.sha1.as_str()),
                                    ("SHA-256:", sums.sha256.as_str()),
                                ] {
                                    ui.label(RichText::new(name).strong());
                                    let mut buf = val.to_string();
                                    ui.add(
                                        egui::TextEdit::singleline(&mut buf)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(310.0),
                                    );
                                    if ui.small_button(t("Copy", lang).as_ref()).clicked() {
                                        ctx.copy_text(val.to_string());
                                    }
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label(t("Computing checksums {0}%", lang).replace("{0}%", "…"));
                    }
                });
            self.show_checksums = open;
        }

        // ── About window ─────────────────────────────────────────────────────
        if self.show_about {
            let mut open = self.show_about;
            egui::Window::new(t("About", lang).as_ref())
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_size([380.0, 200.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                        Self::paint_app_badge(ui.painter(), r);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("FlashCraft v{}", env!("CARGO_PKG_VERSION")))
                                    .strong()
                                    .size(16.0),
                            );
                            ui.label("Clean-room Rust bootable USB & drive utility");
                        });
                    });
                    ui.separator();
                    ui.label(format!("{}: GPL-3.0-only", t("License", lang)));
                    ui.label("UI translations adapted from Rufus (GPL-3.0).");
                    ui.add_space(6.0);
                    if ui.button(t("Close", lang).as_ref()).clicked() {
                        self.show_about = false;
                    }
                });
            if !open {
                self.show_about = false;
            }
        }

        // ── Log (a separate window, like Rufus's IDD_LOG) ────────────────────
        if self.show_log {
            egui::Window::new(t("Log", lang).as_ref())
                .collapsible(false)
                .resizable(true)
                .default_size([440.0, 240.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .max_height(190.0)
                        .show(ui, |ui| {
                            for line in &self.log {
                                ui.label(RichText::new(line).small().monospace());
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(t("Copy", lang).as_ref()).clicked() {
                            ctx.copy_text(self.log.join("\n"));
                        }
                        if ui.button(t("Clear", lang).as_ref()).clicked() {
                            self.log.clear();
                        }
                        if ui.button(t("Close", lang).as_ref()).clicked() {
                            self.show_log = false;
                        }
                    });
                });
        }

        // ── Rufus-style destruction warning ──────────────────────────────────
        if let Some(_kind) = self.pending_confirm {
            egui::Window::new("FlashCraft")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let dev = self.selected_drive.clone().unwrap_or_default();
                    ui.label(
                        RichText::new(tf(
                            "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED",
                            lang,
                            &dev,
                        ))
                        .strong()
                        .color(Color32::from_rgb(0xff, 0x85, 0x85)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        t(
                            "To continue, click OK. To abort the operation, click Cancel.",
                            lang,
                        )
                        .as_ref(),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("OK").strong())
                                    .min_size(egui::vec2(80.0, 24.0)),
                            )
                            .clicked()
                        {
                            self.pending_confirm = None;
                            #[cfg(not(target_arch = "wasm32"))]
                            self.start_job(_kind, ctx, lang);
                        }
                        if ui
                            .add(
                                egui::Button::new(t("Cancel", lang).as_ref())
                                    .min_size(egui::vec2(80.0, 24.0)),
                            )
                            .clicked()
                        {
                            self.pending_confirm = None;
                            self.say(t("aborted by user", lang));
                            self.status_err = false;
                        }
                    });
                });
        }
    }
}

/// Dark Windows-dialog look shared by the desktop and web shells: charcoal
/// face, light text, dark fields, mid-gray buttons, #0078D7 accents, square
/// corners everywhere.
pub fn win32_visuals() -> egui::Style {
    let mut style = egui::Style::default();
    let mut v = egui::Visuals::dark();
    let face = egui::Color32::from_rgb(0x25, 0x26, 0x2a);
    let text = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0xec, 0xec, 0xee));
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x55, 0x57, 0x60));
    v.panel_fill = face;
    v.window_fill = face;
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4a, 0x4c, 0x52));
    v.extreme_bg_color = egui::Color32::from_rgb(0x18, 0x19, 0x1c);
    v.faint_bg_color = egui::Color32::from_rgb(0x1e, 0x1f, 0x23);
    v.hyperlink_color = egui::Color32::from_rgb(0x6c, 0xb8, 0xf6);
    v.widgets.noninteractive.weak_bg_fill = face;
    v.widgets.noninteractive.bg_fill = face;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.fg_stroke = text;
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(0x36, 0x38, 0x3f);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x18, 0x19, 0x1c);
    v.widgets.inactive.bg_stroke = border;
    v.widgets.inactive.fg_stroke = text;
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0x44, 0x46, 0x4f);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x24, 0x25, 0x2a);
    v.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(0x00, 0x78, 0xd7));
    v.widgets.hovered.fg_stroke = text;
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(0x2e, 0x30, 0x36);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(0x14, 0x15, 0x18);
    v.widgets.active.bg_stroke =
        egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(0x00, 0x54, 0x99));
    v.widgets.active.fg_stroke = text;
    v.widgets.open = v.widgets.hovered;
    v.selection.bg_fill = egui::Color32::from_rgb(0x00, 0x78, 0xd7);
    v.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(2);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
    v.widgets.active.corner_radius = egui::CornerRadius::same(2);
    v.widgets.open.corner_radius = egui::CornerRadius::same(2);
    v.window_corner_radius = egui::CornerRadius::same(3);
    v.menu_corner_radius = egui::CornerRadius::same(2);
    style.visuals = v;
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(12.5));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(12.5));
    style
}
