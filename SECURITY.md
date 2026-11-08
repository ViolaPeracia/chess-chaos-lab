# Security policy

This is an early local desktop application. Supported release line: `0.1.x` when a GitHub release is published. The app launches user-supplied UCI executables with the current user's permissions; only run engines you trust. There is no process sandbox. The source has no intended runtime Internet feature, but continuous packet capture has not verified network silence. Build-time Cargo downloads may occur.

Report a suspected vulnerability through GitHub's **Report a vulnerability** private advisory form for this repository. Include affected version, reproduction steps, and impact. Please do not disclose an exploitable issue in a public issue before the maintainer can assess it. See [security notes](docs/security.md) and [offline behavior](docs/offline-mode.md).
