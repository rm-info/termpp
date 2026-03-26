# Pane Navigation & Visual Indicator — Design Spec

## Goal

Three focused UX improvements:
1. Replace `Ctrl+Shift+N` pane cycling with `Ctrl+Tab` (forward) / `Ctrl+Shift+Tab` (backward)
2. Assign `Ctrl+Shift+N` to open a new pane
3. Add a visible left accent bar in the sidebar to mark the active pane

## Architecture

All changes are confined to four files: `src/config.rs`, `src/app.rs`, `src/ui/sidebar.rs`, and a one-liner in `src/ui/theme.rs`. No new files. No new dependencies. The three changes are independent and can be implemented in any order.

## Tech Stack

Rust, iced 0.14, serde/TOML config.

---

## 1. Keybinding Config (`src/config.rs`)

Add two new fields to `Keybindings`:

```rust
pub pane_prev: String,  // default: "ctrl+shift+tab"
pub new_pane: String,   // default: "ctrl+shift+n"
```

Change existing default:

```rust
pub pane_next: String,  // default changes from "ctrl+shift+n" to "ctrl+tab"
```

Update `Default for Keybindings` and the serde default functions accordingly. All three are user-overridable via `config.toml`.

## 2. Named Key Support + Dispatch (`src/app.rs`)

### `matches_binding()` extension

The existing `Key::Named(n)` branch in `matches_binding` uses `format!("{n:?}").to_ascii_lowercase() == key_str`. For `Named::Tab`, the Debug repr is `"Tab"`, which lowercased gives `"tab"`. Therefore the existing named-key branch already handles Tab **if the binding string ends in `"tab"`** — no new code branch is strictly required, but a unit test must pin this assumption (see Testing section).

### Message enum

Add `FocusPrev` to the `Message` enum (parallel to `FocusNext`). `NewPane` already exists and is handled in `update()`. What is missing is:
- `FocusPrev` variant in `Message`
- `FocusPrev` handler in `update()` (wrapping formula in section below)
- Subscription dispatch for `pane_prev` and `new_pane`

### Subscription dispatch

In the key-event subscription closure, add:
- `bindings.pane_next` match → `Message::FocusNext`
- `bindings.pane_prev` match → `Message::FocusPrev`  *(new)*
- `bindings.new_pane` match → `Message::NewPane`

Remove the old `pane_next → FocusNext` mapping (previously hard-wired to `"ctrl+shift+n"`).

**Known trade-off:** `Ctrl+Shift+Tab` normally sends `\x1b[Z` (backtab) to the PTY (reverse-tab in readline/vim). Binding it to `FocusPrev` means the PTY will never receive that sequence. This is an acceptable trade-off for a terminal multiplexer — pane navigation takes priority. Users who need backtab in a pane can remap `pane_prev` in their config.

### `FocusPrev` handler

Use the same pattern as `FocusNext`:

```rust
let ids = state.layout.pane_ids();
if let Some(pos) = ids.iter().position(|&id| id == state.active) {
    state.active = ids[(pos + ids.len() - 1) % ids.len()];
}
```

Using `(pos + ids.len() - 1) % ids.len()` avoids `usize` underflow on position 0 (wraps to last pane). The `if let Some(pos)` guard handles the empty-pane case.

## 3. Accent Bar (`src/ui/sidebar.rs` + `src/ui/theme.rs`)

### New color (`src/ui/theme.rs`)

```rust
pub const ACCENT: Color = Color { r: 0.33, g: 0.73, b: 1.0, a: 1.0 }; // bright blue
```

### Active entry layout (`src/ui/sidebar.rs`)

In `render_entry()`, the normal (non-rename) path: build the name/branch column as before, then wrap in:

```
mouse_area(
    row![
        container(Space::new()).width(3).height(Length::Fill)
            .style(|_| Style { background: Some(ACCENT bg), .. }),
        container(name/branch column).width(Length::Fill).padding([6, 10])
    ]
    .width(Length::Fill)
)
.on_press(select_msg)
```

`mouse_area` must wrap the entire `row![]` so the accent strip is also clickable.

**Layout shift note:** The accent strip (3px) is only shown on the active entry; inactive entries have no strip, so the inner text container is 3px wider on inactive entries. Since the sidebar has fixed `.width(200)`, this is a minor and acceptable visual trade-off — consistent with how the rename path already shifts the layout by 3px. Alternatively, always reserve 3px with a transparent strip on inactive entries to avoid any shift; implementer should choose the simpler approach.

The rename path (early return, lines 144–174) does **not** get the accent bar — the rename input replaces the entire row and the 3px shift during rename is acceptable. The `container` wrapping the rename row keeps its existing `width(Length::Fill)` and `padding([6, 10])`.

When inactive: layout unchanged (no accent bar, existing `SIDEBAR_BG` background).

The existing `PANE_BG` background on active entries is kept — the accent bar and background color are complementary.

## Error Handling

No new error surfaces. Config fields fall back to defaults via `#[serde(default)]` if absent from TOML.

## Testing

- **Unit test**: `matches_binding()` returns `true` for `Key::Named(Named::Tab)` with `Modifiers::CTRL` against `"ctrl+tab"` — pins the `format!("{:?}", Named::Tab)` assumption
- **Unit test**: `matches_binding()` returns `true` for `Key::Named(Named::Tab)` with `Modifiers::CTRL | Modifiers::SHIFT` against `"ctrl+shift+tab"`
- **Unit test**: `FocusPrev` wraps from first pane (index 0) to the last — the logically distinct case vs. `FocusNext`
- **Unit test**: `"ctrl+tab"` now matches the `pane_next` binding → positive guard for the new default
- **Unit test**: `"ctrl+shift+n"` no longer matches the `pane_next` binding (regression guard for the reassignment)
- **Smoke test**: `FocusPrev` handler does not panic with 1 pane (wrap to self), 2 panes
- **Smoke test**: accent bar widget construction does not panic; inactive entries render without the 3px strip
