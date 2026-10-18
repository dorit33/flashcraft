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
    // Format Options
    image_option: String,
    partition_scheme: String,
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
            image_option: "Standard installation".to_string(),
            partition_scheme: "MBR".to_string(),
            volume_label: String::new(),
            file_system: FILE_SYSTEMS[0].to_string(),
            cluster_size: CLUSTER_SIZES[0].to_string(),
            show_adv_drive: false,
            list_usb_hard_drives: false,
            old_bios_fixes: false,
            uefi_media_validation: false,
            show_adv_format: false,
            quick_format: true,
            extended_label: false,
            bad_blocks: false,
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

            // ── Rufus IDD_DIALOG, rendered 1:1 ───────────────────
            // From res/rufus.rc: DIALOGEX 12,12,232,326. Dialog units are
            // scaled to pixels; x positions mirror for RTL languages.
            const DU_X: f32 = 2.3;
            const DU_Y: f32 = 2.0;
            const DU_W: f32 = 232.0;
            let rtl = lang.right_to_left();
            let origin = ui.cursor().min + egui::vec2(0.0, 6.0);
            let ctl = |x: f32, y: f32, w: f32, h: f32| -> egui::Rect {
                let mx = if rtl { DU_W - x - w } else { x };
                egui::Rect::from_min_size(
                    origin + egui::vec2(mx * DU_X, y * DU_Y),
                    egui::vec2(w * DU_X, (h * DU_Y).max(15.0)),
                )
            };
            let label = |ui: &mut egui::Ui, x: f32, y: f32, s: &str, strong: bool| {
                // Labels left-align within their (edge-clamped) rect, like LTEXT
                let rt = RichText::new(s);
                let rt = if strong { rt.strong() } else { rt };
                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(ctl(x, y, DU_W - x, 10.0))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.add(egui::Label::new(rt).truncate());
                    },
                );
            };
            let text = |ui: &mut egui::Ui, x: f32, y: f32, s: &str| {
                label(ui, x, y, s, false);
            };
            let bold = |ui: &mut egui::Ui, x: f32, y: f32, s: &str| {
                label(ui, x, y, s, true);
            };
            let btn = |ui: &mut egui::Ui,
                       x: f32,
                       y: f32,
                       w: f32,
                       s: &str,
                       enabled: bool|
             -> egui::Response {
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(ctl(x, y, w, 12.0)), |ui| {
                    ui.add_enabled(
                        enabled,
                        egui::Button::new(s)
                            .min_size(egui::vec2(w * DU_X, 12.0 * DU_Y))
                            .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                })
                .inner
            };
            let check = |ui: &mut egui::Ui,
                         x: f32,
                         y: f32,
                         on: &mut bool,
                         s: &str,
                         enabled: bool|
             -> egui::Response {
                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(ctl(x, y, 216.0, 10.0)),
                    |ui| ui.add_enabled(enabled, egui::Checkbox::new(on, s)),
                )
                .inner
            };
            let combo = |ui: &mut egui::Ui,
                         x: f32,
                         y: f32,
                         w: f32,
                         id: &str,
                         sel: &mut String,
                         items: &[&str],
                         enabled: bool| {
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(ctl(x, y, w, 12.0)), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        egui::ComboBox::from_id_salt(id)
                            .width(w * DU_X - 16.0)
                            .selected_text(sel.as_str())
                            .show_ui(ui, |ui| {
                                for it in items {
                                    ui.selectable_value(sel, (*it).to_string(), *it);
                                }
                            });
                    });
                });
            };

            // Drive Properties
            bold(ui, 8.0, 6.0, t("Drive Properties", lang).as_ref());
            text(ui, 8.0, 21.0, t("Device", lang).as_ref());
            let chosen = self
                .drives
                .iter()
                .find(|d| Some(&d.path) == self.selected_drive.as_ref());
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(ctl(8.0, 30.0, 196.0, 12.0)),
                |ui| {
                    egui::ComboBox::from_id_salt("device")
                        .selected_text(chosen.map(Self::drive_label).unwrap_or_default())
                        .width(196.0 * DU_X - 16.0)
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
                },
            );
            let refresh = btn(ui, 210.0, 30.0, 14.0, "", true);
            // Circular-arrow icon drawn by hand — no font glyph needed.
            {
                let c = refresh.rect.center();
                let rad = refresh.rect.height() * 0.30;
                let color = ui.visuals().widgets.inactive.fg_stroke.color;
                let stroke = egui::Stroke::new(1.3_f32, color);
                let pts: Vec<egui::Pos2> = (0..=12)
                    .map(|i| {
                        let a = -1.0 + (i as f32 / 12.0) * 4.9; // ~280° arc
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
            text(ui, 8.0, 43.0, t("Boot selection", lang).as_ref());
            let image_name = self
                .image_path
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string();
            let mut sel = if image_name.is_empty() {
                t("Disk or ISO image (Please select)", lang).to_string()
            } else {
                image_name
            };
            combo(
                ui,
                8.0,
                52.0,
                148.0,
                "boot",
                &mut sel,
                &["Disk or ISO image (Please select)"],
                true,
            );
            #[cfg(not(target_arch = "wasm32"))]
            if btn(ui, 180.0, 52.0, 45.0, t("SELECT", lang).as_ref(), true).clicked() {
                self.browse_image(lang);
            }
            text(ui, 8.0, 65.0, t("Image option", lang).as_ref());
            combo(
                ui,
                8.0,
                74.0,
                96.0,
                "imgopt",
                &mut self.image_option,
                &["Standard installation"],
                true,
            );
            // Persistent partition controls (IDC_PERSISTENCE_*) stay hidden —
            // Rufus only shows them for ISOs that support persistence.
            text(ui, 8.0, 87.0, t("Partition scheme", lang).as_ref());
            text(ui, 128.0, 87.0, t("Target system", lang).as_ref());
            combo(
                ui,
                8.0,
                96.0,
                96.0,
                "scheme",
                &mut self.partition_scheme,
                PARTITION_SCHEMES,
                true,
            );
            let mut target = if self.partition_scheme == "GPT" {
                t("UEFI (non CSM)", lang).to_string()
            } else {
                t("BIOS or UEFI", lang).to_string()
            };
            combo(
                ui,
                128.0,
                96.0,
                96.0,
                "target",
                &mut target,
                &["BIOS or UEFI", "UEFI (non CSM)"],
                false,
            );
            check(
                ui,
                8.0,
                109.0,
                &mut self.show_adv_drive,
                t("Show advanced drive properties", lang).as_ref(),
                true,
            );
            if self.show_adv_drive {
                if check(
                    ui,
                    8.0,
                    119.0,
                    &mut self.list_usb_hard_drives,
                    t("List USB Hard Drives", lang).as_ref(),
                    true,
                )
                .clicked()
                {
                    self.refresh_drives();
                }
                check(
                    ui,
                    8.0,
                    129.0,
                    &mut self.old_bios_fixes,
                    t(
                        "Add fixes for old BIOSes (extra partition, align, etc.)",
                        lang,
                    )
                    .as_ref(),
                    false,
                );
                check(
                    ui,
                    8.0,
                    139.0,
                    &mut self.uefi_media_validation,
                    t("Enable runtime UEFI media validation", lang).as_ref(),
                    false,
                );
            }

            // Format Options
            bold(ui, 8.0, 152.0, t("Format Options", lang).as_ref());
            text(ui, 8.0, 167.0, t("Volume label", lang).as_ref());
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(ctl(8.0, 176.0, 216.0, 12.0)),
                |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.volume_label)
                            .desired_width(216.0 * DU_X),
                    );
                },
            );
            text(ui, 8.0, 189.0, t("File system", lang).as_ref());
            text(ui, 128.0, 189.0, t("Cluster size", lang).as_ref());
            combo(
                ui,
                8.0,
                198.0,
                96.0,
                "fs",
                &mut self.file_system,
                FILE_SYSTEMS,
                false,
            );
            combo(
                ui,
                128.0,
                198.0,
                96.0,
                "cluster",
                &mut self.cluster_size,
                CLUSTER_SIZES,
                false,
            );
            check(
                ui,
                8.0,
                211.0,
                &mut self.show_adv_format,
                t("Show advanced format options", lang).as_ref(),
                true,
            );
            if self.show_adv_format {
                check(
                    ui,
                    8.0,
                    221.0,
                    &mut self.quick_format,
                    t("Quick format", lang).as_ref(),
                    false,
                );
                check(
                    ui,
                    8.0,
                    231.0,
                    &mut self.extended_label,
                    t("Create extended label and icon files", lang).as_ref(),
                    false,
                );
                check(
                    ui,
                    8.0,
                    241.0,
                    &mut self.bad_blocks,
                    t("Check device for bad blocks", lang).as_ref(),
                    false,
                );
                let mut passes = BAD_BLOCK_PASSES[self.bad_block_passes].to_string();
                combo(
                    ui,
                    128.0,
                    241.0,
                    96.0,
                    "badblocks",
                    &mut passes,
                    BAD_BLOCK_PASSES,
                    false,
                );
                if let Some(i) = BAD_BLOCK_PASSES.iter().position(|p| *p == passes) {
                    self.bad_block_passes = i;
                }
            }

            // Status
            text(ui, 8.0, 254.0, t("Status:", lang).as_ref());
            let shown_status = if self.status == "READY" {
                t("READY", lang).to_string()
            } else {
                self.status.clone()
            };
            let status_text = if self.status_err {
                RichText::new(shown_status).color(Color32::from_rgb(0xff, 0x70, 0x70))
            } else {
                RichText::new(shown_status)
            };
            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(ctl(34.0, 254.0, DU_W - 34.0, 10.0))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.add(egui::Label::new(status_text).truncate());
                },
            );
            if let Some(err) = &self.drives_error {
                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(ctl(34.0, 262.0, DU_W - 34.0, 8.0))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(err)
                                    .color(Color32::from_rgb(0xe0, 0xb0, 0x30))
                                    .small(),
                            )
                            .truncate(),
                        );
                    },
                );
            }

            // Progress — always visible like Rufus, empty when idle
            #[cfg(not(target_arch = "wasm32"))]
            let frac = self
                .progress
                .as_ref()
                .map(|p| p.percent() as f32 / 100.0)
                .unwrap_or(0.0);
            #[cfg(target_arch = "wasm32")]
            let frac = 0.0_f32;
            // Progress bar — painted like the Win32 control: white field,
            // 1px border, solid green fill (nothing when idle).
            {
                let bar = ctl(8.0, 273.0, 216.0, 9.0);
                ui.painter()
                    .rect_filled(bar, 0.0, ui.visuals().extreme_bg_color);
                ui.painter().rect_stroke(
                    bar,
                    0.0,
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(0x55, 0x55, 0x55)),
                    egui::StrokeKind::Inside,
                );
                if frac > 0.0 {
                    let w = (bar.width() - 2.0) * frac.clamp(0.0, 1.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            bar.min + egui::vec2(1.0, 1.0),
                            egui::vec2(w, bar.height() - 2.0),
                        ),
                        0.0,
                        Color32::from_rgb(0x06, 0xb0, 0x25),
                    );
                }
            }

            // Bottom row — Log left; START + CLOSE right, per the .rc
            if btn(ui, 8.0, 294.0, 50.0, t("Log", lang).as_ref(), true).clicked() {
                self.show_log = !self.show_log;
            }
            ui.put(
                ctl(62.0, 294.0, 60.0, 10.0),
                egui::Label::new(
                    RichText::new(format!("FlashCraft {}", env!("CARGO_PKG_VERSION")))
                        .weak()
                        .small(),
                ),
            );
            #[cfg(not(target_arch = "wasm32"))]
            {
                if btn(
                    ui,
                    124.0,
                    294.0,
                    45.0,
                    t("START", lang).as_ref(),
                    self.job_rx.is_none(),
                )
                .clicked()
                {
                    self.start_pressed(lang);
                }
                if btn(ui, 178.0, 294.0, 45.0, t("Close", lang).as_ref(), true).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });

        // ── Log (a separate window, like Rufus's IDD_LOG) ───────
        if self.show_log {
            egui::Window::new(t("Log", lang).as_ref())
                .collapsible(false)
                .resizable(true)
                .default_size([380.0, 220.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.log {
                                ui.label(RichText::new(line).small().monospace());
                            }
                        });
                    if ui.button(t("Close", lang).as_ref()).clicked() {
                        self.show_log = false;
                    }
                });
        }

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

/// Dark Windows-dialog look shared by the desktop and web shells: charcoal
/// face, light text, dark fields, mid-gray buttons, #0078D7 accents, square
/// corners everywhere.
pub fn win32_visuals() -> egui::Style {
    let mut style = egui::Style::default();
    let mut v = egui::Visuals::dark();
    let face = egui::Color32::from_rgb(0x2b, 0x2b, 0x2b);
    let text = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0xe8, 0xe8, 0xe8));
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x55, 0x55, 0x55));
    v.panel_fill = face;
    v.window_fill = face;
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x4a, 0x4a, 0x4a));
    v.extreme_bg_color = egui::Color32::from_rgb(0x1b, 0x1b, 0x1b);
    v.faint_bg_color = egui::Color32::from_rgb(0x1b, 0x1b, 0x1b);
    v.hyperlink_color = egui::Color32::from_rgb(0x6c, 0xb8, 0xf6);
    v.widgets.noninteractive.weak_bg_fill = face;
    v.widgets.noninteractive.bg_fill = face;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.fg_stroke = text;
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(0x3c, 0x3c, 0x3c);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x1b, 0x1b, 0x1b);
    v.widgets.inactive.bg_stroke = border;
    v.widgets.inactive.fg_stroke = text;
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0x46, 0x46, 0x46);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x24, 0x24, 0x24);
    v.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x00, 0x78, 0xd7));
    v.widgets.hovered.fg_stroke = text;
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(0x33, 0x33, 0x33);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(0x14, 0x14, 0x14);
    v.widgets.active.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x00, 0x54, 0x99));
    v.widgets.active.fg_stroke = text;
    v.widgets.open = v.widgets.hovered;
    v.selection.bg_fill = egui::Color32::from_rgb(0x00, 0x78, 0xd7);
    v.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.open.corner_radius = egui::CornerRadius::ZERO;
    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    style.visuals = v;
    style.spacing.button_padding = egui::vec2(4.0, 1.0);
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(11.5));
    style
}
