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
| `F1` key | always (even during rename) | `ToggleHelp` |
| `Escape` key | `show_help == true` | `ToggleHelp` |
| `Escape` key | `is_renaming == true`, `show_help == false` | `CancelRename` |
| `?` sidebar button | click | `ToggleHelp` |
| `×` in popup | click | `ToggleHelp` |

### Subscription capture tuple

The `.with(...)` tuple in `subscription()` must be expanded from `(bindings, is_renaming)` to `(bindings, is_renaming, show_help)`. The closure's explicit type annotation (currently `((termpp::config::Keybindings, bool), iced::Event)`) must also be updated to `((termpp::config::Keybindings, bool, bool), iced::Event)`, and the destructuring pattern from `((bindings, is_renaming), event)` to `((bindings, is_renaming, show_help), event)`.

### Key dispatch order (inside the filter_map closure)

```rust
// 1. F1 always triggers ToggleHelp — must be BEFORE the is_renaming guard
if key == Named::F1 {
    return Some(Message::ToggleHelp);
}

// 2. During rename: only Escape passes through (cancels rename)
if is_renaming {
    if key == Named::Escape {
        return Some(Message::CancelRename);
    }
    return None;
}

// 3. Help overlay open: only Escape passes through (closes help)
if show_help {
    if key == Named::Escape {
        return Some(Message::ToggleHelp);
    }
    return None; // suppress all other keys — don't reach PTY
}

// 4. Normal key dispatch (existing binding checks + key_to_bytes)
```

### Compound state (show_help + is_renaming simultaneously)

Opening the help overlay while a rename is in progress is prevented: the `ToggleHelp` update handler also clears `renaming_pane`:
```rust
Message::ToggleHelp => {
    state.show_help = !state.show_help;
    if state.show_help {
        state.renaming_pane = None; // dismiss rename when opening help
    }
}
```
This ensures the two states never coexist.

## Sidebar Changes (`src/ui/sidebar.rs`)

Add `on_help: Message` field to `Sidebar<Message>`, matching the existing `on_new: Message` pattern (plain value, not a function pointer).

The `?` button uses identical style to `+`: `TEXT_DIM`, size 16, padding `[6, 10]`, background `SIDEBAR_BG`. It uses the same `mouse_area(container(text("?")))` pattern already established in the file.

### `view()` column restructuring

The current `view()` builds: `column(entries).push(new_btn)` wrapped in a `container`.

It must be restructured to:
```rust
column(entries)
    .push(new_btn)
    .push(Space::new().height(Length::Fill))  // pushes help btn to bottom
    .push(help_btn)
```

The outer `container` is unchanged. The resulting layout:
```
[workspace entry 0]
[workspace entry 1]
...
[+ new pane button]
<flexible space>
[? help button]
```

### Call site

`Sidebar::new()` is called in `src/app.rs::view()` (not `main.rs`). Add `Message::ToggleHelp` as the `on_help` argument. The `Sidebar::new()` constructor gains one additional parameter at the end: `on_help: Message`.

## Help Overlay Widget (`src/ui/help_overlay.rs`)

`iced::widget::stack![]` is available in `iced_widget-0.14.2` (confirmed in `iced_widget/src/helpers.rs`). No additional feature flag needed.



A single free function:
```rust
pub fn help_overlay<Message: Clone + 'static>(
    keybindings: &Keybindings,
    on_close: Message,
) -> Element<'static, Message>
```

**`'static` constraint:** The return type is `Element<'static, Message>`. All string data from `keybindings` must be cloned into owned `String` values (via `.clone()` or `format!()`) before being passed to `text(...)` widgets, so that no lifetime from `&Keybindings` escapes into the returned element. Example:
```rust
text(keybindings.split_horizontal.clone())  // correct
text(keybindings.split_horizontal.as_str()) // WRONG: borrows from argument
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
| `src/app.rs` | Add `show_help` field, `ToggleHelp` message + update handler, subscription tuple expansion + closure type annotation, F1/Escape key dispatch, view stack, pass `on_help` to `Sidebar::new()` |
| `src/ui/sidebar.rs` | Add `on_help` field + `?` button + `Space::fill()` (Space already imported) |
| `src/ui/help_overlay.rs` | New file: `help_overlay()` function |
| `src/ui/mod.rs` | Add `pub mod help_overlay;` |

## Non-Goals

- No mouse interaction documentation in the popup
- No scrolling (5 rows fit comfortably)
- No animations
- No multi-window approach
- No persistent help panel (overlay only)
