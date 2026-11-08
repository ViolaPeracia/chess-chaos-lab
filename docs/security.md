# Security review

Trust boundaries are FEN/PGN text, autosave data and a user-selected UCI executable. FEN is parsed and validated by `shakmaty`. Plain PGN and save input are capped at 1 MB. The engine is launched by exact path with `Command`, without a shell; protocol stages have timeouts and the child is killed afterward. No network API or login exists.

Remaining risks: no process sandbox or CPU/RAM quota; a hostile engine can emit very long lines before timeout; save rename durability has not been crash-tested; corrupted autosave is silently ignored; dependency advisory audit was unavailable because `cargo-audit` is not installed. The app should not be described as security-audited or release-hardened.

