#![forbid(unsafe_code)]

mod app;
mod audio;
mod saves;

use app::GbaApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let arguments: Vec<_> = std::env::args().collect();
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--save-probe")
    {
        return saves::probe(arguments.get(index + 1).map_or("", String::as_str))
            .map_err(|error| eframe::Error::AppCreation(Box::new(std::io::Error::other(error))));
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--audio-probe")
    {
        return audio::probe(arguments.get(index + 1).map_or("pcm", String::as_str))
            .map_err(|error| eframe::Error::AppCreation(Box::new(std::io::Error::other(error))));
    }
    let debug_ui = arguments.iter().any(|argument| argument == "--debug-ui");

    // Native platforms use eframe's Glow renderer and the configured winit backend.
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        centered: true,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Lycan")
            .with_inner_size([760.0, 540.0])
            .with_min_inner_size([360.0, 280.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Lycan",
        native_options,
        Box::new(move |cc| Ok(Box::new(GbaApp::new(cc, debug_ui)))),
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
                Box::new(|cc| Ok(Box::new(GbaApp::new(cc, false)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
