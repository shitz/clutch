<p align="center">
  <img src="assets/Clutch_Logo.png" alt="Clutch" width="300"/>
</p>

<p align="center">
  <img src="assets/screenshot_connection_setup_light.png" alt="Connection setup" width="32%"/>
  <img src="assets/screenshot_main_dark.png" alt="Main torrent list" width="32%"/>
  <img src="assets/screenshot_settings_dark.png" alt="Settings" width="32%"/>
</p>

A desktop GUI for [Transmission](https://transmissionbt.com/) built in Rust using
[iced](https://github.com/iced-rs/iced). Connects to a remote Transmission daemon via its JSON-RPC
API.

## Features

- **Torrent list** — sortable columns (name, size, progress, ETA, speeds, ratio, status), responsive
  even with hundreds of torrents
- **Torrent detail inspector** — General, Files, Trackers, and Peers tabs
- **Add torrents** — by magnet link or `.torrent` file
- **Start / stop / remove** torrents
- **Multiple connection profiles** — save and switch between Transmission instances
- **Credential encryption** — passwords are encrypted at rest using Argon2id + ChaCha20-Poly1305
- **Theme** — light, dark, and system-follow modes (Material Design 3)
- **Cross-platform** — macOS, Linux, and Windows (no GTK, no web views, pure Rust)

## Why

The available remote Transmission GUI clients are either dated in design or not actively maintained.
[Remote Transmission GUI](https://github.com/transmission-remote-gui/transgui) works but looks its
age. Clutch was built to be a clean, fast alternative that stays responsive with large torrent
libraries.

## Installation

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)

### Build from source

```sh
git clone https://github.com/shitz/clutch.git
cd clutch
cargo build --release
./target/release/clutch
```

## How it was built

Clutch was developed using a heavily AI-assisted workflow:

- Each feature was modelled as a structured specification using [OpenSpec](https://openspec.dev/),
  with design docs, specs, and task breakdowns tracked in the repository under `openspec/changes/`
  and `openspec/specs/`.
- Implementation was done almost entirely with **Claude Sonnet 4.6** as the coding agent.
- Code review was performed by **Gemini 3.1 Pro** and the author.

All OpenSpec artifacts (specs, changes, and archives) are included in this repository, so you can
trace the design rationale for any feature.

The architecture is documented in [system_architecture.md](system_architecture.md).

## Contributing

Bug reports and feature requests are welcome — please open an issue.

Pull requests are also welcome. A few ground rules:

- **Disclose AI use.** If a PR was built with AI assistance, say so: include the model(s) used, the
  prompts or agentic workflow, and any relevant context.
- **Bring specs for larger changes.** A non-trivial feature should ideally come with the associated
  OpenSpec artifacts (design doc, spec, task list) so the intent is clear and reviewable.
- For smaller fixes and tweaks, a clear description of the problem and solution is enough.

## License

Clutch is licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.
