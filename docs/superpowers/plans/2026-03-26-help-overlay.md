# Help Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `?` sidebar button and `F1` shortcut that open a modal popup listing all keyboard shortcuts, dismissible via `×` or `Escape`.

**Architecture:** A new `help_overlay` free function produces a full-screen `stack![]` overlay (backdrop + centered card) built entirely from owned strings (satisfying iced's `Element<'static, M>`). A single `show_help: bool` field on `Termpp` controls visibility, toggled by a new `Message::ToggleHelp`.

**Tech Stack:** Rust, iced 0.14 (`iced::widget::stack![]`, `mouse_area`, `container`, `column`, `row`, `text`, `Space`), no new dependencies.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ui/mod.rs` | Modify | Register `help_overlay` module |
| `src/ui/help_overlay.rs` | Create | `help_overlay()` widget function |
| `src/app.rs` | Modify | State field, message, update, subscription, view |
| `src/ui/sidebar.rs` | Modify | `on_help` field, `?` button, column restructure |
| `tests/help_overlay_test.rs` | Create | Smoke test for widget construction |

---

### Task 1: Register the module and create a compilable stub

**Files:**
- Modify: `src/ui/mod.rs`
- Create: `src/ui/help_overlay.rs`

- [ ] **Step 1: Add `pub mod help_overlay;` to `src/ui/mod.rs`**

The file currently contains:
```rust
pub mod pane_grid;
pub mod sidebar;
pub mod theme;
```
Add one line at the end:
```rust
pub mod pane_grid;
pub mod sidebar;
pub mod theme;
pub mod help_overlay;
```

- [ ] **Step 2: Create `src/ui/help_overlay.rs` with a minimal stub**

```rust
use iced::Element;
use crate::config::Keybindings;

pub fn help_overlay<Message: Clone + 'static>(
    _keybindings: &Keybindings,
    _on_close: Message,
) -> Element<'static, Message> {
    iced::widget::text("TODO").into()
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/ui/help_overlay.rs
git commit -m "feat: stub help_overlay module"
```

---

### Task 2: Write the smoke test (TDD — write test first)

**Files:**
- Create: `tests/help_overlay_test.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/help_overlay_test.rs`:
```rust
use termpp::config::Keybindings;
use termpp::ui::help_overlay::help_overlay;

#[test]
fn help_overlay_builds_without_panic() {
    let kb = Keybindings::default();
    // Construct the widget tree — no runtime needed for construction.
    // If this panics, the widget has a logic error.
    let _el: iced::Element<'static, ()> = help_overlay(&kb, ());
}

#[test]
fn help_overlay_uses_keybinding_strings() {
    let mut kb = Keybindings::default();
    kb.split_horizontal = "ctrl+shift+test".to_string();
    // Must not borrow from kb — widget must own all strings
    let _el: iced::Element<'static, ()> = help_overlay(&kb, ());
    drop(kb); // kb is dropped before _el — would fail if _el borrowed from kb
    drop(_el);
}
```

- [ ] **Step 2: Run tests to verify they fail (stub returns `text("TODO")` which compiles but the second test with drop ordering still passes — this is fine, the real test is the lifetime constraint)**

```bash
cargo test help_overlay
```
Expected: tests pass (stub compiles and constructs). The real failure will appear in Task 3 if lifetime constraints are violated.

- [ ] **Step 3: Commit**

```bash
git add tests/help_overlay_test.rs
git commit -m "test: add help_overlay smoke tests"
```

---

### Task 3: Implement the full help_overlay widget

**Files:**
- Modify: `src/ui/help_overlay.rs`

- [ ] **Step 1: Replace the stub with the full implementation**

```rust
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Background, Color, Element, Length};

use crate::config::Keybindings;
use crate::ui::theme::Theme as AppTheme;

pub fn help_overlay<Message: Clone + 'static>(
    keybindings: &Keybindings,
    on_close: Message,
) -> Element<'static, Message> {
    // Clone all strings immediately — no borrows from keybindings may escape
    let shortcuts: Vec<(&'static str, String)> = vec![
        ("Scinder horizontal", keybindings.split_horizontal.clone()),
        ("Scinder vertical",   keybindings.split_vertical.clone()),
        ("Pane suivant",       keybindings.pane_next.clone()),
        ("Fermer le pane",     keybindings.close_pane.clone()),
        ("Aide",               "F1".to_string()),
    ];

    let close_msg = on_close.clone();
    let close_btn: Element<'static, Message> = mouse_area(
        text("×").color(AppTheme::TEXT_DIM).size(14)
    )
    .on_press(close_msg)
    .into();

    let header: Element<'static, Message> = row![
        text("Raccourcis")
            .color(AppTheme::TEXT_PRIMARY)
            .size(15)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT }),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(iced::Alignment::Center)
    .into();

    let separator: Element<'static, Message> = container(Space::new().height(1))
        .width(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::PANE_BORDER)),
            ..Default::default()
        })
        .into();

    let rows: Vec<Element<'static, Message>> = shortcuts
        .into_iter()
        .map(|(label, key)| {
            let badge: Element<'static, Message> = container(
                text(key).color(AppTheme::TEXT_PRIMARY).size(12)
            )
            .padding([2, 6])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(
                    Color { r: 0.10, g: 0.10, b: 0.15, a: 1.0 }
                )),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into();

            row![
                text(label).color(AppTheme::TEXT_DIM).size(12),
                Space::new().width(Length::Fill),
                badge,
            ]
            .align_y(iced::Alignment::Center)
            .into()
        })
        .collect();

    let card_content = column([
        header,
        separator,
    ])
    .extend(rows)
    .spacing(8);

    let card: Element<'static, Message> = container(card_content)
        .width(320)
        .padding(20)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::PANE_BG)),
            border: iced::Border {
                color: AppTheme::PANE_BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into();

    // Backdrop: full-screen semi-transparent overlay, card centered
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(
                Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }
            )),
            ..Default::default()
        })
        .into()
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test help_overlay
```
Expected: both tests pass, no panics.

- [ ] **Step 3: Cargo check (full codebase)**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/help_overlay.rs
git commit -m "feat: implement help_overlay widget"
```

---

### Task 4: Add ToggleHelp to app state, messages, and update handler

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add `show_help: bool` to `Termpp` struct**

In `src/app.rs`, the `Termpp` struct (lines 29–48) gains one field. Add after `blink_tick`:
```rust
    blink_tick:         u8,
    show_help:          bool,
```

- [ ] **Step 2: Initialize `show_help` in `boot()`**

In the `Termpp { ... }` constructor inside `boot()` (around line 91), add:
```rust
        show_help:          false,
```

- [ ] **Step 3: Add `ToggleHelp` to the `Message` enum**

Add after `CancelRename` (line 71):
```rust
    ToggleHelp,
```

- [ ] **Step 4: Add the update handler**

In `update()`, add a new arm in the `match message` block after the `CancelRename` arm:
```rust
        Message::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                // Dismiss any active rename when opening the overlay
                state.renaming_pane = None;
            }
        }
```

- [ ] **Step 5: Cargo check**

```bash
cargo check
```
Expected: warning about `ToggleHelp` unmatched in subscription but no errors (or just unused-variable warnings).

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: add ToggleHelp message and show_help state"
```

---

### Task 5: Update subscription — F1, Escape, show_help suppression

**Files:**
- Modify: `src/app.rs` (subscription function, lines 346–408)

- [ ] **Step 1: Capture `show_help` in the subscription**

Find the lines (around 353–354):
```rust
    let bindings   = state.config.keybindings.clone();
    let is_renaming = state.renaming_pane.is_some();
```
Add a third line:
```rust
    let bindings    = state.config.keybindings.clone();
    let is_renaming = state.renaming_pane.is_some();
    let show_help   = state.show_help;
```

- [ ] **Step 2: Expand the `.with(...)` tuple and update closure signature**

Find (around line 356):
```rust
    let keyboard = iced::event::listen()
        .with((bindings, is_renaming))
        .filter_map(|((bindings, is_renaming), event): ((termpp::config::Keybindings, bool), iced::Event)| -> Option<Message> {
```
Replace with:
```rust
    let keyboard = iced::event::listen()
        .with((bindings, is_renaming, show_help))
        .filter_map(|((bindings, is_renaming, show_help), event): ((termpp::config::Keybindings, bool, bool), iced::Event)| -> Option<Message> {
```

- [ ] **Step 3: Update key dispatch order inside the closure**

The existing closure body starts with a `if let iced::Event::Keyboard(...)` block. Inside that block, find the current guard:
```rust
                if is_renaming {
                    use iced::keyboard::key::Named;
                    if matches!(key, iced::keyboard::Key::Named(Named::Escape)) {
                        return Some(Message::CancelRename);
                    }
                    return None;
                }
```

Replace the entire block (from `if is_renaming {` to the closing `}` of that guard, then through the existing binding checks) with:
```rust
                use iced::keyboard::key::Named;

                // 1. F1 always opens/closes help — checked before any other guard
                if matches!(key, iced::keyboard::Key::Named(Named::F1)) {
                    return Some(Message::ToggleHelp);
                }

                // 2. During rename: only Escape passes through
                if is_renaming {
                    if matches!(key, iced::keyboard::Key::Named(Named::Escape)) {
                        return Some(Message::CancelRename);
                    }
                    return None;
                }

                // 3. Help overlay open: only Escape passes through
                if show_help {
                    if matches!(key, iced::keyboard::Key::Named(Named::Escape)) {
                        return Some(Message::ToggleHelp);
                    }
                    return None;
                }

                // 4. Normal dispatch
                if matches_binding(&key, modifiers, &bindings.split_horizontal) {
                    return Some(Message::SplitPane(SplitDirection::Horizontal));
                }
                if matches_binding(&key, modifiers, &bindings.split_vertical) {
                    return Some(Message::SplitPane(SplitDirection::Vertical));
                }
                if matches_binding(&key, modifiers, &bindings.pane_next) {
                    return Some(Message::FocusNext);
                }
                if matches_binding(&key, modifiers, &bindings.close_pane) {
                    return Some(Message::ClosePane);
                }
                let bytes = key_to_bytes(&key, modifiers, text.as_deref());
                if bytes.is_empty() { None } else { Some(Message::KeyInput(bytes)) }
```

Note: the `use iced::keyboard::key::Named;` that was previously inside the `is_renaming` block is now hoisted up (remove the one inside `is_renaming` if it still exists).

- [ ] **Step 4: Cargo check**

```bash
cargo check
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: wire F1 and Escape for help overlay in subscription"
```

---

### Task 6: Update Sidebar and wire app.rs view (one atomic change)

These two changes are tightly coupled — the Sidebar `on_help` parameter addition breaks the `Sidebar::new()` call in `app.rs`, so both must be fixed before any cargo check or commit.

**Files:**
- Modify: `src/ui/sidebar.rs`
- Modify: `src/app.rs` (view function, lines 501–610)

- [ ] **Step 1: Add `on_help: Message` field to the `Sidebar` struct**

In the `Sidebar<Message>` struct definition (lines 39–50), add after `on_rename_cancel`:
```rust
    on_rename_cancel: Message,
    on_help:          Message,
```

- [ ] **Step 2: Add `on_help` parameter to `Sidebar::new()`**

In the `new()` function signature, add as the last parameter:
```rust
        on_rename_cancel: Message,
        on_help:          Message,
    ) -> Self {
```

In the `Self { ... }` constructor body, add:
```rust
            on_rename_cancel,
            on_help,
```

- [ ] **Step 3: Build the `?` button and restructure `view()`**

In `view()`, find the current column construction (around line 106):
```rust
        container(column(entries).spacing(1).push(new_btn))
```

Replace with:
```rust
        let help_msg = self.on_help.clone();
        let help_btn: Element<'static, Message> = mouse_area(
            container(text("?").color(AppTheme::TEXT_DIM).size(16))
                .width(Length::Fill)
                .padding([6, 10])
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
                    ..Default::default()
                })
        )
        .on_press(help_msg)
        .into();

        container(
            column(entries)
                .spacing(1)
                .push(new_btn)
                .push(Space::new().height(Length::Fill))
                .push(help_btn)
        )
```

- [ ] **Step 4: Add `use` import for `help_overlay` at the top of `app.rs`**

Add to the existing imports block:
```rust
use termpp::ui::help_overlay::help_overlay;
```

- [ ] **Step 5: Pass `Message::ToggleHelp` to `Sidebar::new()` in `app.rs`**

Find the `Sidebar::<Message>::new(` call (around line 511). It currently ends with:
```rust
            Message::CommitRename,
            Message::CancelRename,
        ).view())
```
Add `Message::ToggleHelp` as the final argument:
```rust
            Message::CommitRename,
            Message::CancelRename,
            Message::ToggleHelp,
        ).view())
```

- [ ] **Step 6: Wrap the view with stack when show_help is true**

Find the end of `view()` (around line 606):
```rust
    container(row![sidebar, divider, pane_view])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
```

Replace with:
```rust
    let base: Element<'static, Message> = container(row![sidebar, divider, pane_view])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    if state.show_help {
        iced::widget::stack![
            base,
            help_overlay(&state.config.keybindings, Message::ToggleHelp)
        ]
        .into()
    } else {
        base
    }
```

- [ ] **Step 7: Full build**

```bash
cargo build
```
Expected: clean build, no errors.

- [ ] **Step 8: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/ui/sidebar.rs src/app.rs
git commit -m "feat: wire help overlay — Sidebar ? button, app.rs view stack"
```

---

### Task 8: Manual smoke test

- [ ] **Step 1: Run the application**

```bash
cargo run
```

- [ ] **Step 2: Verify ? button**
  - The `?` button appears at the bottom of the sidebar
  - Click it: the help overlay appears with all 5 shortcut rows
  - Click `×`: overlay closes

- [ ] **Step 3: Verify F1**
  - Press `F1`: overlay opens
  - Press `Escape`: overlay closes
  - Press `F1` again: overlay opens
  - Press `F1` again: overlay closes (toggle)

- [ ] **Step 4: Verify suppression**
  - Open the overlay (`F1`)
  - Type any characters: nothing sent to the terminal (PTY not receiving input)
  - Press `Escape`: overlay closes, terminal active again

- [ ] **Step 5: Verify rename interaction**
  - Click `✎` on a workspace to start a rename
  - Press `F1`: rename is dismissed, overlay opens
  - Press `Escape`: overlay closes

- [ ] **Step 6: Final commit (if any polish needed)**

```bash
git add -p
git commit -m "fix: polish help overlay after manual testing"
```
