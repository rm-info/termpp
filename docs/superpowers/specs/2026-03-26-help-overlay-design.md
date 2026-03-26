# Help Overlay — Design Spec
**Date:** 2026-03-26

## Overview

Add a `?` button at the bottom of the sidebar and an `F1` keyboard shortcut that open a modal popup listing all keyboard shortcuts. The popup is dismissed via a `×` button or `Escape`.

## Scope

- Display keyboard shortcuts only (no mouse interactions documented)
- Shortcuts read dynamically from `state.config.keybindings` (not hardcoded)
- One additional fixed entry: `F1 — Aide`

## State & Messages

### New state field
```rust
// in struct Termpp
show_help: bool,
```
Initialized to `false`.

### New message
```rust
Message::ToggleHelp
```

### Update handler
```rust
Message::ToggleHelp => {
    state.show_help = !state.show_help;
}
```

## Input Handling

| Trigger | Condition | Effect |
|---|---|---|
| `F1` key | always | `ToggleHelp` |
| `Escape` key | `show_help == true` | `ToggleHelp` (checked before `CancelRename`) |
| `?` sidebar button | click | `ToggleHelp` |
| `×` in popup | click | `ToggleHelp` |

The existing `Escape` handler in `subscription()` already handles `CancelRename`. The new check for `show_help` is inserted before it:
```rust
if is_renaming { ... return CancelRename }
// NEW:
if matches F1 key → return ToggleHelp
// existing bindings...
```

When `show_help` is `true`, all key events other than `F1` and `Escape` are suppressed (return `None`) so keystrokes don't reach the PTY.

## Sidebar Changes (`src/ui/sidebar.rs`)

Add `on_help: Message` field to `Sidebar<Message>`, matching the existing `on_new: Message` pattern.

Layout of the sidebar column becomes:
```
[workspace entry 0]
[workspace entry 1]
...
[+ new pane button]
<Space::new().height(Length::Fill)>   ← pushes ? to bottom
[? help button]
```

The `?` button uses identical style to `+`: `TEXT_DIM`, size 16, padding `[6, 10]`, background `SIDEBAR_BG`.

## Help Overlay Widget (`src/ui/help_overlay.rs`)

A single free function:
```rust
pub fn help_overlay<Message: Clone + 'static>(
    keybindings: &Keybindings,
    on_close: Message,
) -> Element<'static, Message>
```

### Visual structure

```
[full-screen backdrop: rgba(0,0,0,0.6)]
  └─ centered popup card
       ├─ header row: "Raccourcis"  [×]
       ├─ separator
       └─ shortcut rows (label left, badge right):
            Scinder horizontal     Ctrl+Shift+H
            Scinder vertical       Ctrl+Shift+V
            Pane suivant           Ctrl+Shift+N
            Fermer le pane         Ctrl+Shift+W
            Aide                   F1
```

### Styling
- **Backdrop**: `container` filling `Length::Fill × Length::Fill`, background `Color { r:0, g:0, b:0, a:0.6 }`
- **Card**: background `AppTheme::PANE_BG`, border `AppTheme::PANE_BORDER` (width 1), border_radius 8, padding 20, min-width ~320px
- **Title**: `TEXT_PRIMARY`, size 15, bold weight
- **`×` button**: `mouse_area` wrapping `text("×")` in `TEXT_DIM`, size 14, emits `on_close`
- **Label**: `TEXT_DIM`, size 12
- **Shortcut badge**: `container` with background slightly lighter than card (`Color { r:0.10, g:0.10, b:0.15, a:1.0 }`), border_radius 4, padding `[2, 6]`, monospace text `TEXT_PRIMARY` size 12

## View Integration (`src/app.rs`)

Use `iced::widget::stack![]` to overlay:

```rust
let base = container(row![sidebar, divider, pane_view])...;

if state.show_help {
    stack![
        base,
        help_overlay(&state.config.keybindings, Message::ToggleHelp)
    ].into()
} else {
    base.into()
}
```

## Files Changed

| File | Change |
|---|---|
| `src/app.rs` | Add `show_help` field, `ToggleHelp` message, update handler, F1 + Escape key handling, view stack |
| `src/ui/sidebar.rs` | Add `on_help` field + `?` button at bottom |
| `src/ui/help_overlay.rs` | New file: `help_overlay()` function |
| `src/main.rs` or call site | Pass `on_help: Message::ToggleHelp` to `Sidebar::new()` |

## Non-Goals

- No mouse interaction documentation in the popup
- No scrolling (5 rows fit comfortably)
- No animations
- No multi-window approach
- No persistent help panel (overlay only)
