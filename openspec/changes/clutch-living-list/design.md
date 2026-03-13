## Context

Clutch is a greenfield native desktop app for controlling a Transmission BitTorrent daemon. The primary motivation is eliminating UI-thread blocking — the defining flaw of existing Transmission GUIs. The tech stack is pure Rust: `iced` for the GUI (Elm architecture), `reqwest` + `tokio` for async networking, `serde_json` for RPC payload handling.

This design covers the v0.1 "living list" slice: a working connection flow and auto-refreshing torrent list.

## Goals / Non-Goals

**Goals:**

- Screen-based app model that makes illegal UI states unrepresentable
- All network I/O inside `Command::perform()` — `update()` is always non-blocking
- Transmission session-id lifecycle handled transparently by the RPC client
- Connection screen with retry on failure
- Auto-refreshing torrent list (polling every 5 seconds)
- Sticky column header above a scrollable torrent list

**Non-Goals:**

- Torrent actions (pause, resume, delete, add) — buttons rendered but disabled
- Detail/inspector panel
- Settings persistence (connection form is in-memory only)
- Resizable panels
- Error recovery beyond returning to the connection screen

## Decisions

### 1. Screen enum as top-level state router

**Decision:** `App` holds a single `Screen` enum rather than a flat `AppState`.

```
enum Screen {
    Connection(ConnectionScreen),
    Main(MainScreen),
}
```

**Rationale:** Makes illegal states unrepresentable at the type level. If `Screen::Connection`, there is no torrent list. If `Screen::Main`, there is no connection form. Adding future screens (settings, torrent detail overlay) slots in cleanly.

**Alternatives considered:** Flat `AppState` with `Option` fields — rejected because it allows contradictory state (e.g., torrents populated while showing the connection form) that requires defensive checks everywhere.

---

### 2. RPC client as a pure async module, not a struct

**Decision:** The RPC layer is a module of free async functions (`rpc::session_get(...)`, `rpc::torrent_get(...)`) that accept credentials and a session-id, and return `Result<T, RpcError>`. The caller (inside `Command::perform()`) owns the retry logic.

**Rationale:** `iced` `Command`s are fire-and-forget closures. There's no natural place to attach a shared stateful client. Passing credentials and session-id explicitly keeps the functions pure and testable.

**Session-id retry pattern:**

```
async fn call_with_retry(credentials, session_id, payload) -> Result<T, RpcError> {
    let resp = post(credentials, session_id, payload).await?;
    if resp.status == 409 {
        let new_id = resp.headers["X-Transmission-Session-Id"];
        // caller receives Err(RpcError::SessionRotated(new_id))
        // update() stores new_id and re-fires the command
    }
    ...
}
```

The session-id lives in `MainScreen` state and is updated via `Message::SessionIdRotated(String)`.

**Alternatives considered:** A `reqwest::Client` stored in `AppState` with middleware — rejected because `iced`'s `Command` closures require `'static` bounds, making shared mutable client awkward without `Arc<Mutex<>>`.

---

### 3. Connection probe via `session-get`

**Decision:** Clicking "Connect" fires a `session-get` RPC call (not `torrent-get`).

**Rationale:** `session-get` is lightweight (no torrent data) and sufficient to prove connectivity, authentication, and trigger the 409/session-id dance. On success, a `torrent-get` is immediately chained. Error classification (`connection refused` vs `401 Unauthorized`) drives distinct error messages on the connection screen.

---

### 4. Conservative concurrency: one RPC call at a time

**Decision:** `MainScreen` tracks `is_loading: bool`. Polling ticks are ignored while a call is in-flight.

**Rationale:** Eliminates session-id race conditions (two concurrent 409s both trying to rotate state). For v0.1 with no user actions, this is indistinguishable from concurrent. Revisit when actions are added.

---

### 5. Sticky header via layout split

**Decision:** The column header row lives _outside_ the `scrollable` widget, immediately above it. Column widths are synchronized using `Length::FillPortion`.

**Rationale:** `iced`'s `scrollable` scrolls all its children. The only way to achieve a sticky header is to keep it outside the scroll region. `FillPortion` weights must match between the header row and each data row.

---

### 6. Deserialization on the tokio thread

**Decision:** JSON parsing of `torrent-get` responses happens inside `Command::perform()`, never in `update()`.

**Rationale:** `serde_json` deserialization of large torrent lists is CPU-bound and could introduce frame drops if called synchronously in `update()`. The parsed `Vec<TorrentData>` (or `RpcError`) is the message payload.

---

### 7. RPC functions accept the full URL, not credentials-derived

**Decision:** All RPC functions (`session_get`, `torrent_get`, `post_rpc`) accept a `&str` or `Url` as their first argument rather than constructing the URL internally from `TransmissionCredentials`.

**Rationale:** Testability. Integration tests using `wiremock` spin up a local HTTP server at a random port. If the URL is hardcoded from credentials, there is no seam to inject the test URL. Production code constructs `http://{host}:{port}/transmission/rpc` at the call site before invoking the RPC function.

**Alternatives considered:** A trait abstraction (`RpcTransport`) — rejected as over-engineering for v0.1; accepting a URL is sufficient and keeps functions as plain `async fn`s.

---

### 8. Testing strategy

**Decision:** Three-layer approach:

1. **RPC client — integration tests with `wiremock`**
   Spin up a local HTTP stub server per test. Stub exact Transmission responses (409 with `X-Transmission-Session-Id` header, 401, well-formed 200 JSON). Tests exercise the full HTTP path including header extraction and error classification. Add `wiremock` as a `[dev-dependency]`.

2. **`update()` logic — pure unit tests**
   `update()` is a pure function: state × message → (state, command). Test state transitions directly with `#[test]` blocks and no async runtime. Key cases: tick debouncing (`is_loading = true` → no command), session-id rotation, error propagation from RPC results.

3. **View layer — manual only**
   `iced` has no headless renderer. Visual correctness is verified by running the app against a real or mock daemon.

---

### 9. Documentation convention

**Decision:** All modules, public structs, enums (and their variants), and all public or non-trivial private functions SHALL carry Rust doc comments.

- Modules: `//!` inner doc comment at the top of each file describing purpose and key invariants.
- Structs/enums: `///` doc comment above the item; variants that are non-obvious get their own `///` line.
- Functions: `///` doc comment describing what the function does, its parameters, and what errors it returns. For async functions, note whether they may be long-running.
- The non-blocking invariant (`update()` must return in microseconds) is documented at the `update()` function and the module level of `app.rs`.

## Risks / Trade-offs

- **iced pre-1.0 API instability** → Pin to a specific `iced` version in `Cargo.toml`. Read release notes before any upgrade.
- **`FillPortion` header/row alignment drift** → Use a shared constant or type for column proportions; enforce in a single layout helper function.
- **Session-id rotation mid-poll** → Handled by `SessionRotated` message + immediate retry. Low probability in practice (requires Transmission restart while app is running).
- **`reqwest` + `iced`'s tokio runtime conflict** → Use `iced`'s built-in `tokio` feature flag; do not initialize a separate `tokio::main` runtime.

## Migration Plan

Greenfield project — no migration needed. Steps to bootstrap:

1. `cargo new clutch`
2. Add dependencies to `Cargo.toml`
3. Implement in order: RPC module → Connection screen → Main screen → Subscription

## Open Questions

- Should the connection form persist across app restarts (e.g., via a config file)? Deferred to v0.3.
- Polling interval: 5s for v0.1; revisit for v1.0 (spec says 1-2s).
- Should `wiremock` integration tests run in CI or be gated as `#[ignore]` requiring a flag? Likely always-on since they use a local in-process server (no external dependency).
