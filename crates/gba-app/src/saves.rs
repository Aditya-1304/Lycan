//! Platform storage boundary. The app dispatches at most one operation at a time;
//! every completion carries ROM identity, session generation and snapshot revision.

use gba_session::{EEPROM8K_BYTES, EEPROM512_BYTES, FLASH64_BYTES, FLASH128_BYTES, SRAM_BYTES};
use sha2::{Digest, Sha256};
use std::sync::mpsc::{self, Receiver, Sender};

const EEPROM_MAGIC: &[u8; 8] = b"GBAEEPR1";
const MAGIC: &[u8; 8] = b"GBASRAM1";
const FLASH128_MAGIC: &[u8; 8] = b"GBAFL128";
const FLASH_MAGIC: &[u8; 8] = b"GBAFLS64";

/// SHA-256 of the original cartridge bytes, independent of filenames and titles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity(pub [u8; 32]);

impl Identity {
    pub fn of(rom: &[u8]) -> Self {
        Self(Sha256::digest(rom).into())
    }
    pub fn key(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let mut output = String::with_capacity(64);

        for byte in self.0 {
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }

        output
    }
}

/// Identity-bearing portable image shared by disk, IndexedDB and downloads.
/// Raw saves are deliberately rejected because their cartridge cannot be verified.
pub fn encode(identity: &Identity, bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(40 + bytes.len());
    // Keep prior envelopes compatible. EEPROM declares a supported capacity;
    // the core additionally validates it against detected or overridden hardware.
    output.extend_from_slice(if matches!(bytes.len(), EEPROM512_BYTES | EEPROM8K_BYTES) {
        EEPROM_MAGIC
    } else if bytes.len() == FLASH128_BYTES {
        FLASH128_MAGIC
    } else if bytes.len() == FLASH64_BYTES {
        FLASH_MAGIC
    } else {
        MAGIC
    });
    output.extend_from_slice(&identity.0);
    output.extend_from_slice(bytes);
    output
}

pub fn decode(identity: &Identity, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let valid_sram = bytes.len() == 40 + SRAM_BYTES && &bytes[..8] == MAGIC;
    let valid_flash = bytes.len() == 40 + FLASH64_BYTES && &bytes[..8] == FLASH_MAGIC;
    let valid_banked = bytes.len() == 40 + FLASH128_BYTES && &bytes[..8] == FLASH128_MAGIC;
    let valid_eeprom = matches!(bytes.len(), 552 | 8232) && &bytes[..8] == EEPROM_MAGIC;
    if !valid_sram && !valid_flash && !valid_banked && !valid_eeprom {
        return Err(
            "Expected a GBASRAM1 (32768 bytes), GBAFLS64 (65536 bytes), GBAFL128 (131072 bytes), or GBAEEPR1 (512 or 8192 bytes) backup export".into(),
        );
    }
    if bytes[8..40] != identity.0 {
        return Err("Save belongs to a different ROM identity".into());
    }
    Ok(bytes[40..].to_vec())
}

/// Internal cartridge record. RTC metadata has its own versioned encoding;
/// portable `.sav` exports contain only the backup bytes.
pub struct StoredCartridge {
    pub backup: Option<Vec<u8>>,
    pub rtc: Option<Vec<u8>>,
}

fn encode_cartridge(
    identity: &Identity,
    backup: Option<&[u8]>,
    rtc: Option<gba_session::RtcImage>,
) -> Vec<u8> {
    let Some(rtc) = rtc else {
        return encode(identity, backup.unwrap_or_default());
    };
    let backup = backup.unwrap_or_default();
    let mut bytes = b"GBACART1".to_vec();
    bytes.extend_from_slice(&identity.0);
    bytes.extend_from_slice(&(backup.len() as u32).to_le_bytes());
    bytes.extend_from_slice(backup);
    bytes.extend_from_slice(&rtc.encode());
    bytes
}

fn decode_cartridge(identity: &Identity, bytes: &[u8]) -> Result<StoredCartridge, String> {
    if !bytes.starts_with(b"GBACART1") {
        return decode(identity, bytes).map(|backup| StoredCartridge {
            backup: Some(backup),
            rtc: None,
        });
    }
    if bytes.len() < 44 || bytes[8..40] != identity.0 {
        return Err("Invalid cartridge record identity or length".into());
    }
    let size = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    if !matches!(
        size,
        0 | EEPROM512_BYTES | EEPROM8K_BYTES | SRAM_BYTES | FLASH64_BYTES | FLASH128_BYTES
    ) || bytes.len() != 44 + size + 26
    {
        return Err("Invalid cartridge backup capacity or RTC length".into());
    }
    let rtc = bytes[44 + size..].to_vec();
    gba_session::RtcImage::decode(&rtc)?;
    Ok(StoredCartridge {
        backup: (size != 0).then(|| bytes[44..44 + size].to_vec()),
        rtc: Some(rtc),
    })
}

pub enum Outcome {
    Loaded(Result<Option<StoredCartridge>, String>),
    Written(Result<(), String>),
    Imported(Result<Option<Vec<u8>>, String>),
    Exported(Result<bool, String>),
}

pub struct Completion {
    pub identity: Identity,
    pub generation: u64,
    pub revision: u64,
    pub rtc_revision: Option<u64>,
    pub outcome: Outcome,
}

/// One in-flight operation makes imports and replacements barriers to old writes.
/// The machine retains dirty bytes until a successful matching write completes.
pub struct Storage {
    sender: Sender<Completion>,
    receiver: Receiver<Completion>,
    pub busy: bool,
    pub failed: bool,
    pub status: String,
}

impl Default for Storage {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            busy: false,
            failed: false,
            status: "No supported backup cartridge loaded".into(),
        }
    }
}

impl Storage {
    pub fn poll(&mut self) -> Option<Completion> {
        let completion = self.receiver.try_recv().ok()?;
        self.busy = false;
        Some(completion)
    }

    /// Starts work off the UI loop on native hosts and on the local WASM executor.
    fn dispatch<F>(
        &mut self,
        ctx: &eframe::egui::Context,
        identity: Identity,
        generation: u64,
        revision: u64,
        rtc_revision: Option<u64>,
        task: F,
    ) where
        F: std::future::Future<Output = Outcome> + 'static,
        F: PlatformFuture,
    {
        if self.busy {
            self.failed = true;
            self.status =
                "Storage operation rejected because another operation is still in flight".into();

            return;
        }

        self.busy = true;
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let work = async move {
            let outcome = task.await;
            let _ = sender.send(Completion {
                identity,
                generation,
                revision,
                rtc_revision,
                outcome,
            });
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(error) = std::thread::Builder::new()
            .name("cartridge-storage".into())
            .spawn(move || pollster::block_on(work))
        {
            self.busy = false;
            self.failed = true;
            self.status = format!("Storage worker failed: {error}; bytes remain pending");
        }
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(work);
    }

    pub fn load(&mut self, ctx: &eframe::egui::Context, identity: Identity, generation: u64) {
        self.status = "Loading initial backup; guest paused".into();
        let key = identity;
        self.dispatch(ctx, identity, generation, 0, None, async move {
            Outcome::Loaded(read(&key).await)
        });
    }

    pub fn write(
        &mut self,
        ctx: &eframe::egui::Context,
        identity: Identity,
        generation: u64,
        image: Option<gba_session::SaveImage>,
        rtc: Option<gba_session::RtcImage>,
    ) {
        let revision = image.as_ref().map_or(0, |image| image.revision);
        let rtc_revision = rtc.map(|image| image.revision);
        self.status = "Saving cartridge backup and clock settings".into();
        let data = encode_cartridge(
            &identity,
            image.as_ref().map(|image| image.bytes.as_slice()),
            rtc,
        );
        let key = identity;
        self.dispatch(
            ctx,
            identity,
            generation,
            revision,
            rtc_revision,
            async move { Outcome::Written(write(&key, &data).await) },
        );
    }

    pub fn import(&mut self, ctx: &eframe::egui::Context, identity: Identity, generation: u64) {
        self.status = "Selecting backup import; guest paused".into();
        let key = identity;
        // Construct the picker during the click to retain browser user activation.
        let picker = rfd::AsyncFileDialog::new()
            .add_filter("Cartridge backup", &["sav", "gbasav"])
            .pick_file();
        self.dispatch(ctx, identity, generation, 0, None, async move {
            let result = if let Some(file) = picker.await {
                #[cfg(not(target_arch = "wasm32"))]
                let result = std::fs::read(file.path()).map_err(|error| error.to_string());
                #[cfg(target_arch = "wasm32")]
                let result = wasm_bindgen_futures::JsFuture::from(file.inner().array_buffer())
                    .await
                    .map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec())
                    .map_err(|error| format!("{error:?}"));
                result
                    .and_then(|bytes| {
                        if matches!(
                            bytes.len(),
                            EEPROM512_BYTES
                                | EEPROM8K_BYTES
                                | SRAM_BYTES
                                | FLASH64_BYTES
                                | FLASH128_BYTES
                        ) {
                            Ok(bytes)
                        } else {
                            decode(&key, &bytes)
                        }
                    })
                    .map(Some)
            } else {
                Ok(None)
            };
            Outcome::Imported(result)
        });
    }

    pub fn export(
        &mut self,
        ctx: &eframe::egui::Context,
        identity: Identity,
        generation: u64,
        image: gba_session::SaveImage,
    ) {
        let bytes = image.bytes;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let picker = rfd::AsyncFileDialog::new()
                .set_file_name(format!("{}.sav", identity.key()))
                .save_file();
            self.dispatch(
                ctx,
                identity,
                generation,
                image.revision,
                None,
                async move {
                    Outcome::Exported(if let Some(file) = picker.await {
                        replace(file.path(), &bytes).map(|()| true)
                    } else {
                        Ok(false)
                    })
                },
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let result = download(&format!("{}.sav", identity.key()), &bytes).map(|()| true);
            self.dispatch(
                ctx,
                identity,
                generation,
                image.revision,
                None,
                async move { Outcome::Exported(result) },
            );
        }
    }
}

// Native tasks must be Send; browser File objects belong to the local executor.
#[cfg(not(target_arch = "wasm32"))]
pub trait PlatformFuture: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> PlatformFuture for T {}
#[cfg(target_arch = "wasm32")]
pub trait PlatformFuture {}
#[cfg(target_arch = "wasm32")]
impl<T> PlatformFuture for T {}

#[cfg(not(target_arch = "wasm32"))]
fn path(identity: &Identity) -> Result<std::path::PathBuf, String> {
    let root = std::env::var_os("GBA_SAVE_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(|p| std::path::PathBuf::from(p).join("gba-rs/saves"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|p| std::path::PathBuf::from(p).join(".local/share/gba-rs/saves"))
        })
        .ok_or("No save directory configured; set GBA_SAVE_DIR")?;
    Ok(root.join(format!("{}.gbasav", identity.key())))
}

/// Removes a staging file when replacement exits before the rename consumes it.
#[cfg(not(target_arch = "wasm32"))]
struct PendingFile(std::path::PathBuf);

#[cfg(not(target_arch = "wasm32"))]
impl Drop for PendingFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Checks file contents in bounded chunks without allocating a save-sized buffer.
#[cfg(not(target_arch = "wasm32"))]
fn verify_file(path: &std::path::Path, expected: &[u8]) -> std::io::Result<()> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0_u8; 8192];
    let mut offset = 0;

    while offset < expected.len() {
        let count = (expected.len() - offset).min(buffer.len());
        file.read_exact(&mut buffer[..count])?;

        if buffer[..count] != expected[offset..offset + count] {
            return Err(std::io::Error::other("save verification failed"));
        }

        offset += count;
    }

    let mut extra = [0_u8; 1];
    if file.read(&mut extra)? != 0 {
        return Err(std::io::Error::other("save file longer than expected"));
    }

    Ok(())
}

/// Checked replacement: complete bytes, fsync, readback, atomic rename, directory
/// sync and final readback. Any failure leaves the machine's revision pending.
#[cfg(not(target_arch = "wasm32"))]
pub fn replace(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let operation = || -> std::io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("missing save directory"))?;
        std::fs::create_dir_all(parent)?;
        // Separate application processes must never truncate each other's
        // staging files. The serial app route handles ordering within a process.
        static NEXT_FILE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let (temporary, file) = loop {
            let nonce = NEXT_FILE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let temporary =
                path.with_extension(format!("gbasav.pending-{}-{nonce}", std::process::id()));

            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        let cleanup = PendingFile(temporary);
        {
            // Close the handle before rename and before the cleanup guard runs on error.
            let mut file = file;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        verify_file(&cleanup.0, bytes)?;
        std::fs::rename(&cleanup.0, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        verify_file(path, bytes)?;
        Ok(())
    };
    operation().map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn read(identity: &Identity) -> Result<Option<StoredCartridge>, String> {
    match std::fs::read(path(identity)?) {
        Ok(bytes) => decode_cartridge(identity, &bytes).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn write(identity: &Identity, bytes: &[u8]) -> Result<(), String> {
    replace(&path(identity)?, bytes)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
// Resolve storage only after transaction completion, never request success.
async function database() {
  return await new Promise((resolve, reject) => {
    const request = indexedDB.open('gba-rs-saves', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('saves');
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error('Save database upgrade blocked'));
    request.onsuccess = () => resolve(request.result);
  });
}
export async function sramStorage(key, bytes) {
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction('saves', bytes === undefined ? 'readonly' : 'readwrite');
      const store = tx.objectStore('saves');
      const request = bytes === undefined ? store.get(key) : store.put(bytes, key);
      let value;
      request.onsuccess = () => { value = request.result; };
      tx.oncomplete = () => resolve(bytes === undefined ? value : undefined);
      tx.onabort = () => reject(tx.error || new Error('Save transaction aborted'));
      tx.onerror = () => {}; // The abort event supplies the definitive failure.
    });
  } finally { db.close(); }
}
export function sramDownload(name, bytes) {
  const url = URL.createObjectURL(new Blob([bytes], {type:'application/octet-stream'}));
  const link = document.createElement('a');
  link.href = url; link.download = name;
  document.body.appendChild(link); link.click(); link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_name = sramStorage)]
    async fn browser_storage(
        key: &str,
        bytes: wasm_bindgen::JsValue,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_name = sramDownload)]
    fn browser_download(
        name: &str,
        bytes: &js_sys::Uint8Array,
    ) -> Result<(), wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn read(identity: &Identity) -> Result<Option<StoredCartridge>, String> {
    let value = browser_storage(&identity.key(), wasm_bindgen::JsValue::UNDEFINED)
        .await
        .map_err(|e| format!("{e:?}"))?;
    if value.is_undefined() {
        return Ok(None);
    }
    decode_cartridge(identity, &js_sys::Uint8Array::new(&value).to_vec()).map(Some)
}
#[cfg(target_arch = "wasm32")]
async fn write(identity: &Identity, bytes: &[u8]) -> Result<(), String> {
    browser_storage(&identity.key(), js_sys::Uint8Array::from(bytes).into())
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}
#[cfg(target_arch = "wasm32")]
fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    browser_download(name, &js_sys::Uint8Array::from(bytes)).map_err(|e| format!("{e:?}"))
}

/// Headless native acceptance uses the same disk/envelope route as the app.
/// Run write and read in separate processes to prove full process reopening.
#[cfg(not(target_arch = "wasm32"))]
pub fn probe(mode: &str) -> Result<(), String> {
    if mode.starts_with("rtc-") {
        return probe_rtc(mode);
    }
    #[path = "../../../fixtures/diagnostics/sram/contract.rs"]
    mod contract;
    #[path = "../../../fixtures/diagnostics/flash/contract.rs"]
    mod flash_contract;
    #[path = "../../../fixtures/diagnostics/flash-banked/contract.rs"]
    mod banked_contract;
    #[path = "../../../fixtures/diagnostics/eeprom/contract.rs"]
    mod eeprom_contract;
    type GuestRun = fn(Option<&[u8]>, bool, u16) -> gba_session::SaveImage;
    let (rom, run): (&[u8], GuestRun) = if mode.starts_with("eeprom512-") {
        eeprom_contract::verify();
        (eeprom_contract::SMALL_ROM, eeprom_contract::run_small)
    } else if mode.starts_with("eeprom8k-") {
        eeprom_contract::verify();
        (eeprom_contract::LARGE_ROM, eeprom_contract::run_large)
    } else if mode.starts_with("banked-") {
        banked_contract::verify();
        (banked_contract::ROM, banked_contract::run)
    } else if mode.starts_with("flash-") {
        flash_contract::verify();
        (flash_contract::ROM, flash_contract::run)
    } else {
        contract::verify();
        (contract::ROM, contract::run)
    };
    let mode = mode
        .strip_prefix("eeprom512-")
        .or_else(|| mode.strip_prefix("eeprom8k-"))
        .or_else(|| mode.strip_prefix("banked-"))
        .or_else(|| mode.strip_prefix("flash-"))
        .unwrap_or(mode);
    let identity = Identity::of(rom);
    let saved = pollster::block_on(read(&identity))?.and_then(|record| record.backup);
    let image = match mode {
        "write" => {
            if saved.is_some() {
                return Err("Probe write needs a fresh GBA_SAVE_DIR".into());
            }
            let image = run(None, true, 1);
            pollster::block_on(write(&identity, &encode(&identity, &image.bytes)))?;
            image
        }
        "read" => {
            let bytes = saved.ok_or("Probe expected an existing saved score")?;
            run(Some(&bytes), false, 1)
        }
        _ => return Err("Use --save-probe [flash-|banked-|eeprom512-|eeprom8k-]write or [flash-|banked-|eeprom512-|eeprom8k-]read".into()),
    };
    let portable = encode(&identity, &image.bytes);

    let decoded = decode(&identity, &portable)?;

    if decoded.as_slice() != image.bytes.as_slice() {
        return Err("Portable save round-trip changed backup bytes".into());
    }

    if decode(&Identity::of(b"different cartridge"), &portable).is_ok() {
        return Err("Portable save accepted the wrong ROM identity".into());
    }

    let export_path = path(&identity)?.with_extension("export.gbasav");

    replace(&export_path, &portable)?;

    let exported = std::fs::read(&export_path).map_err(|error| error.to_string())?;
    let decoded_export = decode(&identity, &exported)?;

    if decoded_export.as_slice() != image.bytes.as_slice() {
        return Err("Exported save round-trip changed backup bytes".into());
    }

    if replace(std::path::Path::new("/dev/null/gba-save"), &portable).is_ok() {
        return Err("Invalid replacement target unexpectedly succeeded".into());
    }
    println!(
        "Native backup {mode} PASS: ROM={}, save={}, export/import validated, failed replacement rejected",
        identity.key(),
        path(&identity)?.display()
    );
    Ok(())
}

/// Uses the app's atomic native record and raw export routes in separate
/// processes. Fixed time proves offline elapsed seconds without sleeping.
#[cfg(not(target_arch = "wasm32"))]
fn probe_rtc(mode: &str) -> Result<(), String> {
    #[path = "../../../fixtures/diagnostics/rtc/contract.rs"]
    mod contract;
    contract::verify();
    let identity = Identity::of(contract::ROM);
    let mut session = gba_session::Session::new();
    session.load_rom(contract::ROM).map_err(|e| e.to_string())?;
    match mode {
        "rtc-write" => {
            if pollster::block_on(read(&identity))?.is_some() {
                return Err("RTC probe requires fresh storage".into());
            }
            session.set_rtc_time(contract::FIXED_TIME);
            for _ in 0..5 {
                session.advance_frame().map_err(|e| e.to_string())?;
            }
            let backup = session.save_image().unwrap();
            let bytes = encode_cartridge(&identity, Some(&backup.bytes), session.rtc_image());
            pollster::block_on(write(&identity, &bytes))?;
            if decode_cartridge(&Identity::of(b"wrong ROM"), &bytes).is_ok()
                || decode_cartridge(&identity, &bytes[..bytes.len() - 1]).is_ok()
            {
                return Err("RTC record accepted invalid identity or truncation".into());
            }
            let export = path(&identity)?.with_extension("sav");
            replace(&export, &backup.bytes)?;
            if std::fs::read(export).map_err(|e| e.to_string())? != backup.bytes {
                return Err("Raw save export changed backup data".into());
            }
            if replace(std::path::Path::new("/dev/null/rtc"), &bytes).is_ok() {
                return Err("Invalid RTC storage target succeeded".into());
            }
        }
        "rtc-read" => {
            let record = pollster::block_on(read(&identity))?.ok_or("RTC record missing")?;
            session.set_rtc_time(contract::FIXED_TIME + 65);
            session.load_save(&record.backup.ok_or("Backup missing")?)?;
            session.load_rtc(&record.rtc.ok_or("RTC metadata missing")?)?;
            for _ in 0..5 {
                session.advance_frame().map_err(|e| e.to_string())?;
            }
            if session.inspect16(0x02000000).unwrap() != 0x0324
                || session.inspect16(0x02000002).unwrap() != 0x0501
                || session.inspect16(0x02000004).unwrap() != 0x0100
                || session.inspect16(0x02000006).unwrap() != 0x4003
            {
                return Err("Reopened guest lost clock settings or offline elapsed time".into());
            }
        }
        _ => return Err("Use --save-probe rtc-write or rtc-read".into()),
    }
    println!(
        "Native RTC {mode} PASS: identity={}, file={}",
        identity.key(),
        path(&identity)?.display()
    );
    Ok(())
}
