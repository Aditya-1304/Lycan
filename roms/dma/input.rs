// Logical input events occur at frame boundaries, before VBlank samples KEYINPUT.
[
    (Cycle(4 * CYCLES_PER_FRAME), Button::A, true),
    (Cycle(7 * CYCLES_PER_FRAME), Button::A, false),
]
