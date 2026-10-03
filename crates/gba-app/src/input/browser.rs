//! Page input ownership and lifecycle observation. JavaScript owns DOM listeners;
//! the app retains all emulated input and execution/pacing decisions.
use eframe::egui::Key;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = gbaInput, js_name = lifecycleRevision)]
    pub fn lifecycle_revision() -> u32;
    #[wasm_bindgen(js_namespace = gbaInput, js_name = setGameplay)]
    pub fn set_gameplay(gameplay: bool);
    #[wasm_bindgen(js_namespace = gbaInput, js_name = setBindings)]
    fn store_bindings(keys: &js_sys::Array);
}

/// Mapping conversion occurs only on preference load or completed binding edits.
/// The page guard uses logical key names, matching egui's existing web integration.
pub fn set_bindings(keys: &[Key; 10]) {
    let names = js_sys::Array::new();
    for key in keys {
        names.push(&JsValue::from_str(key.name()));
    }
    store_bindings(&names);
}
