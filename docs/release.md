# Release status

Version `0.1.0` is an early playable Windows x64 build. A local ZIP staged in `dist/` is separate from any GitHub Release; never reuse a checksum for a new artifact. Check the [Releases page](https://github.com/ViolaPeracia/chess-chaos-lab/releases) for the public package and its checksum.

The [release workflow](../.github/workflows/release.yml) is triggered only by a `vX.Y.Z` tag. It checks that the tag matches the Cargo version and points at the default branch, runs formatting, Clippy, tests and a release build on Windows, packages the executable with the existing GPL license, README, guide and source, computes a fresh SHA-256, creates a draft release, checks the uploaded asset names, and publishes it. A normal push or documentation edit never creates a release. The generated notes and package contents must be reviewed before tagging.

To prepare a release, merge the version and changelog updates, wait for required CI, then tag the intended commit with `vX.Y.Z` and push that tag. Do not change published release assets; issue a new patch version for corrections. There is no installer, signed binary, Linux artifact, or vendored dependency archive. See [testing](testing.md) and [limitations](known-limitations.md).
