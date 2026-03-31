# Mouse Support: Scrollback & Selection

**Date:** 2026-03-31
**Status:** Approved

## Overview

Add mouse wheel scrollback through terminal history and click-drag text selection with clipboard integration.

## Section 1 — Scrollback Buffer

### Data

Two new fields in `GridPerformer` (`src/terminal/grid.rs`):

```rust
scrollback: VecDeque<Vec<Cell>>,  // max 10 000 lines, oldest popped when full
scroll_offset: usize,             // 0 = at bottom; N = N lines scrolled back
```

### Behaviour

- `scroll_up()` pushes the displaced row into `scrollback` before dropping it from the visible grid.
- New method `visible_row(visual_row: usize, offset: usize) -> &[Cell]` returns the correct row for rendering: when `offset > 0`, rows are read from the combined `scrollback + cells` view.
- When new PTY output arrives and `scroll_offset > 0`, the offset is **not reset** — the user stays at their reading position.
- When the user scrolls past the bottom (offset would go below 0), offset is clamped to 0 (returns to live view).
- `scroll_offset` is reset to 0 when the alternate screen is entered/exited (vim, less, etc.).

### Input

- Each pane's `mouse_area` gets `.on_scroll(|delta| Message::PaneScrolled(pane_id, delta))`.
- `Message::PaneScrolled(PaneId, delta: f32)` handler: `scroll_offset += delta * 3` lines, clamped to `[0, scrollback.len()]`.
- Scroll speed: 3 lines per scroll unit (not configurable in V1).

### Visual indicator

When `scroll_offset > 0`, a small dimmed label is rendered at the top-right corner of the canvas (see Section 3). No other visual change to the pane border.

---

## Section 2 — Text Selection & Clipboard

### State

In `PaneState` (`src/multiplexer/pane.rs`):

```rust
selection: Option<((usize, usize), (usize, usize))>
// ((start_col, start_row), (end_col, end_row)) — visual coordinates
```

Visual coordinates: `(0,0)` = top-left of the currently rendered area (accounting for `scroll_offset`).

### Input flow

| Event | Message | Effect |
|-------|---------|--------|
| Left press on pane | `SelectionStart(PaneId, x, y)` | Convert pixel → cell, store start; activate global drag subscription |
| Mouse move (global, while selecting) | `SelectionDrag(x, y)` | Update end cell in `PaneState::selection` |
| Mouse release (global) | `SelectionEnd` | Finalise selection, extract text, write to clipboard, deactivate subscription |
| Right press on pane | `PasteFromClipboard(PaneId)` | Read clipboard, write to PTY of that pane |

Global drag subscription follows the same pattern as `SplitDividerDragged`: activated in `Termpp::subscription()` when `state.is_selecting == true`. New field added to `Termpp`: `is_selecting: bool`.

### Pixel → cell conversion

```
col = floor((x - ACCENT_BAR_W - TERM_PADDING) / (font_size * CHAR_W_RATIO))
row = floor((y - TERM_PADDING) / (font_size * LINE_H_RATIO))
```

Both clamped to `[0, cols-1]` and `[0, rows-1]`.

### Text extraction

For each row in `[start_row..=end_row]`:
- Full row if between start and end.
- Partial first/last row respecting `start_col`/`end_col`.
- Cells with `ch == '\0'` (wide char continuation) are skipped.
- Rows joined with `\n`.

### Clipboard

Crate: `arboard` (cross-platform, already works on Windows).

- **Copy**: `Clipboard::new()?.set_text(extracted_text)` on `SelectionEnd`.
- **Paste**: `Clipboard::new()?.get_text()` on `PasteFromClipboard`, written to PTY via `emu.write_input(text.as_bytes())`.

### Constraints

- Selection is in **visual coordinates only** — no cross-scrollback selection in V1.
- If `scroll_offset` changes while a selection is active, the selection is cleared.
- Single-click (no drag) selects nothing (just focuses the pane as before).

---

## Section 3 — Canvas Rendering

### TerminalPane signature change

```rust
pub fn new(
    grid: Arc<Mutex<GridPerformer>>,
    scroll_offset: usize,
    selection: Option<((usize, usize), (usize, usize))>,
    font_size: f32,
    font_name: &'static str,
    cursor_on: bool,
) -> Self
```

### Rendering with scroll offset

In `draw()`, replace `grid.cell(col, row)` with `grid.visible_row(row, scroll_offset)[col]` to display the correct lines when scrolled.

### Selection highlight

For each cell `(col, row)` in the selection range, swap fg/bg to invert colors instead of drawing the normal cell colors. Selection range check:

```
is_selected = (row, col) is between (start_row, start_col) and (end_row, end_col)
             in reading order (row-major)
```

### Scrollback indicator

When `scroll_offset > 0`, draw a small dimmed label at the top-right corner of the canvas:
```
"↑ N lignes"
```
using `frame.fill_text(...)` with `AppTheme::TEXT_DIM` color.

---

## Dependencies

- Add `arboard` to `Cargo.toml`.

## Out of scope (V2+)

- Selection spanning scrollback history
- Double-click word selection / triple-click line selection
- xterm mouse reporting (forwarding events to terminal apps like vim)
- Configurable scroll speed
- Keyboard shortcuts for copy/paste (Ctrl+Shift+C/V)
