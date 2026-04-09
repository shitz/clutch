# Clutch — System Architecture

Clutch is a cross-platform desktop application built in pure Rust. It serves as a remote GUI for a Transmission BitTorrent daemon, communicating via the Transmission JSON-RPC API.

The application is built using the [`iced`](https://github.com/iced-rs/iced) GUI framework and
strictly follows the **Elm architecture** (`State`, `View`, `Message`, `Update`).

## Core Invariants

To keep the application fast and predictable, we maintain a few strict rules:

- **Non-blocking UI:** The `update()` function must return in microseconds. All I/O, crypto, and
  network calls are offloaded to asynchronous `Task`s or background subscriptions.
- **Serialized RPC:** To prevent overwhelming the Transmission daemon, all RPC calls flow through a
  single `tokio::sync::mpsc` channel. There is at most _one_ HTTP request in-flight at any time.
- **Screen-safe State:** Torrent data is only accessible when `Screen::Main` is active. Illegal UI
  states (like viewing torrents while disconnected) are unrepresentable by the type system.

## High-Level App Routing

The application state (`AppState`) acts as a router. It runs exactly one screen at a time, modelled as a discriminated enum:

```rust
pub enum Screen {
    Connection(ConnectionScreen),  // Startup launchpad & saved profiles
    Main(MainScreen),              // Active torrent list + inspector
    Settings(SettingsScreen),      // Profile and app configuration
}
```

**Message Dispatch:** Each screen has its own Message enum. The top-level `app::Message` wraps these
and routes them through `app.rs`, which delegates startup/global flow, settings-result reconciliation,
and screen-specific keyboard subscriptions to private `src/app/` helpers.

## The RPC Layer & Data Flow

Communication with the Transmission daemon is handled in the `src/rpc/` module using a shared
`reqwest::Client`.

Because Transmission's RPC handler can wedge under concurrent load, Clutch serializes all requests.

1. The UI dispatches an RpcWork item.
1. It enters a `tokio::sync::mpsc::channel`.
1. A background worker loop (`iced::Subscription`) processes the queue one by one.
1. The worker automatically handles session rotation (HTTP 409) and retries.
1. Results are emitted back to the UI as `Messages`.

## Security & Profile Storage

Connection profiles (host, port, username, password) are persisted locally at
dirs::config_dir()/clutch/config.toml.

- Passwords are encrypted directly inside the TOML file.
- We use Argon2id for Key Derivation and ChaCha20-Poly1305 for AEAD encryption (src/crypto.rs).
- The unlocked master passphrase is held in a secrecy::SecretString which automatically zeroizes its backing memory when dropped. Crypto operations are spawned on blocking threads to avoid stalling the UI.

## Client-Side Filtering

The torrent list supports multi-select status filtering entirely in-process. A
`HashSet<StatusFilter>` in `TorrentListScreen` drives two passes on every `view()` call:

1. A **count pass** over the full `Vec<TorrentData>` tallies how many torrents belong to
   each of the five semantic buckets (Downloading, Seeding, Paused, Active, Error).
2. A **filter pass** retains only torrents that match the active `HashSet` before the rows are
   rendered.

Both passes inline the status-matching logic directly rather than allocating intermediate
collections, keeping the render hot path allocation-free on the per-torrent level.

No additional RPC calls are needed; the daemon continues to return the full torrent list on every
poll tick.

## Directory Structure

A quick map of where to find things:

```text
src/
├── main.rs          # Entry point, window constraints, tracing setup
├── app.rs           # AppState, Screen router, top-level facade for update/view
├── app/             # Private routing, settings bridge, and keyboard helpers
├── rpc/             # Transmission API, types, and the serialized worker
├── screens/         # Individual screen states and views
│   ├── connection.rs        # Profile selection and quick-connect
│   ├── main_screen.rs       # Root layout: torrent list + inspector split
│   ├── inspector/           # Detail inspector panel (state / update / view)
│   ├── torrent_list/        # Torrent list (filters, toolbar, columns, dialogs, sort, add-torrent, update, worker)
│   └── settings/            # Profile editing (state / draft / update / view)
├── auth/            # Passphrase setup and unlock flows (update / view)
├── crypto.rs        # Argon2id / ChaCha20-Poly1305 wrappers
├── profile.rs       # TOML storage and runtime configuration
├── theme.rs         # Public Material Design 3 theme facade
├── theme/           # Private widget/style helpers backing crate::theme
└── format.rs        # String formatting for ETA, bytes, speeds
```
