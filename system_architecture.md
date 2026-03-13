# Project Specification & Architecture: Clutch - A Transmission Remote GUI in Rust

## 1. Project Overview

A cross-platform (Windows, macOS, Linux) desktop application built in pure Rust. The application serves as a remote GUI for a Transmission BitTorrent daemon, communicating exclusively via the Transmission JSON-RPC API. It utilizes the `iced` GUI framework, adhering strictly to the Elm architecture (Model, View, Update).

## 2. Core Specification

### Functional Requirements

- **Connection Management:** Configure and connect to a remote Transmission daemon (Host, Port, RPC Path, Username, Password). Handle the `X-Transmission-Session-Id` header lifecycle.
- **Torrent Management:**
  - **Add:** Support adding torrents via local `.torrent` files (Base64 encoded) and Magnet links.
  - **Delete:** Remove torrents (with the option to delete local data).
  - **Update/Control:** Pause, resume, and verify local data for selected torrents.
- **Master-Detail User Interface:**
  - **Master View (List):** A scrollable list displaying active and inactive torrents. Columns must include: Name, Status (Downloading, Seeding, Paused), Progress (%), Download Speed, Upload Speed, Seeders, and Leechers.
  - **Detail View (Inspector):** A panel that appears when a torrent is selected, showing comprehensive details (trackers, peers, file lists, exact downloaded sizes, ETA).

### Non-Functional Requirements

- **Performance:** UI must remain responsive at 60fps. Networking and disk I/O must not block the main UI thread.
- **Cross-Platform:** Must compile natively for Windows, macOS, and Linux without relying on heavy C-bindings (like GTK) or web views.

## 3. System Architecture

The application will follow `iced`'s Elm-inspired architecture, neatly separating the state, the presentation, and the background asynchronous tasks.

### 3.1 The Elm Loop (`iced::Application`)

1.  **State (Model):** A single source of truth containing the list of torrents, connection settings, UI state (selected torrent), and network loading statuses.
2.  **View:** A pure function that takes the current State and returns an `iced::Element` (the widget tree). It maps user interactions (clicks, text input) to `Message`s.
3.  **Message:** An `enum` defining every possible event in the application (e.g., `Tick`, `TorrentSelected`, `AddTorrent`, `RpcResponseReceived`).
4.  **Update:** A function that takes the current State and a Message, mutates the State accordingly, and optionally returns a `Command` (an asynchronous side effect, like making an HTTP request).

### 3.2 Concurrency & Background Polling

To keep the data fresh without freezing the UI, the app will use `iced::Subscription` and async `Command`s:

- **Polling Subscription:** An `iced::time::every` subscription will emit a `Message::Tick` every 1-2 seconds.
- **RPC Dispatcher:** When `Update` receives a `Tick`, it returns an asynchronous `Command` that uses `reqwest` to fetch `torrent-get` from the daemon.
- **State Reconciliation:** Once the async `Command` resolves, it yields a `Message::TorrentsUpdated(Vec<Torrent>)`, which the `Update` function uses to seamlessly overwrite or patch the local state.

---

## 4. Core Data Models

### The Application State

```rust
struct AppState {
    // Daemon connection details
    credentials: TransmissionCredentials,

    // Core Data
    torrents: HashMap<i64, TorrentData>,

    // UI State
    selected_torrent_id: Option<i64>,
    is_loading: bool,
    error_message: Option<String>,
}
```

### The Message Enum

```rust
#[derive(Debug, Clone)]
enum Message {
    // UI Events
    TorrentSelected(i64),
    PauseTorrentClicked(i64),
    ResumeTorrentClicked(i64),
    AddTorrentFile,
    AddMagnetLink(String),

    // Background / Time Events
    Tick(std::time::Instant),

    // Async Results from RPC Calls
    RpcTorrentsFetched(Result<Vec<TorrentData>, RpcError>),
    RpcActionCompleted(Result<(), RpcError>),
    FileSelected(Option<std::path::PathBuf>),
}
```

## 5. UI Layout Structure (The View)

Because iced does not have a native, complex data grid with resizable columns out-of-the-box, the Master list will be constructed using a Scrollable containing a Column of Rows.

- Main Window: A standard vertical Column.
- Top Bar (Toolbar): A Row containing buttons for Add, Pause, Resume, Delete, and Settings.
- Center Area (Splitter):
  - Master (Top 60%): A Scrollable. The first Row acts as the header (Name, Size, Progress, etc.). Subsequent Rows represent individual torrents, mapped from the torrents HashMap. Clicking a row emits Message::TorrentSelected(id).
  - Detail (Bottom 40%): A conditional rendering block. If selected_torrent_id is Some(id), display a tabbed interface (using a row of buttons to toggle active views) showing General Info, Trackers, Peers, and Files.

## 6. Recommended Crate Stack

To fulfill this specification in pure Rust, include the following in your Cargo.toml:

- iced: The core GUI framework (enable features: tokio, canvas, image).

- tokio: For the underlying asynchronous runtime.

- reqwest: For making HTTP POST requests to the Transmission RPC endpoint. Enable the json feature.

- serde & serde_json: For defining structs that map to Transmission's RPC payloads and responses.

- rfd (Rust File Dialog): A 100% pure Rust file dialog crate for the "Add .torrent File" feature.

- base64: For encoding the .torrent file contents before sending them to the daemon.
