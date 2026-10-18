//! FlashCraft shared egui UI — laid out to match Rufus: Drive Properties,
//! Format Options, status bar, START. Thin by rule: view state and command
//! dispatch only; device I/O goes through `flashcraft-native` (desktop only).
//! Every user-facing string goes through `i18n`.

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
use std::sync::mpsc::{channel, Receiver};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use egui::{Color32, RichText};
use i18n::{t, tf, Lang, Resolved};

use flashcraft_core as core;

const SECTION: Color32 = Color32::from_rgb(0xe0, 0x9a, 0x3c);

const FILE_SYSTEMS: &[&str] = &["FAT32 (default)", "NTFS", "exFAT", "UDF", "ext2", "ext3"];
const CLUSTER_SIZES: &[&str] = &[
    "4096 bytes (default)",
    "8192 bytes",
    "16 kilobytes",
    "32 kilobytes",
    "64 kilobytes",
];
const PARTITION_SCHEMES: &[&str] = &["MBR", "GPT"];
const BAD_BLOCK_PASSES: &[&str] = &["1 Pass", "2 Passes", "3 Passes", "4 Passes"];

/// Message from the background write/verify job to the UI.
#[cfg(not(target_arch = "wasm32"))]
enum JobMsg {
    Progress(core::Progress),
    Done(Result<&'static str, String>),
}

/// FlashCraft's view state — mirrors the Rufus dialog, field for field.
pub struct FlashcraftUi {
    // Settings
    settings_open: bool,
    lang: Lang,
    // Drive Properties
    drives: Vec<core::DriveInfo>,
    selected_drive: Option<String>,
    image_path: String,
    #[cfg(not(target_arch = "wasm32"))]
    image_info: Option<core::ImageInfo>,
    persistent_partition: f32,
    // Format Options
    image_option: String,
    partition_scheme: String,
    volume_label: String,
    file_system: String,
    cluster_size: String,
    // Advanced sections
    show_adv_drive: bool,
    list_usb_hard_drives: bool,
    show_adv_format: bool,
    quick_format: bool,
    extended_label: bool,
    bad_block_passes: usize,
    // Run state
    status: String,
    status_err: bool,
    #[cfg(not(target_arch = "wasm32"))]
    progress: Option<core::Progress>,
    log: Vec<String>,
    show_log: bool,
    confirm_destroy: bool,
    drives_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    job_rx: Option<Receiver<JobMsg>>,
}

impl Default for FlashcraftUi {
    fn default() -> Self {
        Self {
            settings_open: false,
            lang: Lang::Auto,
            drives: Vec::new(),
            selected_drive: None,
            image_path: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            image_info: None,
            persistent_partition: 0.0,
            image_option: "Standard installation".to_string(),
            partition_scheme: "MBR".to_string(),
            volume_label: String::new(),
            file_system: FILE_SYSTEMS[0].to_string(),
            cluster_size: CLUSTER_SIZES[0].to_string(),
            show_adv_drive: false,
            list_usb_hard_drives: false,
            show_adv_format: false,
            quick_format: true,
            extended_label: false,
            bad_block_passes: 0,
            status: "READY".to_string(),
            status_err: false,
            #[cfg(not(target_arch = "wasm32"))]
            progress: None,
            log: Vec::new(),
            show_log: false,
            confirm_destroy: false,
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
            "{} [{}] {}",
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
            self.volume_label = stem.chars().take(11).collect();
        }
        self.image_path = path;
        match flashcraft_native::inspect_image(&self.image_path) {
            Ok(info) => {
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
                &["iso", "img", "bin", "xz", "gz", "zip", "vhd", "ffu"],
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

    /// START pressed: build the plan, then pop the destruction warning.
    #[cfg(not(target_arch = "wasm32"))]
    fn start_pressed(&mut self, lang: Resolved) {
        if self.job_rx.is_some() {
            return;
        }
        if self.image_path.is_empty() {
            self.say(t("Please select an image (SELECT)", lang));
            return;
        }
        if self.selected_drive.is_none() {
            self.say(t("Please select a device", lang));
            return;
        }
        if self.build_plan().is_some() {
            self.confirm_destroy = true;
        }
    }

    /// Warning accepted: run the job on a thread, report over a channel.
    #[cfg(not(target_arch = "wasm32"))]
    fn start_job(&mut self, verify_only: bool, ctx: &egui::Context, lang: Resolved) {
        let Some(plan) = self.build_plan() else {
            return;
        };
        let (tx, rx) = channel::<JobMsg>();
        let ctx = ctx.clone();
        self.say(tf(
            if verify_only {
                "verify started"
            } else {
                "write started"
            },
            lang,
            &format!("{} -> {}", plan.image.path, plan.drive.path),
        ));
        self.status_err = false;
        self.progress = Some(core::Progress::default());
        self.job_rx = Some(rx);
        thread::spawn(move || {
            let txp = tx.clone();
            let result = if verify_only {
                flashcraft_native::verify_image(&plan, &mut |p| {
                    let _ = txp.send(JobMsg::Progress(p));
                    ctx.request_repaint();
                })
            } else {
                flashcraft_native::write_image(&plan, &mut |p| {
                    let _ = txp.send(JobMsg::Progress(p));
                    ctx.request_repaint();
                })
            };
            let done = match result {
                Ok(()) => Ok(if verify_only {
                    "VERIFY OK"
                } else {
                    "WRITE DONE"
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
        let mut done = None;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                JobMsg::Progress(p) => self.progress = Some(p),
                JobMsg::Done(result) => done = Some(result),
            }
        }
        if let Some(result) = done {
            self.job_rx = None;
            match result {
                Ok(msg) => {
                    self.status_err = false;
                    self.say(format!("{msg} — {}", t("READY", lang)));
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

    fn section(ui: &mut egui::Ui, title: &str) {
        ui.add_space(4.0);
        ui.label(RichText::new(title).color(SECTION).strong().size(13.0));
    }

    fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.add_sized([170.0, 18.0], egui::Label::new(label));
            add(ui);
        });
    }

    fn combo(ui: &mut egui::Ui, id: &str, value: &mut String, options: &[&str], enabled: bool) {
        ui.add_enabled_ui(enabled, |ui| {
            egui::ComboBox::from_id_salt(id)
                .selected_text(value.as_str())
                .width(240.0)
                .show_ui(ui, |ui| {
                    for opt in options {
                        ui.selectable_value(value, (*opt).to_string(), *opt);
                    }
                });
        });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let lang = self.lang.resolve();
        self.poll_job(lang);

        egui::CentralPanel::default().show(ctx, |ui| {
            // ── Settings strip (language) ────────────────────────
            ui.horizontal(|ui| {
                if ui.small_button(t("Settings", lang).as_ref()).clicked() {
                    self.settings_open = !self.settings_open;
                }
                if self.settings_open {
                    ui.label(t("Language", lang).as_ref());
                    let current = self.lang;
                    egui::ComboBox::from_id_salt("lang")
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.lang, *l, l.label());
                            }
                        });
                }
            });

            // ── Drive Properties ────────────────────────────────
            Self::section(ui, &t("Drive Properties", lang));
            Self::row(ui, &t("Device", lang), |ui| {
                let chosen = self
                    .drives
                    .iter()
                    .find(|d| Some(&d.path) == self.selected_drive.as_ref());
                egui::ComboBox::from_id_salt("device")
                    .selected_text(chosen.map(Self::drive_label).unwrap_or_default())
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for d in &self.drives {
                            let label = if let Err(reason) = d.writable() {
                                format!("{}  ({reason})", Self::drive_label(d))
                            } else {
                                Self::drive_label(d)
                            };
                            ui.selectable_value(
                                &mut self.selected_drive,
                                Some(d.path.clone()),
                                label,
                            );
                        }
                    });
                if ui
                    .small_button("⟳")
                    .on_hover_text(t("Refresh", lang))
                    .clicked()
                {
                    self.refresh_drives();
                    self.say(t("Device list refreshed", lang));
                }
            });
            Self::row(ui, &t("Boot selection", lang), |ui| {
                let mut sel = t("Disk or ISO image (Please select)", lang).to_string();
                Self::combo(
                    ui,
                    "boot",
                    &mut sel,
                    &["Disk or ISO image (Please select)"],
                    true,
                );
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button(t("SELECT", lang).as_ref()).clicked() {
                    self.browse_image(lang);
                }
            });
            if !self.image_path.is_empty() {
                Self::row(ui, "", |ui| {
                    ui.label(RichText::new(&self.image_path).weak());
                });
            }
            Self::row(ui, &t("Persistent partition size", lang), |ui| {
                ui.add_enabled(
                    false,
                    egui::Slider::new(&mut self.persistent_partition, 0.0..=64.0).suffix(" GB"),
                );
            });

            // ── Format Options ──────────────────────────────────
            Self::section(ui, &t("Format Options", lang));
            Self::row(ui, &t("Image option", lang), |ui| {
                Self::combo(
                    ui,
                    "imgopt",
                    &mut self.image_option,
                    &["Standard installation"],
                    true,
                );
            });
            Self::row(ui, &t("Partition scheme", lang), |ui| {
                Self::combo(
                    ui,
                    "scheme",
                    &mut self.partition_scheme,
                    PARTITION_SCHEMES,
                    true,
                );
            });
            Self::row(ui, &t("Target system", lang), |ui| {
                let target = if self.partition_scheme == "GPT" {
                    t("UEFI (non CSM)", lang).to_string()
                } else {
                    t("BIOS or UEFI", lang).to_string()
                };
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut target.clone()).desired_width(240.0),
                );
            });
            Self::row(ui, &t("Volume label", lang), |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.volume_label).desired_width(240.0));
            });
            Self::row(ui, &t("File system", lang), |ui| {
                Self::combo(ui, "fs", &mut self.file_system, FILE_SYSTEMS, false);
            });
            Self::row(ui, &t("Cluster size", lang), |ui| {
                Self::combo(ui, "cluster", &mut self.cluster_size, CLUSTER_SIZES, false);
            });

            // ── Advanced ────────────────────────────────────────
            let adv_drive_key = if self.show_adv_drive {
                "Hide advanced drive properties"
            } else {
                "Show advanced drive properties"
            };
            if ui.small_button(t(adv_drive_key, lang).as_ref()).clicked() {
                self.show_adv_drive = !self.show_adv_drive;
            }
            if self.show_adv_drive
                && ui
                    .checkbox(
                        &mut self.list_usb_hard_drives,
                        t("List USB Hard Drives", lang).as_ref(),
                    )
                    .clicked()
            {
                self.refresh_drives();
            }
            let adv_fmt_key = if self.show_adv_format {
                "Hide advanced format options"
            } else {
                "Show advanced format options"
            };
            if ui.small_button(t(adv_fmt_key, lang).as_ref()).clicked() {
                self.show_adv_format = !self.show_adv_format;
            }
            if self.show_adv_format {
                ui.checkbox(&mut self.quick_format, t("Quick format", lang).as_ref());
                ui.checkbox(
                    &mut self.extended_label,
                    t("Create extended label and icon files", lang).as_ref(),
                );
                ui.horizontal(|ui| {
                    ui.label(t("Check device for bad blocks", lang).as_ref());
                    let mut passes = BAD_BLOCK_PASSES[self.bad_block_passes].to_string();
                    Self::combo(ui, "badblocks", &mut passes, BAD_BLOCK_PASSES, false);
                    if let Some(i) = BAD_BLOCK_PASSES.iter().position(|p| *p == passes) {
                        self.bad_block_passes = i;
                    }
                });
            }

            // ── Status bar ──────────────────────────────────────
            ui.add_space(6.0);
            ui.separator();
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(p) = &self.progress {
                if self.job_rx.is_some() {
                    ui.add(
                        egui::ProgressBar::new(p.percent() as f32 / 100.0)
                            .fill(Color32::from_rgb(0x4c, 0xaf, 0x50))
                            .show_percentage(),
                    );
                }
            }
            let shown_status = if self.status == "READY" {
                t("READY", lang).to_string()
            } else {
                self.status.clone()
            };
            let status_text = if self.status_err {
                RichText::new(shown_status).color(Color32::LIGHT_RED)
            } else {
                RichText::new(shown_status)
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("Status:", lang).as_ref()).weak());
                ui.label(status_text);
            });
            if let Some(err) = &self.drives_error {
                ui.colored_label(Color32::YELLOW, err);
            }

            // ── Bottom bar ──────────────────────────────────────
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button(t("Log", lang).as_ref()).clicked() {
                        self.show_log = !self.show_log;
                    }
                    ui.label(RichText::new("FlashCraft 0.1.0").weak().small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if ui
                                .add_enabled(
                                    self.job_rx.is_none(),
                                    egui::Button::new(t("START", lang).as_ref())
                                        .min_size(egui::vec2(90.0, 26.0)),
                                )
                                .clicked()
                            {
                                self.start_pressed(lang);
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        let _ = ui;
                    });
                });
                if self.show_log {
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.log {
                                ui.label(RichText::new(line).small().monospace());
                            }
                        });
                }
            });
        });

        // ── Rufus-style destruction warning ─────────────────────
        if self.confirm_destroy {
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
                        .strong(),
                    );
                    ui.label(
                        t(
                            "To continue, click OK. To abort the operation, click Cancel.",
                            lang,
                        )
                        .as_ref(),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(t("OK", lang).as_ref()).clicked() {
                            self.confirm_destroy = false;
                            #[cfg(not(target_arch = "wasm32"))]
                            self.start_job(false, ctx, lang);
                        }
                        if ui.button(t("Cancel", lang).as_ref()).clicked() {
                            self.confirm_destroy = false;
                            self.say(t("aborted by user", lang));
                            self.status_err = false;
                        }
                    });
                });
        }
    }
}
