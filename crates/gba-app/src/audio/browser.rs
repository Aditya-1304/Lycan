//! Browser audio bridge. JavaScript copies samples into ordinary owned buffers;
//! it never transfers or detaches WASM linear memory.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = rate)]
    fn audio_rate() -> u32;
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = submit)]
    fn submit_audio(samples: &[f32]);
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = clear)]
    fn clear_audio();
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = setPlaying)]
    fn playing_audio(playing: bool);
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = setGain)]
    fn gain_audio(gain: f32);
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = status)]
    fn status_audio() -> String;
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = state)]
    fn state_audio() -> String;
    #[wasm_bindgen(js_namespace = gbaAudio, js_name = failure)]
    fn failure_audio() -> String;
}

/// Handles the page-owned AudioContext while leaving platform objects in JavaScript.
pub struct Output;
impl Output {
    pub fn start(gain: f32) -> Result<Self, String> {
        gain_audio(gain);
        Ok(Self)
    }
    pub fn state(&self) -> String {
        state_audio()
    }
    pub fn failure(&self) -> Option<String> {
        let error = failure_audio();
        (!error.is_empty()).then_some(error)
    }
    pub fn rate(&self) -> u32 {
        audio_rate()
    }
    pub fn submit(&mut self, samples: &[[f32; 2]]) {
        submit_audio(samples.as_flattened());
    }
    pub fn clear(&mut self) {
        clear_audio();
    }
    pub fn set_playing(&mut self, playing: bool) {
        playing_audio(playing);
    }
    pub fn set_gain(&mut self, gain: f32) {
        gain_audio(gain);
    }
    pub fn status(&self) -> String {
        status_audio()
    }
}
