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
use flashcraft_ui_egui::FlashcraftUi;

struct App {
    ui: FlashcraftUi,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx);
    }
}

/// Classic Windows-dialog look, like Rufus: #F0F0F0 face, black text, white
/// fields, #E1E1E1 buttons, #0078D7 accents, square corners everywhere.
fn win32_visuals() -> egui::Style {
    let mut style = egui::Style::default();
    let mut v = egui::Visuals::light();
    let face = egui::Color32::from_rgb(0xf0, 0xf0, 0xf0);
    let black = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0xad, 0xad, 0xad));
    v.panel_fill = face;
    v.window_fill = face;
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x76, 0x76, 0x76));
    v.extreme_bg_color = egui::Color32::WHITE;
    v.faint_bg_color = egui::Color32::WHITE;
    v.hyperlink_color = egui::Color32::from_rgb(0x00, 0x66, 0xcc);
    v.widgets.noninteractive.weak_bg_fill = face;
    v.widgets.noninteractive.bg_fill = face;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.fg_stroke = black;
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(0xe1, 0xe1, 0xe1);
    v.widgets.inactive.bg_fill = egui::Color32::WHITE;
    v.widgets.inactive.bg_stroke = border;
    v.widgets.inactive.fg_stroke = black;
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0xe5, 0xf1, 0xfb);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0xe5, 0xf1, 0xfb);
    v.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x00, 0x78, 0xd7));
    v.widgets.hovered.fg_stroke = black;
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(0xcc, 0xe4, 0xf7);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(0xcc, 0xe4, 0xf7);
    v.widgets.active.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x00, 0x54, 0x99));
    v.widgets.active.fg_stroke = black;
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
    style
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
            .with_inner_size([480.0, 560.0])
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
