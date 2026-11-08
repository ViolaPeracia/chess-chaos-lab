# Build

Verified environment on 2026-11-07: Windows 11 x64 (build 26200), Rust/Cargo 1.99.1, rustfmt, Clippy, Git, Visual Studio 18 Build Tools, Windows SDK 10.0.26100.0. `rustup` and `cargo-audit` were absent from PATH. Graphics devices: AMD Radeon 740M and NVIDIA GeForce RTX 3050 Laptop GPU. The app uses the OpenGL/glow backend; no discrete GPU is required by design, but low-end hardware has not been benchmarked.

```powershell
cargo build --release --locked
```

Run `target/release/chess-chaos-lab.exe`. Dependencies are locked in `Cargo.lock`. For a machine with the dependencies already cached, `cargo build --release --locked --offline` uses no registry network. For distributable offline source builds, run `cargo vendor vendor` on a connected development machine, place the generated directory with the source, and configure `.cargo/config.toml` to replace `crates-io` with `vendor` as printed by Cargo. Vendor packaging has not yet been verified here.

To rebuild the Windows portable ZIP from the current source, run `powershell -File scripts/package.ps1`. The script builds the release executable, stages the executable, license, guide, documentation and source, then writes the ZIP and its SHA-256 checksum under `dist/`. The ZIP is not an offline source build because third-party Cargo dependencies are not vendored.

Linux x64 is a target, not a verified build; it needs a Linux host and native windowing/OpenGL development packages. No installer or AppImage is provided yet.
