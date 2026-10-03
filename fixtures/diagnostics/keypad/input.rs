// Shared logical transitions: unrelated Right, incomplete A, then complete A+B.
[
    (Cycle(10_000), Button::Right, true),
    (Cycle(12_000), Button::Right, false),
    (Cycle(20_000), Button::A, true),
    (Cycle(30_000), Button::B, true),
    (Cycle(35_000), Button::A, false),
    (Cycle(35_000), Button::B, false),
]
