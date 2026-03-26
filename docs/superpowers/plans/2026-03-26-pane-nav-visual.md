# Pane Navigation & Visual Indicator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Ctrl+Shift+N pane cycling with Ctrl+Tab/Ctrl+Shift+Tab, assign Ctrl+Shift+N to new pane, and add a bright left accent bar in the sidebar for the active pane.

**Architecture:** Three independent changes across four files. Config gets two new keybinding fields; app.rs gets a FocusPrev message and subscription updates; sidebar.rs gets a 3px accent strip rendered via a `row![]` wrapper on active entries; theme.rs gets an ACCENT color constant. The existing `matches_binding` already handles Named keys via Debug repr, so no new branch is needed there — a unit test pins the assumption.

**Tech Stack:** Rust, iced 0.14, serde/TOML config, `termpp` library + binary crates.

---

## File Map

| File | Change |
|---|---|
| `src/config.rs` | Add `pane_prev`, `new_pane` fields; change `pane_next` default |
| `src/app.rs` | Add `FocusPrev` to Message; add FocusPrev handler; update subscription dispatch |
| `src/ui/theme.rs` | Add `ACCENT` color constant |
| `src/ui/sidebar.rs` | Update `render_entry()` to show accent strip on active entry |
| `src/ui/help_overlay.rs` | Add `pane_prev` and `new_pane` rows to shortcut list |
| `tests/keybinding_test.rs` | New: unit tests for default values + matches_binding Tab behavior |
| `tests/help_overlay_test.rs` | Update `Keybindings` struct literal to include new fields |

---

## Task 1: Config — New Keybinding Fields

**Files:**
- Modify: `src/config.rs`
- Modify: `src/ui/help_overlay.rs`
- Modify: `tests/help_overlay_test.rs`
- Create: `tests/keybinding_test.rs`

### Context

`src/config.rs` has a `Keybindings` struct. Currently:
```rust
pub struct Keybindings {
    pub split_horizontal: String,  // default: "ctrl+shift+h"
    pub split_vertical: String,    // default: "ctrl+shift+v"
    pub pane_next: String,         // default: "ctrl+shift+n"  ← CHANGES to "ctrl+tab"
    pub close_pane: String,        // default: "ctrl+shift+w"
}
```

We add two new fields and change the `pane_next` default. The serde default function pattern is already in use — follow it exactly.

`tests/help_overlay_test.rs` line 17 has a struct literal that sets all fields — it will fail to compile once we add new fields (Rust requires all fields in a struct literal). We update it as part of this task.

`src/ui/help_overlay.rs` line 15 shows only `pane_next` for navigation — we add `pane_prev` and `new_pane` rows.

- [ ] **Step 1: Write the failing test**

Create `tests/keybinding_test.rs`:

```rust
use termpp::config::Keybindings;

#[test]
fn default_pane_next_is_ctrl_tab() {
    let kb = Keybindings::default();
    assert_eq!(kb.pane_next, "ctrl+tab");
}

#[test]
fn default_pane_prev_is_ctrl_shift_tab() {
    let kb = Keybindings::default();
    assert_eq!(kb.pane_prev, "ctrl+shift+tab");
}

#[test]
fn default_new_pane_is_ctrl_shift_n() {
    let kb = Keybindings::default();
    assert_eq!(kb.new_pane, "ctrl+shift+n");
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test --test keybinding_test
```

Expected: compile error — `pane_prev` and `new_pane` fields don't exist; `pane_next` default is still `"ctrl+shift+n"`.

- [ ] **Step 3: Add new fields to `src/config.rs`**

Add serde default functions and new fields. The full updated `Keybindings` struct and related code:

```rust
#[derive(Debug, Clone, Hash, Deserialize)]
pub struct Keybindings {
    #[serde(default = "default_split_h")]
    pub split_horizontal: String,
    #[serde(default = "default_split_v")]
    pub split_vertical: String,
    #[serde(default = "default_pane_next")]
    pub pane_next: String,
    #[serde(default = "default_pane_prev")]
    pub pane_prev: String,
    #[serde(default = "default_new_pane")]
    pub new_pane: String,
    #[serde(default = "default_close_pane")]
    pub close_pane: String,
}

fn default_split_h()    -> String { "ctrl+shift+h".to_string() }
fn default_split_v()    -> String { "ctrl+shift+v".to_string() }
fn default_pane_next()  -> String { "ctrl+tab".to_string() }         // changed
fn default_pane_prev()  -> String { "ctrl+shift+tab".to_string() }   // new
fn default_new_pane()   -> String { "ctrl+shift+n".to_string() }     // new
fn default_close_pane() -> String { "ctrl+shift+w".to_string() }
```

Update `impl Default for Keybindings`:

```rust
impl Default for Keybindings {
    fn default() -> Self {
        Self {
            split_horizontal: default_split_h(),
            split_vertical:   default_split_v(),
            pane_next:        default_pane_next(),
            pane_prev:        default_pane_prev(),
            new_pane:         default_new_pane(),
            close_pane:       default_close_pane(),
        }
    }
}
```

- [ ] **Step 4: Update `src/ui/help_overlay.rs`**

In `help_overlay()`, the `shortcuts` vec (line 12) currently has 5 entries. Replace it to add `pane_prev` and `new_pane`:

```rust
let shortcuts: Vec<(&'static str, String)> = vec![
    ("Scinder horizontal",  keybindings.split_horizontal.clone()),
    ("Scinder vertical",    keybindings.split_vertical.clone()),
    ("Pane suivant",        keybindings.pane_next.clone()),
    ("Pane précédent",      keybindings.pane_prev.clone()),
    ("Nouveau pane",        keybindings.new_pane.clone()),
    ("Fermer le pane",      keybindings.close_pane.clone()),
    ("Aide",                "F1".to_string()),
];
```

- [ ] **Step 5: Update `tests/help_overlay_test.rs`**

The struct literal at line 17 must include the new fields. Replace the `Keybindings { ... }` block:

```rust
let kb = Keybindings {
    split_horizontal: "ctrl+shift+test_h".to_string(),
    split_vertical:   "ctrl+shift+test_v".to_string(),
    pane_next:        "ctrl+shift+test_n".to_string(),
    pane_prev:        "ctrl+shift+test_p".to_string(),
    new_pane:         "ctrl+shift+test_np".to_string(),
    close_pane:       "ctrl+shift+test_w".to_string(),
};
```

- [ ] **Step 6: Run tests to verify they pass**

```
cargo test --test keybinding_test --test help_overlay_test
```

Expected: all pass. No compile errors.

- [ ] **Step 7: Commit**

```bash
git add src/config.rs src/ui/help_overlay.rs tests/keybinding_test.rs tests/help_overlay_test.rs
git commit -m "feat: add pane_prev and new_pane keybinding fields; update pane_next default to ctrl+tab"
```

---

## Task 2: FocusPrev Message + Subscription Dispatch

**Files:**
- Modify: `src/app.rs`

### Context

`src/app.rs` is the binary crate (`src/main.rs` does `mod app;`). It contains:

**`Message` enum** (lines 52–75): Has `FocusNext` (line 62) but NO `FocusPrev`. We add it.

**`update()` handler** (lines 276–281): `FocusNext` uses:
```rust
Message::FocusNext => {
    let ids = state.layout.pane_ids();
    if let Some(pos) = ids.iter().position(|&id| id == state.active) {
        state.active = ids[(pos + 1) % ids.len()];
    }
}
```
`FocusPrev` mirrors this with `(pos + ids.len() - 1) % ids.len()`.

**`matches_binding()`** (lines 357–387): The `Key::Named(n)` branch at line 384 already uses `format!("{n:?}").to_ascii_lowercase() == key_str`. For `Named::Tab`, `format!("{:?}", Named::Tab)` = `"Tab"` → lowercased = `"tab"`. So `"ctrl+tab"` and `"ctrl+shift+tab"` work without any code change to `matches_binding`. A unit test must pin this assumption.

**Subscription** (lines 486–498): Currently dispatches:
- `split_horizontal` → `SplitPane(Horizontal)`
- `split_vertical` → `SplitPane(Vertical)`
- `pane_next` → `FocusNext`
- `close_pane` → `ClosePane`

We add `pane_prev` → `FocusPrev` and `new_pane` → `NewPane`.

**`Keybindings` is `Clone + Hash`** (already derived), used in the `.with()` call at line 460. Adding fields requires no change to the subscription wiring.

- [ ] **Step 1: Write failing tests**

Add a `#[cfg(test)]` module at the end of `src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::matches_binding;
    use iced::keyboard::{Key, Modifiers};
    use iced::keyboard::key::Named;

    fn ctrl() -> Modifiers { Modifiers::CTRL }
    fn ctrl_shift() -> Modifiers { Modifiers::CTRL | Modifiers::SHIFT }

    #[test]
    fn matches_ctrl_tab_for_pane_next() {
        assert!(matches_binding(&Key::Named(Named::Tab), ctrl(), "ctrl+tab"));
    }

    #[test]
    fn matches_ctrl_shift_tab_for_pane_prev() {
        assert!(matches_binding(&Key::Named(Named::Tab), ctrl_shift(), "ctrl+shift+tab"));
    }

    #[test]
    fn ctrl_tab_does_not_match_ctrl_shift_n() {
        // Regression: old pane_next default "ctrl+shift+n" must NOT match Ctrl+Tab
        assert!(!matches_binding(&Key::Named(Named::Tab), ctrl(), "ctrl+shift+n"));
    }

    #[test]
    fn focus_prev_wraps_from_first_to_last() {
        // Pure logic test for the wrapping formula — no app state needed
        let len = 3;
        let pos = 0;
        let prev = (pos + len - 1) % len;
        assert_eq!(prev, 2, "wrapping from index 0 in a 3-pane layout should give index 2");
    }

    #[test]
    fn focus_prev_middle_position() {
        let len = 3;
        let pos = 1;
        let prev = (pos + len - 1) % len;
        assert_eq!(prev, 0);
    }

    #[test]
    fn focus_prev_single_pane_wraps_to_self() {
        let len = 1;
        let pos = 0;
        let prev = (pos + len - 1) % len;
        assert_eq!(prev, 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test --bin termpp 2>&1 | grep -E "FAILED|error"
```

Expected: `FocusPrev` does not exist yet — but the wrapping formula tests should PASS already. The `matches_binding` tests should PASS (the function already handles Named keys). Confirm that `focus_prev_wraps_from_first_to_last` and `matches_ctrl_tab_for_pane_next` pass — they test existing behavior. Then proceed.

> **Note:** If `matches_binding` tests FAIL (meaning `format!("{:?}", Named::Tab)` is NOT `"Tab"` in this build of iced 0.14), do NOT proceed. Instead: check the actual Debug repr with a `println!` test and update the string parsing in `matches_binding` to handle the actual repr. Surface this to the coordinator.

- [ ] **Step 3: Add `FocusPrev` to `Message` enum**

In `src/app.rs` at line 62, after `FocusNext`, add:

```rust
FocusPrev,
```

The `Message` enum derives `Debug, Clone` — no other changes needed.

- [ ] **Step 4: Add `FocusPrev` handler in `update()`**

In `src/app.rs`, after the `Message::FocusNext` arm (lines 276–281), add:

```rust
Message::FocusPrev => {
    let ids = state.layout.pane_ids();
    if let Some(pos) = ids.iter().position(|&id| id == state.active) {
        state.active = ids[(pos + ids.len() - 1) % ids.len()];
    }
}
```

- [ ] **Step 5: Update subscription dispatch**

In `src/app.rs`, in the `keyboard` subscription closure (the block starting at line 486 labeled `// 4. Normal dispatch`), add the two new dispatches after the `pane_next` check:

Replace:
```rust
if matches_binding(&key, modifiers, &bindings.pane_next) {
    return Some(Message::FocusNext);
}
```

With:
```rust
if matches_binding(&key, modifiers, &bindings.pane_next) {
    return Some(Message::FocusNext);
}
if matches_binding(&key, modifiers, &bindings.pane_prev) {
    return Some(Message::FocusPrev);
}
if matches_binding(&key, modifiers, &bindings.new_pane) {
    return Some(Message::NewPane);
}
```

- [ ] **Step 6: Run tests to verify everything passes**

```
cargo test --bin termpp 2>&1 | grep -E "test .* ok|FAILED|error"
```

Expected: all 6 new tests pass. No compile errors.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat: add FocusPrev message + Ctrl+Tab/Ctrl+Shift+Tab navigation, Ctrl+Shift+N for new pane"
```

---

## Task 3: Sidebar Accent Bar

**Files:**
- Modify: `src/ui/theme.rs`
- Modify: `src/ui/sidebar.rs`
- Create: `tests/sidebar_test.rs`

### Context

**`src/ui/theme.rs`**: Simple constants file. We add one constant.

**`Sidebar::new()` signature** (from `src/ui/sidebar.rs` lines 54–87):
```rust
pub fn new(
    workspaces:       &[WorkspaceEntry],  // 1
    active_id:        usize,              // 2
    renaming:         Option<(usize, String)>, // 3
    on_select:        fn(usize) -> Message,    // 4
    on_close:         fn(usize) -> Message,    // 5
    on_new:           Message,                 // 6
    on_rename_start:  fn(usize) -> Message,    // 7
    on_rename_change: fn(String) -> Message,   // 8
    on_rename_commit: Message,                 // 9
    on_rename_cancel: Message,                 // 10
    on_help:          Message,                 // 11
) -> Self
```
All `fn(usize) -> Message` callbacks are fn pointers (not closures). With `Message = ()`, use `|_| ()` for each — Rust coerces a non-capturing closure to a fn pointer.

**`src/ui/sidebar.rs` `render_entry()` (lines 138–228)**: The normal-mode path (lines 176–228) currently:

```rust
mouse_area(
    container(column![name_row, branch_row].spacing(2))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg_color)),
            ..Default::default()
        })
)
.on_press(select_msg)
.into()
```

For active entries, we replace this with a `row![]` containing a 3px accent strip + the content container. The `mouse_area` wraps the entire row.

The rename path (lines 143–174) is left unchanged — no accent bar during rename (transient mode, 3px shift acceptable).

Imports already in scope in `sidebar.rs`: `row`, `container`, `Space`, `Background`, `Length`, `mouse_area`.

- [ ] **Step 1: Write smoke test**

Create `tests/sidebar_test.rs`:

```rust
use termpp::ui::sidebar::{Sidebar, WorkspaceEntry};

#[test]
fn sidebar_renders_with_active_entry() {
    // Tests that the widget tree builds without panic when there is an active entry.
    // This exercises the accent-bar code path in render_entry().
    let entries = vec![
        WorkspaceEntry { id: 0, name: "main".to_string(), git_branch: Some("main".to_string()), cwd: "/home".to_string(), has_waiting: false },
        WorkspaceEntry { id: 1, name: "dev".to_string(),  git_branch: None, cwd: "/tmp".to_string(), has_waiting: true },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,     // active_id = first entry — exercises the accent-bar code path
        None,  // not renaming
        |_| (), // on_select
        |_| (), // on_close
        (),     // on_new
        |_| (), // on_rename_start
        |_| (), // on_rename_change
        (),     // on_rename_commit
        (),     // on_rename_cancel
        (),     // on_help
    ).view();
}

#[test]
fn sidebar_renders_with_no_active_match() {
    // active_id not in entries — exercises the inactive (no accent bar) path
    let entries = vec![
        WorkspaceEntry { id: 0, name: "main".to_string(), git_branch: None, cwd: "/home".to_string(), has_waiting: false },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        99,     // active_id = no match
        None,
        |_| (), |_| (), (), |_| (), |_| (), (), (), (),
    ).view();
}
```

- [ ] **Step 2: Run to verify it compiles (it should)**

```
cargo test --test sidebar_test
```

Expected: tests PASS (or compile) — this is a baseline before the accent-bar change. If they fail for unrelated reasons, investigate.

- [ ] **Step 3: Add ACCENT to `src/ui/theme.rs`**

After `BADGE_ACTIVE`, add:

```rust
pub const ACCENT: Color = Color { r: 0.33, g: 0.73, b: 1.0, a: 1.0 };
```

- [ ] **Step 4: Update `render_entry()` in `src/ui/sidebar.rs`**

At the end of `render_entry()` (the normal-mode path, lines 217–228), replace the final `mouse_area(...)` block:

**Before** (current code, lines 217–228):
```rust
mouse_area(
    container(column![name_row, branch_row].spacing(2))
        .width(Length::Fill)
        .padding([6, 10])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg_color)),
            ..Default::default()
        })
)
.on_press(select_msg)
.into()
```

**After**:
```rust
let content = container(column![name_row, branch_row].spacing(2))
    .width(Length::Fill)
    .padding([6, 10])
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    });

if is_active {
    let accent = container(Space::new())
        .width(3)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::ACCENT)),
            ..Default::default()
        });
    mouse_area(
        row![accent, content]
            .width(Length::Fill)
            .height(Length::Shrink)
    )
    .on_press(select_msg)
    .into()
} else {
    mouse_area(content)
        .on_press(select_msg)
        .into()
}
```

Make sure `AppTheme` is in scope — it is: `use crate::ui::theme::Theme as AppTheme;` is at line 5.

- [ ] **Step 5: Run tests to verify they pass**

```
cargo test --test sidebar_test
```

Expected: both tests pass. No compile errors.

- [ ] **Step 6: Run the full test suite**

```
cargo test 2>&1 | tail -20
```

Expected: `keybinding_test`, `help_overlay_test`, `sidebar_test`, and inline `app.rs` tests all pass. Note: `config_test` and `pty_integration_test` have pre-existing failures unrelated to this feature — they are acceptable to see fail.

- [ ] **Step 7: Commit**

```bash
git add src/ui/theme.rs src/ui/sidebar.rs tests/sidebar_test.rs
git commit -m "feat: add sidebar accent bar for active pane (bright blue left border)"
```
