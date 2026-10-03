// Shared original demo input. The headless manifest independently freezes these
// transitions; the runner verifies equality before accepting the app replay.
[
    (Cycle(0), Button::Right, true),
    (Cycle(842_688), Button::Right, false),
    (Cycle(842_688), Button::Down, true),
    (Cycle(1_404_480), Button::Down, false),
    (Cycle(1_685_376), Button::Left, true),
    (Cycle(1_966_272), Button::Left, false),
    (Cycle(1_966_272), Button::Up, true),
    (Cycle(2_247_168), Button::Up, false),
]
