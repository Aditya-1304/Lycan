//! Original guest acceptance: fixed time, calendar rollover, lifecycle retention,
//! replay and complete scanout. No host clock is read by this contract.
use gba_session::{Button, Cycle, RtcTimeEvent, Session};

pub const ROM: &[u8] = include_bytes!("../rtc.gba");
pub const FIXED_TIME: u64 = 1_700_000_000;
const TIME_EVENTS: [RtcTimeEvent; 2] = include!("time_events.rs");

fn run(session: &mut Session, expected: [u8; 8]) {
    let start = session.cycles().0;
    let mut instructions = 0;
    let mut pc = 0;
    for _ in 0..5 {
        let report = session.advance_frame().unwrap().unwrap();
        assert!((0x0800012c..=0x08000340).contains(&report.instruction_address));
        assert!(report.instructions <= 200_000);
        instructions += report.instructions;
        pc = report.instruction_address;
    }
    assert!(session.cycles().0 - start <= 5 * gba_session::CYCLES_PER_FRAME + 32);
    for (index, pair) in expected.as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(
            session.inspect16(0x02000000 + index as u32 * 2).unwrap(),
            u16::from_le_bytes([pair[0], pair[1]])
        );
    }
    assert_eq!(session.inspect16(0x02000010).unwrap(), 0x30);
    assert!(session.framebuffer_generation() >= 5);
    // Independent glyph oracle uses visual row strings rather than the guest's
    // packed table. Verify every pixel, including the untouched black background.
    let glyphs = [
        "111101101101111",
        "010110010010111",
        "111001111100111",
        "111001111001111",
        "101101111001001",
        "111100111001111",
        "111100111101111",
        "111001001001001",
        "111101111101111",
        "111101111001111",
    ];
    for y in 0..160 {
        for x in 0..240 {
            let row = expected[y / 20];
            let digit = if x < 12 {
                Some(row >> 4)
            } else if (16..28).contains(&x) {
                Some(row & 15)
            } else {
                None
            };
            let color = digit.filter(|d| *d < 10).map_or(0, |d| {
                let bit = ((y % 20) / 4) * 3 + (x % 16) / 4;
                if glyphs[usize::from(d)].as_bytes()[bit] == b'1' {
                    0x03e0
                } else {
                    0
                }
            });
            assert_eq!(
                session.framebuffer()[y * 240 + x],
                color,
                "scanout ({x},{y})"
            );
        }
    }
    println!(
        "RTC checkpoint: bcd={expected:02x?} cycles={} instructions={instructions} pc={pc:#010x} generation={}",
        session.cycles().0,
        session.framebuffer_generation()
    );
}

/// Reusable diagnostic for the session test and production CLI verifier.
pub fn verify() -> Vec<u16> {
    let initial = [0x24, 2, 0x29, 4, 0x23, 0x59, 0x58, 0x40];
    let rollover = [0x24, 3, 1, 5, 0, 0, 3, 0x40];
    let reopened = [0x24, 3, 1, 5, 0, 1, 3, 0x40];
    let mut session = Session::new();
    session.load_rom(ROM).unwrap();
    let first = session.set_rtc_time(FIXED_TIME);
    run(&mut session, initial);
    session.toggle_pause();
    let paused_cycle = session.cycles();
    let second = session.set_rtc_time(FIXED_TIME + 5);
    assert_eq!([first, second], TIME_EVENTS);
    println!("RTC recorded time events: {first:?}, {second:?}");
    assert!(session.advance_frame().unwrap().is_none());
    assert_eq!(session.cycles(), paused_cycle);
    session.toggle_pause();
    run(&mut session, rollover);
    let rtc = session.rtc_image().unwrap();
    assert!(rtc.dirty);
    // A stale storage completion must not acknowledge guest changes.
    session.acknowledge_rtc(rtc.revision - 1);
    assert!(session.rtc_image().unwrap().dirty);
    session.acknowledge_rtc(rtc.revision);
    assert!(!session.rtc_image().unwrap().dirty);
    session.reset();
    run(&mut session, rollover);
    let backup = session.save_image().unwrap();
    let metadata = rtc.encode();
    assert_eq!(backup.bytes.len(), gba_session::SRAM_BYTES);
    assert_eq!(metadata.len(), 26);
    let mut opened = Session::new();
    opened.load_rom(ROM).unwrap();
    opened.set_rtc_time(FIXED_TIME + 65);
    opened.load_save(&backup.bytes).unwrap();
    opened.load_rtc(&metadata).unwrap();
    run(&mut opened, reopened);
    // Malformed metadata cannot alter a live configured clock.
    let before = opened.rtc_image();
    assert!(opened.load_rtc(&metadata[..25]).is_err());
    assert_eq!(opened.rtc_image(), before);
    let mut replay = Session::new();
    replay.load_rom(ROM).unwrap();
    replay.replay_rtc_time(TIME_EVENTS[0]).unwrap();
    run(&mut replay, initial);
    replay.toggle_pause();
    replay.replay_rtc_time(TIME_EVENTS[1]).unwrap();
    replay.toggle_pause();
    run(&mut replay, rollover);
    assert_eq!(replay.framebuffer(), session.framebuffer());
    assert!(
        replay
            .replay_rtc_time(RtcTimeEvent {
                cycle: Cycle(0),
                unix_seconds: 0
            })
            .is_err()
    );
    // User A writes a new date, which must receive a new persistent revision.
    opened.set_button(Button::A, true);
    for _ in 0..2 {
        opened.advance_frame().unwrap();
    }
    opened.set_button(Button::A, false);
    run(&mut opened, initial);
    assert!(opened.rtc_image().unwrap().dirty);
    opened.set_button(Button::B, true);
    for _ in 0..2 {
        opened.advance_frame().unwrap();
    }
    opened.set_button(Button::B, false);
    let twelve_hour = [0x24, 2, 0x29, 4, 0x91, 0x59, 0x58, 0];
    run(&mut opened, twelve_hour);
    opened.reset();
    run(&mut opened, twelve_hour);
    let backup = opened.save_image().unwrap();
    let metadata = opened.rtc_image().unwrap().encode();
    let mut restored = Session::new();
    restored.load_rom(ROM).unwrap();
    restored.set_rtc_time(FIXED_TIME + 65);
    restored.load_save(&backup.bytes).unwrap();
    restored.load_rtc(&metadata).unwrap();
    run(&mut restored, twelve_hour);
    println!(
        "RTC PASS: leap-day rollover, pause/reset/reopen, A/B settings, 12-hour mode, revision barrier, fixed-time replay, full-frame oracle; final cycles={}",
        opened.cycles().0
    );
    opened.framebuffer().to_vec()
}
