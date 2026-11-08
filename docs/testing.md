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

Twenty-six automated tests passed: the original 23 plus accessible square-name, corrupt-clock-save preservation, and truncated/oversized-autosave regressions. The original suite covers initial perft depths 1–3, legal AI moves, a 100-ply AI simulation, material evaluation, mate with undo/redo, resignation, illegal FEN, en passant, underpromotion, castling, draw rules, save recovery, three clock modes and time-forfeit rules, plain PGN round trip, UCI parsing and a subprocess fixture handshake. More FIDE edge positions and property/fuzz tests remain. The M0 branch used the commands above; the original baseline run used `--offline --locked`.

The [M0 Windows audit](m0-windows-audit.md) records the new manual GUI, keyboard, Windows UI Automation, autosave-failure and crash-recovery observations. It also lists every manual gate still **Not Verified**. The isolated build used `cargo build --release --locked --target-dir .m0-target` because an existing Windows process held the default `target/release/chess-chaos-lab.exe` open.

Earlier reported manual GUI smoke (pre-repository; only the app launch and accessibility tree were repeated during that setup): app launched on Windows; board and controls rendered; selected `e2`, legal targets highlighted; played `e4`; turn, FEN, move list and autosave changed; after closing and reopening, the game restored. Started a 10+5 Fischer game, saw the clock count down and increment after `e4`, paused it, confirmed move selection was locked, and reopened to find the saved game and clock paused. In a Bronstein game, the initial display held at 01:00 during the delay and then counted down. In an hourglass game, White fell to 00:58 while Black rose to 01:03; reopening restored both times, Hourglass mode, and the paused state. Selected **Local UCI engine**, pasted the local fixture path, played `e4`, and observed the fixture's `e5` response in the board, move list, and FEN. Repeated `e4 e5` with official Stockfish 19, then analyzed the position for one second. The display showed `g1f3`, depth 21, score +35 cp from White's perspective, and 764,570 nodes. The Windows accessibility tree exposed buttons, labels, edits and the opponent selector. **Copy FEN** populated the OS clipboard with the current position. A complete game and full AI-vs-AI GUI match were not tested at that earlier stage; the [M0 Windows audit](m0-windows-audit.md) includes them.
