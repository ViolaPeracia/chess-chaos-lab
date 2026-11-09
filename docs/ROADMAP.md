# Roadmap

This roadmap is based on the current source, existing project notes, and the repository setup request dated 9 November 2026. The original Chess Chaos Lab product specification was not present in the workspace; this is **not** a complete trace of that unknown specification. Status means **Completed**, **In progress**, **Planned**, or **Blocked**; a file or TODO alone is never completion evidence.

| Milestone | Status | Dependencies | Acceptance evidence |
| --- | --- | --- | --- |
| M0 — Playable Windows desktop `0.1.0` | Completed | None | 27 automated tests, a Windows GUI audit of the main game and recovery paths, keyboard-only play, Narrator Speech Recap, and 125%/150% display scaling. The [audit](m0-windows-audit.md) records exact observations and remaining scope limits; the public ZIP/checksum and CI establish the original release baseline. |
| M1 — Chess correctness and PGN | In progress | Rules and notation tests | FIDE edge positions, nested PGN round trips and UI/file integration pass locally; PR CI and manual UI acceptance remain to be verified. |
| M2 — Engine reliability | Planned | M1 position correctness | Full real-engine games, malformed output/timeout/crash tests, process cleanup, bounded resources. |
| M3 — Analysis and training | Planned | M2, licensed data | Review and score history, puzzles/openings with provenance and tests. |
| M4 — Advanced modes | Planned | M1/M2, match scheduler | Recoverable AI tournaments and variant-specific rules, notation, and tests. |
| M5 — Personalization and accessibility | Planned | UI interaction audit | Theme/audio preferences, keyboard and screen-reader testing, contrast fixes. |
| M6 — Library and statistics | Planned | Stable save format | Indexed local games, correct real-data statistics and export. |
| M7 — Hardening | Planned | Stable feature paths | Fuzzing, corruption recovery, long-running process tests, profiling. |
| M8 — Distribution | Planned | M0 release pipeline | Installer, signed artifacts if feasible, license manifest, verified Linux build on Linux. |

Current partial implementations are detailed in the [feature matrix](feature-matrix.md). The [published `v0.1.0` release](https://github.com/ViolaPeracia/chess-chaos-lab/releases/tag/v0.1.0) establishes the original Windows package and CI baseline. M0 completion does not create a new release or imply that M1/M2/M5 work is complete. The next path is broader chess/PGN correctness, full UCI matches, and accessibility refinements. No dates are promised for unscoped milestones.
