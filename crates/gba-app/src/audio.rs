//! Host output consumes prepared PCM. Session lifecycle boundaries reset both
//! interpolation history and the adapter queue without changing guest devices.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
use browser::Output;
use std::num::NonZeroU32;

use gba_session::{Session, StereoResampler};
#[cfg(not(target_arch = "wasm32"))]
use native::Output;

/// Frontend-owned playback state and reusable sample staging for one session.
pub struct Audio {
    output: Option<Output>,
    resampler: Option<StereoResampler>,
    rate: u32,
    core: Vec<[f32; 2]>,
    converted: Vec<[f32; 2]>,
    playing: bool,
    pub volume: f32,
    pub muted: bool,
    error: Option<String>,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            output: None,
            resampler: None,
            rate: 0,
            core: Vec::with_capacity(4096),
            converted: Vec::with_capacity(16384),
            playing: false,
            volume: 0.5,
            muted: false,
            error: None,
        }
    }
}

impl Audio {
    /// Invoked by an explicit frontend gesture on both platforms.
    pub fn start(&mut self) {
        if self.output.is_none() {
            match Output::start() {
                Ok(output) => {
                    self.output = Some(output);
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
        } else {
            #[cfg(target_arch = "wasm32")]
            let _ = Output::start();
        }
        self.clear();
    }

    /// Invalidates prepared host audio and resets interpolation history.
    pub fn clear(&mut self) {
        if let Some(output) = &mut self.output {
            output.clear();
        }
        if let Some(resampler) = &mut self.resampler {
            resampler.reset();
        }
        self.core.clear();
        self.converted.clear();
    }

    pub fn set_playing(&mut self, playing: bool) {
        if self.playing != playing {
            self.playing = playing;
            self.clear();
        }
        if let Some(output) = &mut self.output {
            output.set_playing(playing);
        }
    }

    /// Drains every callback, even before audio is enabled, so startup cannot
    /// replay historical sound or accumulate a core-side staging backlog.
    pub fn pump(&mut self, session: &mut Session) {
        self.core.clear();
        session.drain_stereo_pcm(&mut self.core);
        let Some(output) = &mut self.output else {
            return;
        };
        let Some(output_rate) = NonZeroU32::new(output.rate()) else {
            return;
        };
        let rate = output_rate.get();
        if rate != self.rate {
            self.rate = rate;
            self.resampler = Some(StereoResampler::new(output_rate));
            output.clear();
        }
        output.set_gain(if self.muted { 0.0 } else { self.volume });
        if !self.playing {
            return;
        }
        self.converted.clear();
        if let Some(resampler) = &mut self.resampler {
            resampler.process(&self.core, &mut self.converted);
        }
        output.submit(&self.converted);
    }

    /// Formats negotiated rate, bounded queue state and adapter error counters.
    pub fn status(&self) -> String {
        if let Some(error) = &self.error {
            return format!("Audio unavailable: {error}");
        }
        self.output.as_ref().map_or_else(
            || "Audio off; click Enable audio".to_owned(),
            Output::status,
        )
    }
}

/// CLI-only device probe uses the real guest/session/resampler/output path.
/// It exercises lifecycle discontinuities without opening a native window.
#[cfg(not(target_arch = "wasm32"))]
pub fn probe(scene: &str) -> Result<(), String> {
    use gba_session::Button;
    use std::time::{Duration, Instant};
    let rom: &[u8] = match scene {
        "pcm" => include_bytes!("../../../fixtures/diagnostics/pcm.gba"),
        "noise" => include_bytes!("../../../fixtures/diagnostics/noise.gba"),
        _ => return Err("Audio probe scene must be pcm or noise".to_owned()),
    };
    let mut session = Session::new();
    session.load_rom(rom).map_err(|e| e.to_string())?;
    session.enable_test_firmware();
    let mut audio = Audio::default();
    audio.start();
    if let Some(error) = audio.error.take() {
        return Err(error);
    }
    let origin = Instant::now();
    let mut phase = 0;
    // The noise scene retains the existing lifecycle probes, then sustains
    // mixed playback long enough to expose refill or host-buffer rate drift.
    let duration = if scene == "noise" { 30 } else { 6 };
    while origin.elapsed() < Duration::from_secs(duration) {
        let now = origin.elapsed();
        let next = now.as_millis() / 1000;
        if next != phase {
            phase = next;
            match phase {
                2 => session.toggle_pause(),
                3 => {
                    session.toggle_pause();
                    audio.clear();
                }
                4 => {
                    session.reset();
                    audio.clear();
                }
                5 => {
                    session.set_active(false);
                    audio.set_playing(false);
                }
                6 if scene == "noise" => {
                    session.set_active(true);
                    audio.clear();
                }
                _ => {}
            }
            println!("AUDIO phase={phase} {}", audio.status());
        }
        session.set_button(Button::A, phase % 2 == 1);
        session.set_button(Button::B, scene == "noise" && phase % 4 >= 2);
        session.set_button(Button::Left, phase == 1);
        session.set_button(Button::Right, phase == 3);
        audio.set_playing(phase != 5 && !session.paused());
        session.advance_host_time(now).map_err(|e| e.to_string())?;
        audio.pump(&mut session);
        std::thread::sleep(Duration::from_millis(2));
    }
    session.set_active(true);
    audio.set_playing(true);
    let resume = origin.elapsed();
    while origin.elapsed() - resume < Duration::from_secs(2) {
        let now = origin.elapsed();
        session.set_button(Button::A, true);
        // Exercise guest mixer writes through the production playback path,
        // including master silence, PWM bias, gain and deliberate FIFO resets.
        let mixer_phase = (now - resume).as_millis() / 400;
        session.set_button(Button::Down, mixer_phase == 0);
        session.set_button(Button::Up, mixer_phase == 1);
        session.set_button(Button::Select, mixer_phase == 2);
        session.set_button(Button::B, mixer_phase == 3);
        session.advance_host_time(now).map_err(|e| e.to_string())?;
        audio.pump(&mut session);
        std::thread::sleep(Duration::from_millis(2));
    }
    println!("AUDIO resumed {}", audio.status());
    println!("AUDIO core_counters={:?}", session.pcm_counters());
    audio.set_playing(false);
    Ok(())
}
