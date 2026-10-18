//! FlashCraft shared egui UI. Thin by rule: it renders engine state and
//! dispatches command ids, it does no device I/O itself except through
//! `flashcraft-native` (desktop only; wasm builds get the layout without
//! backends).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use flashcraft_core as core;

/// What the user picked; the shell decides what to do with it.
pub struct FlashcraftUi {
    image_path: String,
    selected_drive: Option<String>,
    status: String,
    progress: Option<core::Progress>,
    confirm_write: bool,
    drives: Vec<core::DriveInfo>,
    drives_error: Option<String>,
}

impl Default for FlashcraftUi {
    fn default() -> Self {
        Self {
            image_path: String::new(),
            selected_drive: None,
            status: "Pick an image and a removable drive.".to_string(),
            progress: None,
            confirm_write: false,
            drives: Vec::new(),
            drives_error: None,
        }
    }
}

impl FlashcraftUi {
    /// Desktop: enumerate real drives through the native backend.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn refresh_drives(&mut self) {
        match flashcraft_native::linux::list_drives_live() {
            Ok(drives) => {
                self.drives = drives.into_iter().filter(|d| d.removable).collect();
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
        self.drives_error = Some("device access is not available in the browser".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn browse_image(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("images", &["iso", "img", "bin", "xz", "gz", "zip"])
            .pick_file()
        {
            self.image_path = path.to_string_lossy().to_string();
        }
    }

    fn gib(bytes: u64) -> String {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_write(&mut self, verify_only: bool) {
        let image = self.image_path.clone();
        let drive = match self.selected_drive.clone() {
            Some(d) => d,
            None => {
                self.status = "no drive selected".to_string();
                return;
            }
        };
        let info = match flashcraft_native::inspect_image(&image) {
            Ok(i) => i,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        let dev = match self.drives.iter().find(|d| d.path == drive) {
            Some(d) => d.clone(),
            None => {
                self.status = format!("no such device: {drive}");
                return;
            }
        };
        let plan = match core::WritePlan::new(info, dev) {
            Ok(p) => p,
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        let ui_progress = std::cell::Cell::new(core::Progress::default());
        let mut cb = |p: core::Progress| ui_progress.set(p);
        let result = if verify_only {
            flashcraft_native::verify_image(&plan, &mut cb)
        } else {
            flashcraft_native::write_image(&plan, &mut cb)
        };
        self.progress = Some(ui_progress.get());
        self.status = match result {
            Ok(()) => {
                self.confirm_write = false;
                if verify_only {
                    "verify OK".to_string()
                } else {
                    "write finished; run verify to check".to_string()
                }
            }
            Err(e) => e.to_string(),
        };
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("FlashCraft");
            ui.label("Write a bootable image to a removable drive.");

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Image:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.image_path)
                        .desired_width(360.0)
                        .hint_text("path/to/image.iso"),
                );
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Browse…").clicked() {
                    self.browse_image();
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Drive:");
                egui::ComboBox::from_id_salt("drive")
                    .selected_text(
                        self.selected_drive
                            .clone()
                            .unwrap_or_else(|| "select…".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for d in &self.drives {
                            let label = format!(
                                "{}  {}  {}",
                                d.path,
                                Self::gib(d.size_bytes),
                                d.model.clone().unwrap_or_default()
                            );
                            ui.selectable_value(
                                &mut self.selected_drive,
                                Some(d.path.clone()),
                                label,
                            );
                        }
                    });
                if ui.button("Refresh").clicked() {
                    self.refresh_drives();
                }
            });
            if let Some(err) = &self.drives_error {
                ui.colored_label(egui::Color32::YELLOW, err);
            }

            // Show why the selected drive may refuse.
            if let Some(sel) = &self.selected_drive {
                if let Some(d) = self.drives.iter().find(|d| &d.path == sel) {
                    if let Err(reason) = d.writable() {
                        ui.colored_label(egui::Color32::YELLOW, format!("{sel}: {reason}"));
                    }
                }
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let ready = !self.image_path.is_empty() && self.selected_drive.is_some();
                ui.add_enabled_ui(ready, |ui| {
                    ui.checkbox(
                        &mut self.confirm_write,
                        "I understand the drive will be erased",
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let write_btn =
                            ui.add_enabled(self.confirm_write, egui::Button::new("Write image"));
                        if write_btn.clicked() {
                            self.run_write(false);
                        }
                        if ui.button("Verify").clicked() {
                            self.run_write(true);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    let _ = ui.add_enabled(false, egui::Button::new("Write image"));
                });
            });

            if let Some(p) = &self.progress {
                ui.add(egui::ProgressBar::new(p.percent() as f32 / 100.0).show_percentage());
            }
            ui.add_space(8.0);
            ui.separator();
            ui.label(&self.status);
        });
    }
}
