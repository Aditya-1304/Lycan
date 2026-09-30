// Continue the original input replay after its synchronized PCM checkpoints.
// Each mixer setting remains active for two frames before its stable capture.
[
    (Cycle(10 * CYCLES_PER_FRAME), Button::A, true),
    (Cycle(12 * CYCLES_PER_FRAME), Button::Left, true),
    (Cycle(14 * CYCLES_PER_FRAME), Button::Left, false),
    (Cycle(14 * CYCLES_PER_FRAME), Button::Right, true),
    (Cycle(16 * CYCLES_PER_FRAME), Button::Right, false),
    (Cycle(16 * CYCLES_PER_FRAME), Button::Down, true),
    (Cycle(18 * CYCLES_PER_FRAME), Button::Down, false),
    (Cycle(20 * CYCLES_PER_FRAME), Button::Select, true),
    (Cycle(22 * CYCLES_PER_FRAME), Button::Select, false),
    (Cycle(22 * CYCLES_PER_FRAME), Button::Up, true),
    (Cycle(24 * CYCLES_PER_FRAME), Button::Up, false),
    (Cycle(24 * CYCLES_PER_FRAME), Button::B, true),
    (Cycle(26 * CYCLES_PER_FRAME), Button::B, false),
]
