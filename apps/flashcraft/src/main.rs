//! FlashCraft desktop app: an eframe shell over `flashcraft-ui-egui`.
//! The shell owns only window state; all behaviour lives in the crates.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use eframe::egui;
use flashcraft_ui_egui::{win32_visuals, FlashcraftUi};

struct App {
    ui: FlashcraftUi,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx);
    }
}

/// Liberation Sans (Arial metrics, close to Segoe UI) as the UI font, plus
/// Noto CJK as fallback so the CJK catalogs render. Any missing file or
/// unparsable font is skipped — the egui built-ins remain the fallback.
fn load_fonts(ctx: &egui::Context) {
    let fonts_to_try = [
        (
            "LiberationSans",
            "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
        ),
        (
            "NotoSansCJK",
            "/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc",
        ),
    ];
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = Vec::new();
    for (name, path) in fonts_to_try {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                name.to_string(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            loaded.push(name);
        }
    }
    if loaded.is_empty() {
        return;
    }
    let family = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    for name in loaded.into_iter().rev() {
        family.insert(0, name.to_string());
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 700.0])
            .with_resizable(false),
        ..Default::default()
    };
    let mut ui = FlashcraftUi::default();
    ui.refresh_drives();
    eframe::run_native(
        "FlashCraft",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_style(win32_visuals());
            load_fonts(&cc.egui_ctx);
            Ok(Box::new(App { ui }))
        }),
    )
}
