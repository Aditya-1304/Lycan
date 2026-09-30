//! Platform storage boundary. The app dispatches at most one operation at a time;
//! every completion carries ROM identity, session generation and snapshot revision.

use gba_session::{FLASH64_BYTES, FLASH128_BYTES, SRAM_BYTES};
use sha2::{Digest, Sha256};
use std::sync::mpsc::{self, Receiver, Sender};

const MAGIC: &[u8; 8] = b"GBASRAM1";
const FLASH128_MAGIC: &[u8; 8] = b"GBAFL128";
const FLASH_MAGIC: &[u8; 8] = b"GBAFLS64";

/// SHA-256 of the original cartridge bytes, independent of filenames and titles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity(pub [u8; 32]);

impl Identity {
    pub fn of(rom: &[u8]) -> Self {
        Self(Sha256::digest(rom).into())
    }
    pub fn key(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

/// Identity-bearing portable image shared by disk, IndexedDB and downloads.
/// Raw saves are deliberately rejected because their cartridge cannot be verified.
pub fn encode(identity: &Identity, bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(40 + bytes.len());
    // Keep the existing SRAM envelope byte-for-byte compatible. Flash exports
    // declare their own capacity; the core validates it against the loaded chip.
    output.extend_from_slice(if bytes.len() == FLASH128_BYTES {
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
    if !valid_sram && !valid_flash && !valid_banked {
        return Err(
            "Expected a GBASRAM1 (32768 bytes), GBAFLS64 (65536 bytes), or GBAFL128 (131072 bytes) backup export".into(),
        );
    }
    if bytes[8..40] != identity.0 {
        return Err("Save belongs to a different ROM identity".into());
    }
    Ok(bytes[40..].to_vec())
}

pub enum Outcome {
    Loaded(Result<Option<Vec<u8>>, String>),
    Written(Result<(), String>),
    Imported(Result<Option<Vec<u8>>, String>),
    Exported(Result<bool, String>),
}

pub struct Completion {
    pub identity: Identity,
    pub generation: u64,
    pub revision: u64,
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
        task: F,
    ) where
        F: std::future::Future<Output = Outcome> + 'static,
        F: PlatformFuture,
    {
        assert!(!self.busy, "storage operations must be serialized");
        self.busy = true;
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let work = async move {
            let outcome = task.await;
            let _ = sender.send(Completion {
                identity,
                generation,
                revision,
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
        let key = identity.clone();
        self.dispatch(ctx, identity, generation, 0, async move {
            Outcome::Loaded(read(&key).await)
        });
    }

    pub fn write(
        &mut self,
        ctx: &eframe::egui::Context,
        identity: Identity,
        generation: u64,
        image: gba_session::SaveImage,
    ) {
        self.status = format!(
            "Saving revision {}; awaiting storage completion",
            image.revision
        );
        let data = encode(&identity, &image.bytes);
        let key = identity.clone();
        self.dispatch(ctx, identity, generation, image.revision, async move {
            Outcome::Written(write(&key, &data).await)
        });
    }

    pub fn import(&mut self, ctx: &eframe::egui::Context, identity: Identity, generation: u64) {
        self.status = "Selecting backup import; guest paused".into();
        let key = identity.clone();
        // Construct the picker during the click to retain browser user activation.
        let picker = rfd::AsyncFileDialog::new()
            .add_filter("Cartridge backup", &["gbasav"])
            .pick_file();
        self.dispatch(ctx, identity, generation, 0, async move {
            let result = if let Some(file) = picker.await {
                #[cfg(not(target_arch = "wasm32"))]
                let result = std::fs::read(file.path()).map_err(|error| error.to_string());
                #[cfg(target_arch = "wasm32")]
                let result = wasm_bindgen_futures::JsFuture::from(file.inner().array_buffer())
                    .await
                    .map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec())
                    .map_err(|error| format!("{error:?}"));
                result.and_then(|bytes| decode(&key, &bytes)).map(Some)
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
        let bytes = encode(&identity, &image.bytes);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let picker = rfd::AsyncFileDialog::new()
                .set_file_name(format!("{}.gbasav", identity.key()))
                .save_file();
            self.dispatch(ctx, identity, generation, image.revision, async move {
                Outcome::Exported(if let Some(file) = picker.await {
                    replace(file.path(), &bytes).map(|()| true)
                } else {
                    Ok(false)
                })
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let result = download(&format!("{}.gbasav", identity.key()), &bytes).map(|()| true);
            self.dispatch(ctx, identity, generation, image.revision, async move {
                Outcome::Exported(result)
            });
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
        let nonce = NEXT_FILE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temporary =
            path.with_extension(format!("gbasav.pending-{}-{nonce}", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if std::fs::read(&temporary)? != bytes {
            return Err(std::io::Error::other("save verification failed"));
        }
        std::fs::rename(&temporary, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        if std::fs::read(path)? != bytes {
            return Err(std::io::Error::other("replacement verification failed"));
        }
        Ok(())
    };
    operation().map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn read(identity: &Identity) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path(identity)?) {
        Ok(bytes) => decode(identity, &bytes).map(Some),
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
async fn read(identity: &Identity) -> Result<Option<Vec<u8>>, String> {
    let value = browser_storage(&identity.key(), wasm_bindgen::JsValue::UNDEFINED)
        .await
        .map_err(|e| format!("{e:?}"))?;
    if value.is_undefined() {
        return Ok(None);
    }
    decode(identity, &js_sys::Uint8Array::new(&value).to_vec()).map(Some)
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
    #[path = "../../../roms/sram/contract.rs"]
    mod contract;
    #[path = "../../../roms/flash/contract.rs"]
    mod flash_contract;
    #[path = "../../../roms/flash-banked/contract.rs"]
    mod banked_contract;
    type GuestRun = fn(Option<&[u8]>, bool, u16) -> gba_session::SaveImage;
    let (rom, run): (&[u8], GuestRun) = if mode.starts_with("banked-") {
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
        .strip_prefix("banked-")
        .or_else(|| mode.strip_prefix("flash-"))
        .unwrap_or(mode);
    let identity = Identity::of(rom);
    let saved = pollster::block_on(read(&identity))?;
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
        _ => return Err("Use --save-probe [flash-|banked-]write or [flash-|banked-]read".into()),
    };
    let portable = encode(&identity, &image.bytes);
    assert_eq!(decode(&identity, &portable)?, image.bytes);
    assert!(decode(&Identity::of(b"different cartridge"), &portable).is_err());
    let export_path = path(&identity)?.with_extension("export.gbasav");
    replace(&export_path, &portable)?;
    assert_eq!(
        decode(
            &identity,
            &std::fs::read(&export_path).map_err(|e| e.to_string())?
        )?,
        image.bytes
    );
    assert!(replace(std::path::Path::new("/dev/null/gba-save"), &portable).is_err());
    println!(
        "Native backup {mode} PASS: ROM={}, save={}, export/import validated, failed replacement rejected",
        identity.key(),
        path(&identity)?.display()
    );
    Ok(())
}
