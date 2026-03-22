## Why

Users currently have no way to add new torrents through the GUI — they must manage their daemon
separately. Adding magnet link and `.torrent` file support closes the core usability gap and makes
Clutch a standalone management interface.

## What Changes

- Add a magnet-link text input to the main-screen toolbar that fires `torrent-add` via RPC.
- Add a file-picker button that opens a native dialog, reads a `.torrent` file, Base64-encodes it,
  and submits it via `torrent-add`.
- Introduce two new crate dependencies: `rfd` (native file dialog) and `base64` (encoding).
- Extend the RPC client with a `torrent_add` function.
- After a successful add, immediately refresh the torrent list.

## Capabilities

### New Capabilities

- `add-torrent`: UI and RPC flow for adding torrents via magnet link or `.torrent` file.

### Modified Capabilities

- `rpc-client`: New `torrent_add` function added to the existing client.

## Impact

- **src/rpc.rs** — new `torrent_add(url, creds, session_id, payload)` function.
- **src/screens/main_screen.rs** — toolbar extended with magnet input and file-picker button;
  new message variants handled in `update()`.
- **src/app.rs** — new `Message` variants forwarded to `MainScreen`.
- **Cargo.toml** — adds `rfd` and `base64` crate dependencies.
