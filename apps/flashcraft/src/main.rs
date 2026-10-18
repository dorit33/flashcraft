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
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 280.0]),
        ..Default::default()
    };
    let mut ui = FlashcraftUi::default();
    ui.refresh_drives();
    eframe::run_native(
        "FlashCraft",
        options,
        Box::new(move |_cc| Ok(Box::new(App { ui }))),
    )
}
