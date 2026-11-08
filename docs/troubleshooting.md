# Troubleshooting

## The application does not build

Run `rustc --version`, `cargo --version`, then `cargo build --release --locked` at the repository root. On Windows, install the MSVC Rust target, Visual Studio C++ Build Tools, and Windows SDK. The first build may need network access to fetch dependencies; `--offline` works only after the cache is populated.

## An engine does not answer

Use an absolute path to a trusted local UCI executable. Check that it launches directly from a terminal and speaks the UCI protocol. A single analysis request has a bounded wait. See [engine integration](engine-integration.md). The app cannot configure arbitrary engine options yet.

## A saved game is missing or damaged

The app keeps one autosave at `%LOCALAPPDATA%\ChessChaosLab\autosave.txt`. Back up this file before changing it. Corrupt data is ignored at startup; there is no recovery UI. Timed games reopen paused and need **Resume clock**.

## PGN import rejects a game

The importer accepts one plain mainline. Remove comments, variations and NAGs before import. Full PGN support is [planned](ROADMAP.md).
