// Frame-boundary input is delivered before the guest's next VBlank callback.
[
    (Cycle(4 * CYCLES_PER_FRAME), Button::A, true),
    (Cycle(7 * CYCLES_PER_FRAME), Button::A, false),
]
