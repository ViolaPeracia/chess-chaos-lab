# Feature matrix

Status is based on code and tests in this workspace. “Partial” is not release acceptance.

| Feature | Priority | Module | Dependencies | Status | Acceptance criteria | Test evidence | Reference |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Standard legal chess | Essential | core | shakmaty | Implemented | Legal moves and mate/stalemate work | `initial_perft`, `scholar_mate_undo_redo` | architecture |
| Castling, en passant, promotions | Essential | core/UI | shakmaty | Implemented | All cases playable and regression tested | Castle-through-check, en-passant discovered-check, underpromotion and perft position tests; keyboard underpromotion passed in GUI | [M0 audit](m0-windows-audit.md) |
| Draw rules | Essential | core/UI | shakmaty | Partial | Correct claimable and automatic outcomes | Stalemate, insufficient material, threefold, fivefold, 50/75, checkmate priority, announced-move claim and time-forfeit tests; announced-move claim lacks UI | known limitations |
| Human vs human | Essential | app | core | Implemented | Two local sides can move | Core move sequence test; GUI not automated | user guide |
| Four internal AI policies | High | ai/app | core | Implemented | Each selects legal move | `every_style_returns_legal_move` | user guide |
| Strong internal engine | High | ai | search/evaluation | Not implemented | Measured strength and tactical coverage | None | roadmap |
| AI vs AI autoplay | High | app | ai | Partial | Full match with pause/replay | 44-ply complete GUI match and 100-ply simulation; pause/replay not verified | [M0 audit](m0-windows-audit.md) |
| External UCI analysis | High | uci/app | local engine executable | Partial | Real engine handshake, analysis, timeout | Parser, fixture round trip, Stockfish 19 GUI analysis; timeout not exercised with real engine | engine integration |
| Play White vs local UCI engine | High | uci/app | local engine executable | Partial | Legal engine replies, errors and timeouts in a real match | GUI `e4 e5` with fixture and Stockfish 19; no full match | engine integration |
| FEN import/export | Essential | core/app | shakmaty | Implemented | Invalid positions rejected | `fen_rejects_illegal_position` | user guide |
| PGN import/export | Essential | core/app | SAN | Partial | Full PGN with variations/comments and result | Nested RAV, headers, Unicode, NAG, FEN and autosave round trips; file import and UI rows tested in code; clipboard workflow, manual GUI and CI acceptance pending | known limitations |
| Autosave/recovery | Essential | app/core | local filesystem | Partial | Crash-safe, corruption recovery | Abrupt-close GUI recovery and corrupt-save backup passed in isolation; write errors are visible; failure paths and recovery UI remain limited | [M0 audit](m0-windows-audit.md) |
| Time controls | Essential | core/app | clocks | Partial | Fischer/delay/hourglass/forfeits and recovery | Unit tests for all three; isolated GUI smoke for each mode and paused hourglass restart; other modes absent | [M0 audit](m0-windows-audit.md) |
| Chess960/variants | High | core | variant rules | Not implemented | Rules, notation, tests and compatible AI | None | roadmap |
| Tournaments/council | Advanced | future | engine scheduler | Not implemented | Reproducible results and recovery | None | roadmap |
| Analysis/review | High | future | UCI | Partial | Live evaluation and reviewed moves | One-shot UCI adapter only | engine integration |
| Puzzles/openings/training | High | future | licensed data | Not implemented | Real data and scoring | None | roadmap |
| Themes/accessibility | High | app | egui | Partial | Usable themes, keyboard and contrast checks | Keyboard move, Narrator speech, menu focus, and 125%/150% scaling passed; board text contrast ≥5.44:1; broader M5 audit remains | [M0 audit](m0-windows-audit.md) |
| Statistics/library | Advanced | future | storage | Not implemented | Real local dataset and filters | None | roadmap |
| Audio/achievements | Fun | future | licensed assets | Not implemented | Functional and locally stored | None | roadmap |
| Offline runtime | Essential | all | local assets | Implemented by design | No runtime network code | Code inspection; packet-level test pending | offline mode |
