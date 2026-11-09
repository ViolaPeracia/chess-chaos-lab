# Troubleshooting

## The application does not build

Run `rustc --version`, `cargo --version`, then `cargo build --release --locked` at the repository root. On Windows, install the MSVC Rust target, Visual Studio C++ Build Tools, and Windows SDK. The first build may need network access to fetch dependencies; `--offline` works only after the cache is populated.

## An engine does not answer

Use an absolute path to a trusted local UCI executable. Check that it launches directly from a terminal and speaks the UCI protocol. A single analysis request has a bounded wait. See [engine integration](engine-integration.md). The app cannot configure arbitrary engine options yet.

## A saved game is missing or damaged

The app keeps one autosave at `%LOCALAPPDATA%\ChessChaosLab\autosave.txt`. Back up this file before changing it. If loading fails, the app copies the damaged file to `autosave-corrupt-*.txt` and starts a new game. If that copy fails, saving is disabled for the session so the original is not overwritten. Fix the path or permissions, then restart. There is no recovery UI. A failed save may leave a newer `autosave.tmp`; inspect both files before replacing either. Timed games reopen paused and need **Resume clock**.

## PGN import rejects a game

The importer accepts one plain mainline. Remove comments, variations and NAGs before import. Full PGN support is [planned](ROADMAP.md).

## Keyboard focus or the board is hard to use

Tab and Shift+Tab move between controls. Once a square has focus, arrow keys move around the board, and Enter or Space selects a piece or destination. Escape clears a selected piece and closes an open opponent or clock menu; focus returns to that menu. If the board is cut off after changing Windows display scaling, close and reopen the app so Windows applies the new scale, or maximize the window. The M0 audit verified the complete board at 125% and 150% scaling.
