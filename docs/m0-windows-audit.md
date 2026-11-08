# M0 Windows desktop audit — 9 November 2026

## Environment and method

- Windows x64, Rust stable MSVC 1.99, `eframe`/`egui` 0.37. Release build from `codex/m0-windows-desktop-completion` in an isolated target directory. The existing `target/release` process was left untouched.
- Each test instance inherited a separate `LOCALAPPDATA` under `.m0-test-*` in the repository. No normal `%LOCALAPPDATA%\ChessChaosLab` save was opened or changed.
- Native interaction and accessibility observations used Windows UI Automation through Computer Use. The committed [v0.1.0 screenshot](../assets/screenshots/desktop-windows-x64.jpg) shows the baseline layout. The observations below are the log for the changed build; the UI Automation tree is the evidence for names, roles, states and game results.
- The public `v0.1.0` ZIP has SHA-256 `88e5a1fdef335cdbfa2b3262ad6ce50b33d5c050f5372b17699e13a1e6ca66d5`. `v0.1.0..HEAD` changed only roadmap/release documentation before this branch, so the baseline application source matches that release tag.

`PASS` below means the specified observation was made in this audit. `Not Verified` never implies a pass.

## Manual test matrix

| Area | Result | Observation |
| --- | --- | --- |
| Human vs human, legal move | PASS | Keyboard Tab reached board; Return selected `f2`, Shift+Tab reached `f4`, Return played `f4`. Status changed to Black to move; FEN and move list updated. |
| Illegal move | PASS | Selected `e2` then clicked illegal `e5`; status stayed White to move and the starting FEN was unchanged. |
| Check and checkmate | PASS | FEN `4k3/8/8/8/8/8/4R3/K7 b - - 0 1` exposed status `Black to move — check`. Plain PGN `1. f3 e5 2. g4 Qh4#` imported; status was `Checkmate — Black wins`. |
| Draw | PASS for insufficient material | Keyboard underpromotion to knight left king+knight versus king; GUI status changed to `Draw by insufficient material`. Claimable draws were not exercised in GUI. |
| Four internal AI policies | PASS | From Black to move after `e4`, GUI returned legal moves: Random `a6`, Capture hunter `Nh6`, Material collector `Nf6`, Alpha-beta depth 3 `Nf6`. |
| AI vs AI full game | PASS | Autoplay ran from the initial position to 44 ply and GUI status `Checkmate — Black wins` (`r3kb2/ppp1p1p1/8/6q1/PP2p2r/8/7K/8 w q - 0 23`). The window remained responsive. |
| FEN import and rejection | PASS | Invalid FEN produced `Invalid FEN: invalid board part in fen` without changing the game. Valid Black-to-move FEN imported and triggered the selected AI. |
| FEN clipboard | PASS | Clicked Copy FEN, replaced the FEN field with `invalid`, then Ctrl+A/Ctrl+V restored the exact starting FEN from the OS clipboard. |
| PGN mainline | PASS | Fool's Mate mainline imported; four SAN moves, final FEN, and winner appeared. `1. e4 {comment} e5` was rejected with the plain-mainline-only message. Advanced PGN remains outside M0. |
| Undo and redo | PASS | Undo from mate returned to Black to move before `Qh4#`; Redo restored checkmate. |
| Resignation and restart | PASS | Resign changed status to `Black resigned`; New game restored the starting position. |
| Promotion | PASS | From `8/P7/8/8/8/8/8/K6k w - - 0 1`, 78 Tabs reached `Promote to knight`; Space selected it. Shift+Tab reached `a7, White pawn`, Return selected it, Up focused `a8, empty, legal destination`, Return produced `a8, White knight` and FEN `N7/8/8/8/8/8/8/K6k b - - 0 1`. |
| Fischer, Bronstein, hourglass | PASS for GUI smoke | A 1+2 Fischer game stored `clock 54924 56963 2000 b 1 f 0` after `e4`. Bronstein's 2-second delay was consumed before White's main time fell (`clock 58264 60000 2000 w 1 b 0`). Hourglass transferred White's spent time to Black, was paused, rejected a move while paused, and reopened at White `00:47`, Black `01:14`, Hourglass, `Resume clock`. Exact long-game forfeits remain covered by core tests. |
| Autosave and abrupt close | PASS | After `e4`, the isolated test process was terminated forcibly; a new instance restored `e4`, Black to move, and the saved FEN. |
| Corrupt autosave | PASS | `invalid autosave` was copied byte-for-byte to `autosave-corrupt-*.txt`; the GUI explained the error. A subsequent `e4` wrote a new autosave while the corrupt copy remained intact. |
| Read-only autosave | PASS | With a test-only read-only `autosave.txt`, Resign changed the in-memory game but the GUI showed `Could not save game: Access is denied. (os error 5)`. The old save remained and `autosave.tmp` held the newer state. |
| Unavailable data path | PASS | With `LOCALAPPDATA` set to a test file instead of a directory, a move showed `Could not save game: Cannot create a file when that file already exists. (os error 183)`. |
| UCI smoke | PASS | Existing subprocess fixture handshake passed. In the release GUI, the local `mock-uci.exe` fixture answered `e4` with `e5`; status returned to White to move, move list showed `1. e4 e5`, and FEN updated. The prior one-move Stockfish 19 smoke is in [testing](testing.md); no full engine qualification is claimed. |

## Native accessibility

| Check | Result | Observation |
| --- | --- | --- |
| Keyboard board control | PASS | Tab/Shift+Tab traversed buttons; arrow keys moved focus between adjacent squares; Return selected and moved. Escape cleared a selected square. No trap was observed in this path. |
| Focus visibility | PASS for board path | Focus outline was visible on the active button in the captured Windows UI; focus order starts with toolbar then 64 board squares then sidebar. Traversing 64 squares is slow. |
| Square names, roles and states | PASS in UI Automation | Before the fix, occupied squares were indistinguishable `WP`/`BP`. After the fix the tree exposed `e7, Black pawn, selected`, `e6, empty, legal destination`, and `e5, empty, legal destination` as buttons. |
| Game status | PASS in UI Automation | Status role/value exposed `Black to move` and `Checkmate — Black wins`; the node is configured as a polite live region. Audible announcement was not verified. |
| Promotion and settings names | PASS in UI Automation | Buttons expose `Promote to queen/rook/bishop/knight`; Minutes, bonus, clock mode, FEN, PGN and engine path controls have associated names. |
| Menu/dialog keyboard paths | Not Verified | Opponent menu options were operable through UI Automation and Space selected the promotion choice; Enter/Space/Escape and focus return for every popup were not fully audited. |
| Screen reader speech (Narrator/NVDA) | Not Verified | Windows Narrator executable exists, but speech output could not be heard or captured through the test interface. NVDA was not found on PATH. UI Automation inspection is not a substitute for a spoken-output test. |
| Board piece-text contrast | PASS calculated | The minimum contrast among the four board fills and both piece text colors is 5.44:1. Other text and focus outlines were not measured. |
| DPI/display scaling | Not Verified | No Windows display-scale change was made, so 125%/150% behavior remains untested. |

## Severity and open gates

- **High, fixed:** indistinguishable occupied square names prevented a screen-reader user from locating a piece. Full square, piece and move-state names now appear in the Windows accessibility tree.
- **High, fixed:** a corrupt autosave was silently ignored and could be overwritten by the next move. It is now copied to a unique backup before a fresh save; saving is disabled if backup fails.
- **Medium, open:** 64 sequential board controls make Tab traversal slow; arrow keys work, but a dedicated board-entry shortcut may improve navigation.
- **M0 gate still open:** actual Narrator/NVDA speech, DPI scaling, and the remaining manual matrix rows need evidence. Keep Issue #5 open and M0 In progress until the gate is met.
