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
and routes them to the active screen's `update()` function, handling global intercepts (like opening
settings or locking the app) at the top level.

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
2. A **filter pass** retains only torrents whose `matching_filters()` set intersects the active
   `HashSet` before the rows are rendered.

No additional RPC calls are needed; the daemon continues to return the full torrent list on every
poll tick.

## Directory Structure

A quick map of where to find things:

```text
src/
├── main.rs          # Entry point, window constraints, tracing setup
├── app.rs           # AppState, Screen router, top-level update/view
├── rpc/             # Transmission API, types, and the serialized worker
├── screens/         # Individual screen states and views
│   ├── connection/  # Profile selection and quick-connect
│   ├── main_screen/ # The core app: torrent list and detail inspector
│   └── settings/    # Profile editing and app preferences
├── auth.rs          # Passphrase setup and unlock dialogs
├── crypto.rs        # Argon2id / ChaCha20 wrappers
├── profile.rs       # TOML storage and runtime configuration
├── theme.rs         # Material Design 3 palettes and widget styling
└── format.rs        # String formatting for ETA, bytes, speeds
```
