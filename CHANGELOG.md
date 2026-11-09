# Changelog

## Unreleased

- Add FIDE edge-position regression tests, referenced perft positions, and a typed game outcome separate from draw-claim eligibility.
- Add bounded single-game PGN import/export with headers, comments, NAGs, nested variations, starting FEN, tree navigation, and autosave preservation. PGN can be pasted or loaded from a file path; the full tree is copied to the clipboard.
- Reject PGN moves after automatic fivefold repetition, preserve results from legacy terminal saves, and keep annotated move rows aligned while scrolling large variation trees.

## 0.1.0 — 2026-11-08

Initial playable Windows build: standard chess, local AI policies, Fischer clocks, FEN, limited PGN, autosave, and local UCI analysis adapter. This is a development release; see `docs/known-limitations.md`.
