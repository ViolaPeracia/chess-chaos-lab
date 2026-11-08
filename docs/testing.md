# Testing and evidence

Commands run on Windows x64:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --offline --locked
cargo build --release --offline --locked
```

Twenty-three automated tests passed: initial perft depths 1–3, legal AI moves, a 100-ply AI match simulation, material evaluation, mate with undo/redo, explicit resignation side, illegal FEN, en passant, underpromotion, castling, stalemate, insufficient material, threefold/fivefold repetition, 50/75-move thresholds including checkmate priority, save recovery, Fischer/Bronstein/hourglass clock and time-forfeit rules, backward-compatible Fischer clock saves, plain PGN round trip including a Black-to-move FEN, UCI info parsing, and a full subprocess handshake with a local test engine. More FIDE edge positions and property/fuzz tests remain. The final test/build run used `--offline --locked`.

Manual GUI smoke: app launched on Windows; board and controls rendered; selected `e2`, legal targets highlighted; played `e4`; turn, FEN, move list and autosave changed; after closing and reopening, the game restored. Started a 10+5 Fischer game, saw the clock count down and increment after `e4`, paused it, confirmed move selection was locked, and reopened to find the saved game and clock paused. In a Bronstein game, the initial display held at 01:00 during the delay and then counted down. In an hourglass game, White fell to 00:58 while Black rose to 01:03; reopening restored both times, Hourglass mode, and the paused state. Selected **Local UCI engine**, pasted the local fixture path, played `e4`, and observed the fixture's `e5` response in the board, move list, and FEN. Repeated `e4 e5` with official Stockfish 19, then analyzed the position for one second. The display showed `g1f3`, depth 21, score +35 cp from White's perspective, and 764,570 nodes. The Windows accessibility tree exposed buttons, labels, edits and the opponent selector. **Copy FEN** populated the OS clipboard with the current position. A complete game and full AI-vs-AI GUI match were not tested. See `known-limitations.md`.

