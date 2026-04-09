# Agent Guidelines for Clutch

Clutch is a cross-platform desktop BitTorrent client written in pure Rust, built on the
[iced](https://github.com/iced-rs/iced) GUI framework and strictly following the **Elm
architecture**. Read `system_architecture.md` for the high-level design before making
non-trivial changes.

---

## Quality Gates

After every major change, all three commands **must** pass without warnings or errors:

```sh
cargo fmt
cargo check
cargo clippy -- -D warnings
cargo test
```

Run them in that order. Fix every warning before moving on — the CI pipeline treats
warnings as errors.

---

## License Header

Every `.rs` source file must start with the Apache 2.0 header **exactly** as shown:

```rust
// Copyright 2026 The clutch authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
```

Never create a new file without this header.

---

## Code Conventions

- Edition: **Rust 2024**. Use its idioms (e.g. `use<'a>` for precise capturing).
- Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification for commit
  messages.
- Keep `update()` functions **non-blocking**: every network call, file I/O, and crypto
  operation must be dispatched via `iced::Task::perform()` or a background subscription.
  Blocking in `update()` is a bug.
- All RPC calls go through the single `mpsc` queue in `src/rpc/worker.rs`. Never issue an
  HTTP request directly from `update()` or `view()`.
- Represent illegal UI states as unrepresentable types — see the `Screen` enum in `app.rs`.
- Use `secrecy::SecretString` / `SecretBox` for all in-memory credentials. Never store raw
  passwords in plain `String`s.

---

## Theme & UI Elements (`crate::theme`)

All visual styling lives in the public `crate::theme` module, implemented by
`src/theme.rs` plus private `src/theme/*` helpers. Import only from `crate::theme`
— never hard-code colors, fonts, or sizes inline.

### When to use each helper

| Helper                              | Where to use                                                                                                                                                                              |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `theme::clutch_theme(is_dark)`      | Pass to `iced::application().theme()` in `main.rs`. Do **not** call elsewhere.                                                                                                            |
| `theme::icon(codepoint)`            | Render any Material Icons glyph (24 px). Use the named `ICON_*` constants, never raw codepoints inline.                                                                                   |
| `theme::icon_button(content)`       | Every icon-only toolbar button (pause, play, delete, settings, …). Gives a 36×36 circular hover target. Always attach `.on_press(msg)` — without it the button is visually disabled.      |
| `theme::m3_primary_button`          | The **primary CTA** of a screen or dialog (e.g. "Connect", "Save"). Solid brand-blue fill. Use only once per view context.                                                                |
| `theme::m3_tonal_button`            | **Secondary actions** that are important but not the primary CTA (e.g. "Test Connection", "Undo"). Lighter brand wash.                                                                    |
| `theme::segmented_control(...)`     | Mutually exclusive toggle groups (e.g. filter by status, sort direction). Pass `equal_width = true` when the control spans a fixed container; `compact = true` in space-constrained rows. |
| `theme::m3_card(theme)`             | Card-style containers: elevated surface with 16 px radius and a subtle drop shadow. Use inside the inspector panel and settings panels.                                                   |
| `theme::inspector_surface(theme)`   | The inspector panel's own background container. Rounded top corners, no bottom rounding — designed to sit flush against the bottom edge.                                                  |
| `theme::selected_row(theme)`        | The `container` wrapping a highlighted torrent row in the list. Apply only to the selected torrent; all other rows get no explicit style.                                                 |
| `theme::m3_tooltip(theme)`          | `iced_aw::Tooltip` containers. Dark contrasting surface, legible in both modes.                                                                                                           |
| `theme::m3_text_input`              | All `text_input` widgets via `.style(theme::m3_text_input)`. Provides M3 outlined-field visuals with active/hovered/focused/disabled states.                                              |
| `theme::progress_bar_style(status)` | `progress_bar` widgets in torrent rows. Pass the Transmission status code; the function returns the correct color (green = downloading, blue = seeding, gray = paused/other).             |

### Icon constants

Use the named constants — never embed raw Unicode codepoints in view code:

```text
ICON_PAUSE, ICON_PLAY, ICON_DELETE, ICON_ADD, ICON_LINK,
ICON_LOGOUT, ICON_DOWNLOAD, ICON_UPLOAD, ICON_SETTINGS, ICON_CLOSE,
ICON_SAVE, ICON_UNDO, ICON_TRASH
```

### Dark / light mode detection

Inside a style closure you already receive `&Theme`. To branch on dark vs. light mode use:

```rust
let is_dark = theme.extended_palette().background.base.color.r < 0.5;
```

Do **not** keep a separate `bool` flag for this purpose.

---

## Screen & Module Layout

```text
src/
├── main.rs          # Entry point only — window setup, font registration, tracing init
├── app.rs           # AppState, Screen router, top-level facade for update/view/subscription
├── app/             # Private routing, settings-bridge, and keyboard helpers backing crate::app
├── rpc/             # Transmission RPC: api, models, transport, worker (serialized queue)
├── screens/
│   ├── connection.rs        # Profile selection / quick-connect
│   ├── main_screen.rs       # Root layout: torrent list + inspector split
│   ├── inspector.rs         # Detail inspector panel
│   └── torrent_list/        # List screen: view orchestration, toolbar/header/dialog helpers, sort, add-torrent, update, worker
│   └── settings/            # Profile editing (split into state/draft/update/view)
├── auth.rs          # Passphrase setup and unlock flows
├── crypto.rs        # Argon2id KDF + ChaCha20-Poly1305 AEAD
├── profile.rs       # TOML config persistence
├── theme.rs         # Public theme facade (colors, fonts, widget helpers)
├── theme/           # Private theme submodules backing crate::theme
└── format.rs        # Human-readable formatting (sizes, speeds, ETA)
```

When adding a new screen:

1. Mirror the `state / update / view` split used by `screens/settings/`.
2. Add a variant to the `Screen` enum in `app.rs`.
3. Route messages through `app::update`.

---

## Commit Messages

Follow the Go commit style with a component prefix:

```text
component: short imperative summary

Optional body explaining *why*, not *what* (the diff shows that).
```

Component examples: `clutch/theme`, `clutch/rpc`, `clutch/torrent-list`, `clutch/settings`,
`clutch/ci`, `clutch/packaging`.

---

## Security

- Never log or `println!` credentials, session tokens, or encryption keys.
- Crypto operations (Argon2, ChaCha20) must run on a blocking thread
  (`tokio::task::spawn_blocking`) — not the async executor.
- Input from RPC responses is untrusted. Use typed deserialization (`serde`) and never
  pass raw JSON strings to `eval`-style APIs.
- Follow OWASP Top 10 guidelines. Any dependency that handles auth, crypto, or network
  must be reviewed before adding.

---

## Adding Dependencies

- Prefer crates already in `Cargo.toml` before adding new ones.
- Security-sensitive crates (crypto, TLS, auth) require an explicit rationale in the PR
  description.
- Run `cargo audit` if adding or updating a security-relevant crate.

---

## Testing

- Unit tests live in the same file as the code they test (`#[cfg(test)]` module).
- Integration tests for RPC go in `src/rpc/` and may use `wiremock` (already in
  `[dev-dependencies]`) to mock the Transmission daemon.
- Tests must not perform real network I/O, disk writes to production paths, or shell out.

## Documentation

Keep the system_architecture.md and README.md up to date with any architectural changes or new
features. The README should be a user-friendly overview, while system_architecture.md can go into
technical depth for future maintainers. Change logs must be recorded in CHANGELOG.md following the
[Keep a Changelog format](https://keepachangelog.com/en/1.0.0/).

## Markdown formatting

- Use fenced code blocks with language tags for all code snippets.
- Use relative links for internal references (e.g. `[system architecture](system_architecture.md)`).
- Line length should be at most 100 characters for readability on narrow viewports.
