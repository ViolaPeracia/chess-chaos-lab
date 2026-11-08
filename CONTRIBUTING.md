# Contributing

Open an issue before taking on a large feature. Check the [roadmap](docs/ROADMAP.md) and [feature matrix](docs/feature-matrix.md) so a partial feature is not mistaken for a finished one. For bugs, include a legal FEN or minimal PGN, expected behavior, actual behavior, OS, and engine details where relevant. Do not post secrets or a private engine executable.

Make focused changes on a branch and open a pull request against `main`. Use a Conventional Commit message such as `fix(core): correct repetition outcome`. Update the corresponding guide when behavior changes. Run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

For release or packaging changes, also run `cargo build --release --locked`. Windows x64 is the verified platform; other platforms need their own evidence. See [AGENTS.md](AGENTS.md) for repository conventions and [SECURITY.md](SECURITY.md) for vulnerability reports. The repository retains the existing GPL-3.0-or-later declaration; contributions are made under that project license.
