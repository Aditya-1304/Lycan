//! Browser-only gesture hit targets. The DOM adapter executes activation-gated
//! operations and reports actual state; this module never owns a fullscreen flag.
use eframe::egui;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = gbaPresentation, js_name = configure)]
    fn configure(gain: f32, notify: &js_sys::Function);
    #[wasm_bindgen(js_namespace = gbaPresentation, js_name = beginFrame)]
    pub fn begin_frame();
    #[wasm_bindgen(js_namespace = gbaPresentation, js_name = fullscreen)]
    pub fn fullscreen() -> bool;
    #[wasm_bindgen(js_namespace = gbaPresentation, js_name = error)]
    pub fn error() -> String;
    #[wasm_bindgen(js_namespace = gbaPresentation, js_name = region)]
    fn region(name: &str, left: f32, top: f32, right: f32, bottom: f32, focused: bool);
}

/// Publishes a visible egui control in normalized canvas coordinates. Updating a
/// target is bounded presentation work; activation happens only in a DOM event.
pub fn register(name: &str, response: &egui::Response) {
    let viewport = response.ctx.input(|input| input.viewport_rect());
    if viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return;
    }
    let rect = response.rect.intersect(response.ctx.content_rect());
    region(
        name,
        (rect.left() - viewport.left()) / viewport.width(),
        (rect.top() - viewport.top()) / viewport.height(),
        (rect.right() - viewport.left()) / viewport.width(),
        (rect.bottom() - viewport.top()) / viewport.height(),
        response.has_focus(),
    );
}

/// One page-owned callback requests an event-driven repaint on asynchronous audio
/// completion or DOM fullscreen changes. Idle/paused screens need no polling loop.
pub fn initialize(ctx: &egui::Context, gain: f32) {
    let ctx = ctx.clone();
    let callback = Closure::<dyn FnMut()>::new(move || ctx.request_repaint());
    let value = callback.into_js_value();
    configure(gain, value.unchecked_ref());
}
