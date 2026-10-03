// Recorded Unix-time samples at the original guest boundaries. Replay consumes
// this retained timeline, so it never depends on a fresh host clock sample.
[
    RtcTimeEvent { cycle: Cycle(0), unix_seconds: 1_700_000_000 },
    RtcTimeEvent { cycle: Cycle(1_404_485), unix_seconds: 1_700_000_005 },
]
