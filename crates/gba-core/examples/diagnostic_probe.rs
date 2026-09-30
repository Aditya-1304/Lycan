use gba_core::Machine;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let rom = std::fs::read(path).unwrap();
    let mut machine = Machine::new();
    machine.load_rom(&rom).unwrap();
    for _ in 0..2_000_000 {
        match machine.step() {
            Ok(pc) => {
                if pc == 0x080034f0 || pc == 0x08000dcc {
                    println!("terminal {pc:#010x}");
                    break;
                }
            }
            Err(e) => { println!("{e}"); break; }
        }
    }
}
