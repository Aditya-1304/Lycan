//! Runtime BIOS bridge for the production WASM core, with no native fallback.
#[path = "contract.rs"]
mod contract;
use std::cell::RefCell;
thread_local! {
    static BIOS: RefCell<Vec<u8>> = RefCell::new(vec![0; gba_core::BIOS_SIZE]);
    static MACHINE: RefCell<Option<gba_core::Machine>> = const { RefCell::new(None) };
}
/// Copies a host-supplied byte without exposing raw pointers across the boundary.
#[unsafe(no_mangle)]
pub extern "C" fn bios_byte(index: usize, value: u8) {
    BIOS.with(|bios| bios.borrow_mut()[index] = value);
}
#[unsafe(no_mangle)]
pub extern "C" fn verify() {
    let machine = BIOS.with(|bios| contract::verify(&bios.borrow())).expect("BIOS guest contract");
    MACHINE.with(|slot| *slot.borrow_mut() = Some(machine));
}
#[unsafe(no_mangle)]
pub extern "C" fn pixel(index: usize) -> u16 {
    MACHINE.with(|slot| slot.borrow().as_ref().unwrap().framebuffer()[index])
}
