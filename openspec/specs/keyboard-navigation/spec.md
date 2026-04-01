# keyboard-navigation Specification

## Purpose

Defines keyboard accessibility requirements for all text-input forms and modal dialogs in
Clutch: Tab/Shift-Tab focus cycling, Enter-to-confirm for primary CTAs, and automatic
focus of the first empty text input when a dialog opens.

## Requirements

### Requirement: Tab focus cycling in text-input screens

All `text_input` widgets on the active screen or dialog SHALL participate in a keyboard
Tab ring. Pressing Tab moves focus to the next input in declaration order, cycling back
to the first after the last. Pressing Shift-Tab moves focus to the previous input,
cycling to the last after the first. Focus cycling is driven by an
`iced::keyboard::listen` subscription that is active only while the relevant screen or
dialog is displayed.

#### Scenario: Tab advances focus from first to second input

- **WHEN** the first text input has focus
- **AND** the user presses Tab
- **THEN** focus moves to the second text input in the declared order

#### Scenario: Tab wraps from last to first input

- **WHEN** the last text input in the ring has focus
- **AND** the user presses Tab
- **THEN** focus moves to the first text input in the ring

#### Scenario: Shift-Tab moves focus backwards

- **WHEN** the second text input has focus
- **AND** the user presses Shift-Tab
- **THEN** focus moves to the first text input in the ring

#### Scenario: Shift-Tab wraps from first to last input

- **WHEN** the first text input in the ring has focus
- **AND** the user presses Shift-Tab
- **THEN** focus moves to the last text input in the ring

### Requirement: Enter key triggers primary CTA in dialogs and forms

In any modal dialog or the Quick Connect form, pressing Enter (without Ctrl or Alt
modifiers) SHALL emit the same message as clicking the primary CTA button. If the
primary action is currently disabled (e.g. a connection probe is in-flight), Enter
SHALL be ignored.

#### Scenario: Enter confirms the primary action when available

- **WHEN** a dialog or Quick Connect form is active
- **AND** the primary CTA button is enabled
- **AND** the user presses Enter (no Ctrl or Alt modifier)
- **THEN** the primary CTA action is triggered as if the button was clicked

#### Scenario: Enter is ignored during in-flight operations

- **WHEN** a connection probe or add-torrent RPC call is in-flight
- **AND** the primary CTA is disabled
- **AND** the user presses Enter
- **THEN** no duplicate action is triggered

#### Scenario: Enter with Ctrl or Alt modifier is ignored

- **WHEN** a dialog or form is active
- **AND** the user presses Ctrl+Enter or Alt+Enter
- **THEN** no primary CTA action is triggered

### Requirement: Auto-focus first empty input on dialog open

When a modal dialog opens, the system SHALL automatically focus the first text input
that is empty. If all text inputs have pre-filled values, focus SHALL move to the first
text input in the ring. This focus SHALL be applied via a `focus(id)` Task returned
from the `update()` call that opens the dialog.

#### Scenario: First empty input is focused when dialog opens

- **WHEN** a modal dialog opens with at least one empty text input
- **THEN** focus is placed on the first empty text input without any mouse interaction

#### Scenario: First input is focused when all fields are pre-filled

- **WHEN** a modal dialog opens and all text inputs contain pre-filled values
- **THEN** focus is placed on the first text input in the ring
