//! FlashCraft web app: an eframe shell over `flashcraft-ui-egui`, compiled to
//! WebAssembly. Drive enumeration and writing are desktop-only; the web build
//! shows the same dialog for inspection and localization.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

#[cfg(target_arch = "wasm32")]
struct App {
    ui: flashcraft_ui_egui::FlashcraftUi,
}

#[cfg(target_arch = "wasm32")]
impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    let web_options = eframe::WebOptions::default();
    let mut ui = flashcraft_ui_egui::FlashcraftUi::default();
    ui.refresh_drives();
    wasm_bindgen_futures::spawn_local(async move {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("flashcraft_canvas"))
            .and_then(|e| {
                use wasm_bindgen::JsCast;
                e.dyn_into::<web_sys::HtmlCanvasElement>().ok()
            });
        let Some(canvas) = canvas else {
            log::error!("canvas element 'flashcraft_canvas' not found");
            return;
        };
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| {
                    cc.egui_ctx.set_style(flashcraft_ui_egui::win32_visuals());
                    Ok(Box::new(App { ui }) as Box<dyn eframe::App>)
                }),
            )
            .await;
        if let Err(e) = result {
            log::error!("failed to start: {e:?}");
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "flashcraft-web is a WebAssembly build; run it via `trunk serve` or packaging/web/web.sh"
    );
}
