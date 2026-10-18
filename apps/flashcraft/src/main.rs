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
            // Rufus-like chrome: hard square corners, dark panel.
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.widgets.open.corner_radius = egui::CornerRadius::ZERO;
            style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
            style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
            cc.egui_ctx.set_style(style);
            Ok(Box::new(App { ui }))
        }),
    )
}
