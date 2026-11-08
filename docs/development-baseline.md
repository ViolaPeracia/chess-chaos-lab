# Prior development report — 2026-11-09

This is the pre-repository development report retained as historical context. Its test, Stockfish, GUI, and benchmark details were reported before the GitHub setup; this file is not evidence that those checks were rerun for the public release. Current status is in [ROADMAP.md](ROADMAP.md) and the delivery report.

## A. Product overview

Chess Chaos Lab 0.1.0 is a playable, native Windows chess development build. It is far short of the requested feature-rich release.

## B. Actual architecture

Four Rust crates: chess core (`shakmaty`), basic local AI, UCI subprocess adapter, and native `eframe` UI. See `architecture.md`.

## C. Implemented features

Standard legal chess, local two-player play, four basic AI choices, AI-vs-AI autoplay, draw claim/automatic draw bookkeeping, Fischer/Bronstein/hourglass clocks with pause and time forfeits, FEN, plain PGN mainline import/export, undo/redo for untimed games, two themes, autosave, one-shot UCI analysis, White versus a local UCI engine, perft inspector, Windows clipboard export and accessibility controls.

## D. In progress or incomplete

Remaining clock variants, full PGN, chess variants, training, openings, tournaments, game library, advanced AI, statistics, audio, achievements, accessibility review, installer packaging, and more. See `feature-matrix.md`.

## E. AI and engines that actually work

Random legal moves, capture hunter, material collector, and depth-three alpha-beta select legal moves in tests. The UCI adapter passed a round trip with a deterministic local test executable. The GUI accepted `e4` and applied `e5` from both the fixture and official Stockfish 19. Stockfish also returned a one-second analysis with a best move, depth, score and node count. The official release ZIP's SHA-256 matched its published asset digest; the engine is not bundled. A full real-engine match and failure-path tests remain.

## F. Automated tests

Twenty-three tests passed. `cargo fmt --check`, Clippy with warnings denied, `cargo test --workspace --offline --locked`, and release build passed on Windows x64. See `testing.md`.

## G. Performance benchmarks

Perft depth 5 yielded 4,865,609 leaves in a median 58.220 ms over five samples on the AMD Ryzen 7 7445HS laptop. Search is capped at depth three and off the UI thread. See `performance.md`.

## H. Offline verification

Test and release build passed with Cargo offline mode using the local dependency cache. Runtime code has no network client. A process socket snapshot found no TCP connections or UDP endpoints; continuous packet capture is pending.

## I. Findings and fixes

The first build exposed changed `shakmaty` SAN ownership and eframe UI APIs; both were corrected. A GUI screenshot exposed a collapsed board and horizontal controls; layout was rebuilt and visually rechecked. A mislabeled material metric was corrected and tested. A PGN result-token parsing edge case was fixed. A resignation-side bug in human-versus-computer play was fixed and regression-tested. Missing eframe clipboard and accessibility features were enabled; pasted engine paths, OS clipboard export, and the Windows accessibility tree were then verified.

## J. Executable and run instructions

`target/release/chess-chaos-lab.exe` was built and launched. A portable ZIP is in `dist/`. Run the executable directly or use `cargo run --release -p chess-chaos-lab`. It saves the current game under `%LOCALAPPDATA%\ChessChaosLab`.

## K. Remaining limits

See `known-limitations.md`. The project is not release-ready by the user's acceptance criteria. In particular, real-engine testing covers only one move and one analysis; full PGN, installer and Linux build are absent.

## L. Next work

Complete FIDE edge tests, remaining clock modes, full PGN parser, robust save recovery, real-engine match and failure-path tests, and repeatable GUI automation before broader feature expansion.
