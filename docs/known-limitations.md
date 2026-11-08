# Known limitations

- Fischer, Bronstein and hourglass clocks are available; other requested clock modes remain. There is no full PGN parser, game library, tournaments, training, opening database, statistics, audio, achievements, or chess variants.
- The four internal AI policies are basic; the depth-three alpha-beta policy has no strength calibration. They are not Elo-rated.
- UCI supports one-shot analysis and playing White against one local engine. Stockfish 19 passed a one-move game smoke test and one-second analysis, but a full match, real-engine failure cases, and process resource isolation remain untested or absent.
- Chess960 castling and variant rules are not exposed. Standard castling is selectable by the king destination.
- Draw logic needs more FIDE edge-case tests; draw agreement is absent. Time forfeits are implemented, including a lone-king draw test.
- Autosave stores one game and optional clock. Corrupt files are ignored, without a recovery UI or backup. A running clock resumes paused after restart. Undo/redo is disabled during timed games; redo moves in untimed games are lost after restart.
- PGN import supports only one plain mainline, no comments, variations, NAGs, or batch import. PGN headers are not fully validated.
- UI uses text piece tokens and two themes. Windows accessibility controls are exposed, but no screen-reader or keyboard-only audit has been done.
- Linux build, installer, offline vendor bundle, cargo advisory audit, and broader UI/AI performance benchmarks are pending. A perft microbenchmark has been run.

