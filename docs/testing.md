# Testing and evidence

Commands run on Windows x64:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
npm ci --prefix site
npm run build --prefix site
```

Twenty-seven automated tests passed: the original 23 plus accessible square-name, corrupt-clock-save preservation, truncated/oversized-autosave, and combo-box focus-ID regressions. The original suite covers initial perft depths 1–3, legal AI moves, a 100-ply AI simulation, material evaluation, mate with undo/redo, resignation, illegal FEN, en passant, underpromotion, castling, draw rules, save recovery, three clock modes and time-forfeit rules, plain PGN round trip, UCI parsing and a subprocess fixture handshake. More FIDE edge positions and property/fuzz tests remain. The M0 branch used the commands above; the original baseline run used `--offline --locked`.

## M1 branch evidence

The M1 branch adds reference perft positions 2–4 (depths 1–3), castling-through-check and discovered-check en-passant regressions, announced-move draw claims, insufficient-material and resignation edge tests, generated legal-game PGN round trips, nested RAV and comment/NAG/header/FEN round trips, invalid-input and resource-limit cases, 2,000 deterministic small malformed-input parser probes, variation navigation and autosave tests, and a file-import/UI row integration test. The perft counts come from [Chess Programming Wiki's reference results](https://www.chessprogramming.org/Perft_Results); draw distinctions follow [FIDE Laws of Chess articles 5 and 9](https://handbook.fide.com/chapter/e012023). PGN structure and Seven Tag Roster follow the [PGN specification](https://www.saremba.de/chessgml/standards/pgn/pgn-complete.htm). The malformed-input probes are a bounded property-style check, not a coverage-guided fuzz target.

PGN import accepts exactly one game. A second game or content after its result is an error; missing or malformed header quoting, illegal SAN, mismatched result tags, unmatched parentheses, excessive NAGs, and an invalid SetUp/FEN combination are tested. Export normalizes move numbers and whitespace while preserving the parsed tree, headers, comments, NAGs and result. Input is bounded to 1 MB, 50,000 nodes and 64 RAV levels. Manual GUI acceptance and PR CI must be recorded before M1 is marked complete.

The latest local Windows workspace run passed 44 automated tests, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo build --release --locked`. The docs site passed locked dependency installation, Astro check/build and generated-link verification. Direct Windows UI automation could not launch the executable because the Computer Use app approval timed out; that manual path remains unverified.

The [M0 Windows audit](m0-windows-audit.md) records manual GUI, keyboard, Narrator Speech Recap, display scaling, Windows UI Automation, autosave-failure and crash-recovery observations. It distinguishes passed paths from still-unverified broader cases. The isolated build used `cargo build --release --locked --target-dir .m0-target` because an existing Windows process initially held the default `target/release/chess-chaos-lab.exe` open.

Earlier reported manual GUI smoke (pre-repository; only the app launch and accessibility tree were repeated during that setup): app launched on Windows; board and controls rendered; selected `e2`, legal targets highlighted; played `e4`; turn, FEN, move list and autosave changed; after closing and reopening, the game restored. Started a 10+5 Fischer game, saw the clock count down and increment after `e4`, paused it, confirmed move selection was locked, and reopened to find the saved game and clock paused. In a Bronstein game, the initial display held at 01:00 during the delay and then counted down. In an hourglass game, White fell to 00:58 while Black rose to 01:03; reopening restored both times, Hourglass mode, and the paused state. Selected **Local UCI engine**, pasted the local fixture path, played `e4`, and observed the fixture's `e5` response in the board, move list, and FEN. Repeated `e4 e5` with official Stockfish 19, then analyzed the position for one second. The display showed `g1f3`, depth 21, score +35 cp from White's perspective, and 764,570 nodes. The Windows accessibility tree exposed buttons, labels, edits and the opponent selector. **Copy FEN** populated the OS clipboard with the current position. A complete game and full AI-vs-AI GUI match were not tested at that earlier stage; the [M0 Windows audit](m0-windows-audit.md) includes them.
