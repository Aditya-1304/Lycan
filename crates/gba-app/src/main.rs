#![forbid(unsafe_code)]

mod app;
mod audio;

use app::GbaApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    if std::env::args().any(|argument| argument == "--audio-probe") {
        return audio::probe()
            .map_err(|error| eframe::Error::AppCreation(Box::new(std::io::Error::other(error))));
    }
    // Native platforms use eframe's Glow renderer and the configured winit backend.
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "gba-rs",
        native_options,
        Box::new(|cc| Ok(Box::new(GbaApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    // Mount the same shared app implementation on the canvas owned by Trunk's page.
    wasm_bindgen_futures::spawn_local(async move {
        let document = eframe::web_sys::window()
            .expect("browser window unavailable")
            .document()
            .expect("browser document unavailable");

        let canvas = document
            .get_element_by_id("gba_canvas")
            .expect("missing #gba_canvas")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("#gba_canvas is not a canvas");

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(GbaApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
