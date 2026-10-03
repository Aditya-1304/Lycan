//! Shared CLI errors, repository paths, digests, image capture, and argument values.

use gba_core::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gba_session::Button;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub(crate) fn fail(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::other(message.into()).into()
}
pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn capture(path: &Path, framebuffer: &[u16]) -> Result<()> {
    let mut output = format!("P6\n{SCREEN_WIDTH} {SCREEN_HEIGHT}\n255\n").into_bytes();
    for &pixel in framebuffer {
        for shift in [0, 5, 10] {
            let value = ((pixel >> shift) & 31) as u8;
            output.push((value << 3) | (value >> 2));
        }
    }
    fs::write(path, output)?;
    Ok(())
}

/// Retains the first file error so callback-based verification can return it afterward.
pub(crate) fn record_capture_error(
    error: &mut Option<Box<dyn Error>>,
    path: PathBuf,
    framebuffer: &[u16],
) {
    if error.is_some() {
        return;
    }

    if let Err(cause) = capture(&path, framebuffer) {
        *error = Some(cause);
    }
}

/// Resolves fixture input names to the same logical buttons used by the app.
pub(crate) fn button(name: &str) -> Result<Button> {
    match name {
        "A" => Ok(Button::A),
        "B" => Ok(Button::B),
        "Select" => Ok(Button::Select),
        "Start" => Ok(Button::Start),
        "Right" => Ok(Button::Right),
        "Left" => Ok(Button::Left),
        "Up" => Ok(Button::Up),
        "Down" => Ok(Button::Down),
        "R" => Ok(Button::R),
        "L" => Ok(Button::L),
        _ => Err(fail(format!("unknown logical button: {name}"))),
    }
}

pub(crate) fn option(args: &[String], key: &str) -> Result<Option<String>> {
    match args.iter().position(|arg| arg == key) {
        Some(index) => args
            .get(index + 1)
            .cloned()
            .map(Some)
            .ok_or_else(|| fail(format!("missing value for {key}"))),
        None => Ok(None),
    }
}
