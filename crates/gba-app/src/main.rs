#![forbid(unsafe_code)]

mod app;
mod audio;
mod input;
mod saves;
mod settings;

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
/// Writes startup failures to the browser console for detached async startup.
fn report_web_error(message: &str) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(message));
}

#[cfg(target_arch = "wasm32")]
/// Resolves the Trunk canvas and returns WebRunner startup failures to the caller.
async fn start_web() -> Result<(), String> {
    use wasm_bindgen::JsCast as _;

    let window = web_sys::window().ok_or_else(|| "browser window unavailable".to_owned())?;

    let document = window
        .document()
        .ok_or_else(|| "browser document unavailable".to_owned())?;

    let element = document
        .get_element_by_id("gba_canvas")
        .ok_or_else(|| "missing #gba_canvas".to_owned())?;

    let canvas = element
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "#gba_canvas is not a canvas".to_owned())?;

    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(GbaApp::new(cc, false)))),
        )
        .await
        .map_err(|error| format!("failed to start eframe: {error:?}"))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = start_web().await {
            report_web_error(&error);
        }
    });
}
