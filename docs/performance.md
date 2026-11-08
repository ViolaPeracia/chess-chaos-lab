# Performance

The UI dispatches internal AI and UCI work to background threads. Internal search is fixed at depth three at most, with no transposition table or configurable resource budget. UCI analysis has a one-second move time and a five-second timeout margin. `perft` is a developer action and runs on the UI thread at depth three; larger depths are not exposed.

The previously reported benchmark (9 November 2026) used an AMD Ryzen 7 7445HS and five sequential release-profile depth-5 samples: 54.499, 60.001, 58.220, 59.448 and 57.136 ms; reported median **58.220 ms** for 4,865,609 leaves. Run `cargo run --release --locked -p chess-chaos-ai --example perft-benchmark` to reproduce the count and time it on your machine. This turn has not repeated the five-sample timing, so the median remains a reported baseline. OS load and exact build environment were not recorded. This is not an app FPS or AI-strength measure; no RAM, battery, search NPS, or long-running tournament benchmark is available.

