//! Host output consumes prepared PCM. Session lifecycle boundaries reset both
//! interpolation history and the adapter queue without changing guest devices.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
use browser::Output;
use gba_session::{Resampler, Session};
#[cfg(not(target_arch = "wasm32"))]
use native::Output;

/// Frontend-owned playback state and reusable sample staging for one session.
pub struct Audio {
    output: Option<Output>,
    resampler: Option<[Resampler; 2]>,
    rate: u32,
    core: Vec<[f32; 2]>,
    converted: Vec<[f32; 2]>,
    channels: [Vec<f32>; 2],
    resampled: [Vec<f32>; 2],
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
            channels: std::array::from_fn(|_| Vec::with_capacity(4096)),
            resampled: std::array::from_fn(|_| Vec::with_capacity(16384)),
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
            for channel in resampler {
                channel.reset();
            }
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
        let rate = output.rate();
        if rate == 0 {
            return;
        }
        if rate != self.rate {
            self.rate = rate;
            self.resampler = Some(std::array::from_fn(|_| Resampler::new(rate)));
            output.clear();
        }
        output.set_gain(if self.muted { 0.0 } else { self.volume });
        if !self.playing {
            return;
        }
        self.converted.clear();
        if let Some(resampler) = &mut self.resampler {
            // Both converters retain identical phase while interpolating each
            // speaker independently; routing is never collapsed to mono.
            for (index, converter) in resampler.iter_mut().enumerate() {
                self.channels[index].clear();
                self.channels[index].extend(self.core.iter().map(|frame| frame[index]));
                self.resampled[index].clear();
                converter.process(&self.channels[index], &mut self.resampled[index]);
            }
            self.converted.extend(
                self.resampled[0]
                    .iter()
                    .zip(&self.resampled[1])
                    .map(|(&l, &r)| [l, r]),
            );
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
pub fn probe() -> Result<(), String> {
    use gba_session::Button;
    use std::time::{Duration, Instant};
    let rom = include_bytes!("../../../roms/pcm.gba");
    let mut session = Session::new();
    session.load_rom(rom).map_err(|e| e.to_string())?;
    session.enable_test_firmware();
    let mut audio = Audio::default();
    audio.start();
    if let Some(error) = &audio.error {
        return Err(error.clone());
    }
    let origin = Instant::now();
    let mut phase = 0;
    while origin.elapsed() < Duration::from_secs(6) {
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
                _ => {}
            }
            println!("AUDIO phase={phase} {}", audio.status());
        }
        session.set_button(Button::A, phase % 2 == 1);
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
