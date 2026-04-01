## Context

Clutch runs on iced 0.14 with the Elm architecture. All UI state is immutable between
frames; side-effects are expressed as `Task` values returned from `update()`. The app
currently has no keyboard event handling.

Key iced 0.14 APIs relevant here:

- `text_input::Id` — stable identifier for a `text_input` widget.
- `.id(id)` on `text_input` — binds the id.
- `text_input::focus(id): Task<_>` — returns a `Task` that moves focus to the widget.
- `iced::keyboard::on_key_press(f): Subscription<M>` — fires `f` on every key press,
  returning `Some(msg)` to emit a message or `None` to ignore.

## Goals / Non-Goals

**Goals:**

- Tab / Shift-Tab cycles focus through all `text_input` widgets on the active screen
  or dialog, wrapping around at the ends.
- Pressing Enter in any dialog or quick-connect form triggers the primary CTA without
  a mouse click.
- When a modal dialog (add-torrent, passphrase unlock) opens, the first empty text
  input is focused automatically; if all fields are pre-filled the primary CTA button
  receives focus.

**Non-Goals:**

- Full WCAG 2.1 compliance or ARIA roles (desktop app).
- Arrow-key navigation in lists or tables.
- Keyboard shortcuts / accelerators (Cmd+N, Cmd+, etc.) — separate change.
- Settings screen Enter-to-save (no modal confirmation; save is explicit Save button).

## Decisions

### 1 — Subscription-based keyboard event routing

**Decision:** Use `iced::keyboard::on_key_press` subscriptions to intercept Tab and
Enter keys, emitting screen-specific messages when those keys are pressed.

**Rationale:** Centralising key handling in the subscription is idiomatic iced: it keeps
`view()` free of imperative callbacks and means the logic lives next to the `update()`
that acts on it. The per-widget `on_key_press` alternative would scatter focus logic
across every `text_input` call site, making it fragile when fields are added or removed.

**Alternatives considered:**

- _Per-widget `on_key_press`_: every `text_input` returns a focus-next message. Rejected
  — duplicates focus-order knowledge in view code.
- _HTML-style `tabindex`_: not available in iced.

### 2 — Static focus order derived from field declaration order

**Decision:** The focus order (Tab ring) for each screen/dialog is a fixed `&[text_input::Id]`
slice, declared alongside the view of that screen. Tab advances the index mod N;
Shift-Tab decrements it.

**Rationale:** The number of inputs per screen is small (≤ 5) and rarely changes.
A dynamic discovery approach adds complexity without real benefit.

### 3 — Auto-focus via Task returned from `update()`

**Decision:** Whenever `update()` creates or opens a dialog, it returns
`text_input::focus(first_empty_id)` as part of its `Task`. If all fields are
pre-populated, focus falls back to the first text input in the ring.

**Rationale:** Expressing focus side-effects as `Task` is the correct iced pattern.
It integrates seamlessly with existing `Task::batch(...)` returns. `button::focus`
is not available in iced 0.14 — the `button` widget has no `.id()` method and is
not wired into the engine's `focusable` operation system. The fallback (first input
in the ring) is acceptable: Decision 4 ensures the user can still press Enter to
confirm the primary action regardless of which widget holds focus.

### 4 — Enter key mapped to the logical primary action message

**Decision:** In the subscription handler, `Key::Named(Named::Enter)` maps to the
same message that the primary CTA button's `.on_press(...)` emits (e.g.
`Message::ConnectClicked`, `Message::AddConfirmed`). The `update()` handler for
those messages already performs the correct action; no duplication needed.

**Rationale:** Re-using the existing message keeps the state machine consistent. The
modifier check (`!modifiers.control() && !modifiers.alt()`) prevents conflicts with
other shortcuts.

## Risks / Trade-offs

- **[Risk] Subscription active when dialog is not open** → Each subscription handler
  checks the relevant state flag (e.g., `is_dialog_open`) and returns `None` when
  inactive. This eliminates spurious message emission.
- **[Risk] Focus task races with widget construction** → iced processes `Task`s after
  the next frame render, so the widget will exist by the time focus is requested. No
  known issue.
- **[Risk] iced 0.14 lacks `button::focus`** → Confirmed: the `button` widget has no
  `.id()` method and cannot receive programmatic focus. Fall back to the first text
  input in the ring when all fields are pre-filled. UX impact is minimal: Enter still
  submits the form via the subscription handler (Decision 4).
