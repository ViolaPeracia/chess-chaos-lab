# Offline operation and data

Gameplay, AI, rules, save/load and GUI run locally. The runtime source contains no network client, telemetry, analytics, login or cloud integration. Dependency downloads happen only during development/build. The final executable was inspected while running: no TCP connections or UDP endpoints were returned for its process at that instant. Continuous packet capture has not been run.

On Windows the app stores one autosaved game at `%LOCALAPPDATA%\ChessChaosLab\autosave.txt`. It writes a temporary file and renames it to the save path. The file contains FEN, UCI moves, outcome and an optional clock line; no personal profile is collected. Timed games restart paused. Back up or delete this file manually. Corrupted save files are ignored at startup; backup and in-app deletion/export controls are not implemented yet.

