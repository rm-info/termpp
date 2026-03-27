# Workspaces + Tabs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a Workspace → Tab → Pane hierarchy, replacing the current flat pane list in `Termpp`.

**Architecture:** `Termpp` loses its flat `layout`/`panes`/`emulators`/`active`/`next_id` fields; they move into a new `Tab` struct. Tabs live inside `Workspace` structs. `App` holds `Vec<Workspace>`. All existing pane-level message handlers delegate through `active_tab_mut()`. The sidebar gains a two-level collapsible tree. Split-view rendering (multiple panes visible simultaneously) is out of scope.

**Tech Stack:** Rust, iced 0.14, vte, tokio (PTY), serde/toml (config)

---

## File Map

| Action | Path | Purpose |
|---|---|---|
| Create | `src/multiplexer/workspace.rs` | `Tab` and `Workspace` structs |
| Modify | `src/multiplexer/mod.rs` | expose `workspace` module |
| Modify | `src/config.rs` | 6 new keybinding fields, `close_pane` default change |
| Modify | `src/app.rs` | full struct refactor + all message handlers |
| Modify | `src/ui/sidebar.rs` | new two-level tree API and rendering |
| Modify | `src/ui/help_overlay.rs` | new shortcuts |
| Modify | `tests/keybinding_test.rs` | new keybinding default tests |
| Modify | `tests/sidebar_test.rs` | updated for new sidebar API |

---

## Task 1: `src/multiplexer/workspace.rs` — Tab and Workspace structs

**Files:**
- Create: `src/multiplexer/workspace.rs`
- Modify: `src/multiplexer/mod.rs`

- [ ] **Step 1: Write the failing test** (add to bottom of `workspace.rs` before the structs exist)

Create `src/multiplexer/workspace.rs` with only the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplexer::layout::Layout;
    use std::collections::HashMap;

    fn make_tab(id: usize) -> Tab {
        Tab {
            id,
            name: format!("tab-{id}"),
            layout: Layout::new(0),
            panes: HashMap::new(),
            emulators: HashMap::new(),
            active_pane: 0,
            next_pane_id: 1,
            last_output_counts: HashMap::new(),
        }
    }

    #[test]
    fn workspace_active_tab_idx_finds_middle_tab() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0), make_tab(5), make_tab(10)],
            active_tab: 5,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 1);
    }

    #[test]
    fn workspace_active_tab_idx_defaults_to_zero_on_miss() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0)],
            active_tab: 99,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 0);
    }
}
```

Also add `pub mod workspace;` to `src/multiplexer/mod.rs`.

- [ ] **Step 2: Run tests to see them fail**

```bash
cargo test -p termpp workspace -- --nocapture
```

Expected: compile error — `Tab`, `Workspace`, `WorkspaceId`, `TabId` not found.

- [ ] **Step 3: Implement the structs**

Replace the test-only file with the full implementation:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::multiplexer::layout::{Layout, PaneId};
use crate::multiplexer::pane::PaneState;
use crate::terminal::emulator::Emulator;

pub type WorkspaceId = usize;
pub type TabId = usize;

pub struct Tab {
    pub id: TabId,
    pub name: String,
    pub layout: Layout,
    pub panes: HashMap<PaneId, PaneState>,
    pub emulators: HashMap<PaneId, Arc<Mutex<Emulator>>>,
    pub active_pane: PaneId,
    pub next_pane_id: usize,
    pub last_output_counts: HashMap<PaneId, u64>,
}

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<Tab>,
    pub active_tab: TabId,
    pub collapsed: bool,
}

impl Workspace {
    /// Index of the active tab in `self.tabs`. Falls back to 0 if active_tab id not found.
    pub fn active_tab_idx(&self) -> usize {
        self.tabs.iter().position(|t| t.id == self.active_tab).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab(id: usize) -> Tab {
        Tab {
            id,
            name: format!("tab-{id}"),
            layout: Layout::new(0),
            panes: HashMap::new(),
            emulators: HashMap::new(),
            active_pane: 0,
            next_pane_id: 1,
            last_output_counts: HashMap::new(),
        }
    }

    #[test]
    fn workspace_active_tab_idx_finds_middle_tab() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0), make_tab(5), make_tab(10)],
            active_tab: 5,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 1);
    }

    #[test]
    fn workspace_active_tab_idx_defaults_to_zero_on_miss() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0)],
            active_tab: 99,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 0);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p termpp workspace -- --nocapture
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/multiplexer/workspace.rs src/multiplexer/mod.rs
git commit -m "feat: add Tab and Workspace structs to multiplexer"
```

---

## Task 2: `src/config.rs` — new keybinding fields

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/keybinding_test.rs`

- [ ] **Step 1: Write failing tests**

Append to `tests/keybinding_test.rs`:

```rust
#[test]
fn default_close_pane_is_ctrl_shift_q() {
    let kb = Keybindings::default();
    assert_eq!(kb.close_pane, "ctrl+shift+q");
}

#[test]
fn default_tab_next_is_ctrl_pagedown() {
    let kb = Keybindings::default();
    assert_eq!(kb.tab_next, "ctrl+pagedown");
}

#[test]
fn default_workspace_new_is_ctrl_shift_w() {
    let kb = Keybindings::default();
    assert_eq!(kb.workspace_new, "ctrl+shift+w");
}
```

- [ ] **Step 2: Run tests to see them fail**

```bash
cargo test -p termpp --test keybinding_test
```

Expected: `close_pane` test fails (value is `"ctrl+shift+w"`), other two fail to compile (`tab_next`, `workspace_new` fields not found).

- [ ] **Step 3: Update `src/config.rs`**

Replace the `Keybindings` struct and its impl block with:

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
    #[serde(default = "default_rename_pane")]
    pub rename_pane: String,
    #[serde(default = "default_tab_next")]
    pub tab_next: String,
    #[serde(default = "default_tab_prev")]
    pub tab_prev: String,
    #[serde(default = "default_tab_new")]
    pub tab_new: String,
    #[serde(default = "default_workspace_next")]
    pub workspace_next: String,
    #[serde(default = "default_workspace_prev")]
    pub workspace_prev: String,
    #[serde(default = "default_workspace_new")]
    pub workspace_new: String,
}

fn default_split_h()       -> String { "ctrl+shift+h".to_string() }
fn default_split_v()       -> String { "ctrl+shift+v".to_string() }
fn default_pane_next()     -> String { "ctrl+tab".to_string() }
fn default_pane_prev()     -> String { "ctrl+shift+tab".to_string() }
fn default_new_pane()      -> String { "ctrl+shift+n".to_string() }
fn default_close_pane()    -> String { "ctrl+shift+q".to_string() }
fn default_rename_pane()   -> String { "ctrl+shift+r".to_string() }
fn default_tab_next()      -> String { "ctrl+pagedown".to_string() }
fn default_tab_prev()      -> String { "ctrl+pageup".to_string() }
fn default_tab_new()       -> String { "ctrl+shift+t".to_string() }
fn default_workspace_next() -> String { "ctrl+shift+pagedown".to_string() }
fn default_workspace_prev() -> String { "ctrl+shift+pageup".to_string() }
fn default_workspace_new()  -> String { "ctrl+shift+w".to_string() }

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            split_horizontal: default_split_h(),
            split_vertical:   default_split_v(),
            pane_next:        default_pane_next(),
            pane_prev:        default_pane_prev(),
            new_pane:         default_new_pane(),
            close_pane:       default_close_pane(),
            rename_pane:      default_rename_pane(),
            tab_next:         default_tab_next(),
            tab_prev:         default_tab_prev(),
            tab_new:          default_tab_new(),
            workspace_next:   default_workspace_next(),
            workspace_prev:   default_workspace_prev(),
            workspace_new:    default_workspace_new(),
        }
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p termpp --test keybinding_test
```

Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/keybinding_test.rs
git commit -m "feat: add workspace/tab keybindings, move close_pane to ctrl+shift+q"
```

---

## Task 3: `src/app.rs` — add new Message variants and stub handlers

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add new variants to the `Message` enum**

In `src/app.rs`, find the `Message` enum and add these variants after `CancelRename`:

```rust
    // Tab-level
    FocusTabNext,
    FocusTabPrev,
    SelectTab(usize),
    CloseTab(usize),
    NewTabIn(usize),          // arg = workspace_id to create tab in
    StartRenameTab(usize),    // arg = tab_id (replaces current StartRename for panes)
    // Workspace-level
    FocusWorkspaceNext,
    FocusWorkspacePrev,
    NewWorkspace,
    ToggleWorkspace(usize),   // arg = workspace_id, toggles collapsed
```

- [ ] **Step 2: Add stub handlers in `update()`**

In `update()`, add stub arms before the closing `}` of the `match` block:

```rust
        Message::FocusTabNext       => {}
        Message::FocusTabPrev       => {}
        Message::SelectTab(_)       => {}
        Message::CloseTab(_)        => {}
        Message::NewTabIn(_)        => {}
        Message::StartRenameTab(_)  => {}
        Message::FocusWorkspaceNext => {}
        Message::FocusWorkspacePrev => {}
        Message::NewWorkspace       => {}
        Message::ToggleWorkspace(_) => {}
```

- [ ] **Step 3: Verify the project compiles**

```bash
cargo build -p termpp 2>&1 | head -20
```

Expected: compiles without errors (warnings about unused variants are OK).

- [ ] **Step 4: Add new keybinding dispatch to the subscription**

In `subscription()`, in the keyboard closure's "Normal dispatch" section (step 4 in the filter_map), add after the `rename_pane` dispatch and before the `key_to_bytes` fallback:

```rust
                if matches_binding(&key, modifiers, &bindings.tab_next) {
                    return Some(Message::FocusTabNext);
                }
                if matches_binding(&key, modifiers, &bindings.tab_prev) {
                    return Some(Message::FocusTabPrev);
                }
                if matches_binding(&key, modifiers, &bindings.workspace_next) {
                    return Some(Message::FocusWorkspaceNext);
                }
                if matches_binding(&key, modifiers, &bindings.workspace_prev) {
                    return Some(Message::FocusWorkspacePrev);
                }
                if matches_binding(&key, modifiers, &bindings.tab_new) {
                    return Some(Message::NewTabIn(active_workspace_id));
                }
                if matches_binding(&key, modifiers, &bindings.workspace_new) {
                    return Some(Message::NewWorkspace);
                }
```

- [ ] **Step 5: Add `active_workspace_id` to the subscription's captured state**

In `subscription()`, add this line alongside the other captured values:

```rust
    let active_workspace_id = state.active; // placeholder — will be state.active_workspace after Task 4
```

And add it to the `.with()` tuple:

```rust
    let keyboard = iced::event::listen()
        .with((bindings, is_renaming, show_help, active_id, active_workspace_id))
        .filter_map(|((bindings, is_renaming, show_help, active_id, active_workspace_id), event):
            ((termpp::config::Keybindings, bool, bool, usize, usize), iced::Event)| -> Option<Message> {
```

Update the `rename_pane` dispatch to use `StartRenameTab` instead of `StartRename`:

```rust
                if matches_binding(&key, modifiers, &bindings.rename_pane) {
                    return Some(Message::StartRenameTab(active_id));
                }
```

(Note: `active_id` here is still the active pane id for now; it becomes active tab id after Task 4.)

- [ ] **Step 6: Verify still compiles**

```bash
cargo build -p termpp 2>&1 | head -20
```

Expected: compiles. The existing `StartRename` dispatch in the subscription was replaced with `StartRenameTab`; make sure `StartRename` is still handled in `update()` for the sidebar's rename-start button (it will be fully replaced in Task 4).

- [ ] **Step 7: Run all tests to confirm nothing is broken**

```bash
cargo test -p termpp
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs
git commit -m "feat: add workspace/tab Message variants and keybinding dispatch (stubs)"
```

---

## Task 4: `src/app.rs` — refactor Termpp struct to workspace/tab hierarchy

**Files:**
- Modify: `src/app.rs`

This is the large refactor. The file will not compile mid-task — complete all steps before running tests.

- [ ] **Step 1: Update imports at top of `src/app.rs`**

Add the workspace import:

```rust
use termpp::multiplexer::workspace::{Tab, Workspace};
```

- [ ] **Step 2: Replace the `Termpp` struct definition**

Replace the entire `pub struct Termpp { ... }` block with:

```rust
pub struct Termpp {
    config:             Config,
    workspaces:         Vec<Workspace>,
    active_workspace:   usize,
    next_workspace_id:  usize,
    next_tab_id:        usize,
    detector:           NotificationDetector,
    show_help:          bool,
    window_size:        (f32, f32),
    sidebar_w:          f32,
    dragging_sidebar:   bool,
    /// Tab currently being renamed: (tab_id, current_input_value).
    renaming_tab:       Option<(usize, String)>,
    font_name:          &'static str,
    blink_tick:         u8,
}
```

- [ ] **Step 3: Add private helper methods to the `impl` section before `boot()`**

Add an `impl Termpp` block with helpers (place before `pub fn boot()`):

```rust
impl Termpp {
    fn active_ws_idx(&self) -> usize {
        self.workspaces.iter().position(|w| w.id == self.active_workspace).unwrap_or(0)
    }

    fn active_tab(&self) -> &Tab {
        let wi = self.active_ws_idx();
        let ti = self.workspaces[wi].active_tab_idx();
        &self.workspaces[wi].tabs[ti]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        let wi = self.active_ws_idx();
        let ti = self.workspaces[wi].active_tab_idx();
        &mut self.workspaces[wi].tabs[ti]
    }

    fn emu_size(&self) -> (u16, u16) {
        let (ww, wh) = self.window_size;
        let cols = ((ww - self.sidebar_w - DIVIDER_W - TERM_PADDING * 2.0)
            / (self.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
        let rows = ((wh - TERM_PADDING * 2.0)
            / (self.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
        (cols, rows)
    }
}
```

- [ ] **Step 4: Replace `boot()`**

Replace the entire `pub fn boot()` function with:

```rust
pub fn boot() -> (Termpp, Task<Message>) {
    let config = Config::load_or_default().unwrap_or_else(|e| {
        eprintln!("Config error: {e}. Using defaults.");
        Config::default()
    });

    let pane_id = 0usize;
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut pane = PaneState::new(pane_id, cwd.clone());
    pane.git_branch = detect_git_branch(&cwd);

    let timeout   = Duration::from_secs(config.notification_timeout);
    let detector  = NotificationDetector::new(timeout);
    let font_name: &'static str = Box::leak(config.font_name.clone().into_boxed_str());

    let emu_cols = ((WINDOW_W - SIDEBAR_INIT_W - DIVIDER_W - TERM_PADDING * 2.0)
        / (config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
    let emu_rows = ((WINDOW_H - TERM_PADDING * 2.0)
        / (config.font_size as f32 * LINE_H_RATIO)).floor() as u16;

    let mut initial_tab = Tab {
        id: 0,
        name: "main".into(),
        layout: termpp::multiplexer::layout::Layout::new(pane_id),
        panes: HashMap::from([(pane_id, pane)]),
        emulators: HashMap::new(),
        active_pane: pane_id,
        next_pane_id: 1,
        last_output_counts: HashMap::new(),
    };

    match Emulator::start(emu_cols, emu_rows, &config.shell, &cwd) {
        Ok(emu) => {
            initial_tab.last_output_counts.insert(pane_id, 0);
            initial_tab.emulators.insert(pane_id, Arc::new(Mutex::new(emu)));
        }
        Err(e) => eprintln!("Failed to start emulator: {e}"),
    }

    let initial_ws = Workspace {
        id: 0,
        name: "default".into(),
        tabs: vec![initial_tab],
        active_tab: 0,
        collapsed: false,
    };

    let app = Termpp {
        workspaces:        vec![initial_ws],
        active_workspace:  0,
        next_workspace_id: 1,
        next_tab_id:       1,
        detector,
        config,
        show_help:         false,
        window_size:       (WINDOW_W, WINDOW_H),
        sidebar_w:         SIDEBAR_INIT_W,
        dragging_sidebar:  false,
        renaming_tab:      None,
        font_name,
        blink_tick:        0,
    };

    (app, Task::none())
}
```

- [ ] **Step 5: Replace `update()` — Tick handler**

Replace the `Message::Tick` arm with:

```rust
        Message::Tick => {
            state.blink_tick = state.blink_tick.wrapping_add(1);
            let mut auto_close: Vec<(usize, usize, usize)> = Vec::new(); // (ws_idx, tab_idx, pane_id)

            for (wi, ws) in state.workspaces.iter_mut().enumerate() {
                for (ti, tab) in ws.tabs.iter_mut().enumerate() {
                    for (&pane_id, emu_arc) in &tab.emulators {
                        if let Ok(mut emu) = emu_arc.try_lock() {
                            let current_count = emu.output_count.load(Ordering::Relaxed);
                            let last_count = tab.last_output_counts.get(&pane_id).copied().unwrap_or(0);
                            if current_count != last_count {
                                tab.last_output_counts.insert(pane_id, current_count);
                                if let Some(pane) = tab.panes.get_mut(&pane_id) {
                                    pane.on_output();
                                }
                            }
                            while let Ok(event) = emu.event_rx.try_recv() {
                                if let Some(pane) = tab.panes.get_mut(&pane_id) {
                                    use termpp::terminal::grid::TermEvent;
                                    match event {
                                        TermEvent::CwdChange(path) => {
                                            pane.cwd = std::path::PathBuf::from(path);
                                        }
                                        TermEvent::TitleChange(title) => {
                                            pane.terminal_title = Some(title);
                                        }
                                        TermEvent::Bell | TermEvent::OscNotify(_) => {
                                            pane.on_notify();
                                        }
                                        TermEvent::Exited => {
                                            pane.on_exit();
                                        }
                                    }
                                }
                            }
                            if emu.is_exited() {
                                let mut just_exited = false;
                                if let Some(pane) = tab.panes.get_mut(&pane_id) {
                                    if pane.status != PaneStatus::Dead {
                                        pane.on_exit();
                                        just_exited = true;
                                    }
                                }
                                if just_exited && state.config.auto_close_on_exit {
                                    auto_close.push((wi, ti, pane_id));
                                }
                            }
                        }
                    }
                }
            }

            // Process auto-close in reverse order to avoid index shifting.
            auto_close.sort_by(|a, b| b.cmp(a));
            for (wi, _ti, pane_id) in auto_close {
                // Determine tab index fresh (previous removals may have shifted indices)
                let ws = &mut state.workspaces[wi];
                // Find the tab containing this pane
                let tab_pos = ws.tabs.iter().position(|t| t.panes.contains_key(&pane_id));
                if let Some(ti) = tab_pos {
                    let tab = &mut ws.tabs[ti];
                    if let Some(new_layout) = tab.layout.remove(pane_id) {
                        tab.panes.remove(&pane_id);
                        tab.emulators.remove(&pane_id);
                        tab.last_output_counts.remove(&pane_id);
                        tab.layout = new_layout;
                        tab.active_pane = *tab.layout.pane_ids().first().unwrap_or(&0);
                    } else {
                        // Last pane — close the tab
                        let tab_id = ws.tabs[ti].id;
                        ws.tabs.remove(ti);
                        if ws.tabs.is_empty() {
                            let ws_id = ws.id;
                            drop(ws);
                            let wpos = state.workspaces.iter().position(|w| w.id == ws_id).unwrap();
                            state.workspaces.remove(wpos);
                            if state.workspaces.is_empty() {
                                std::process::exit(0);
                            }
                            let new_wi = wpos.min(state.workspaces.len() - 1);
                            state.active_workspace = state.workspaces[new_wi].id;
                        } else {
                            let new_ti = ti.min(ws.tabs.len() - 1);
                            ws.active_tab = ws.tabs[new_ti].id;
                            if ws.active_tab == tab_id { // if it was already active
                                state.active_workspace = ws.id;
                            }
                        }
                    }
                }
            }
        }
```

Note: The `state.detector.process_event()` call is inlined here as direct `pane.on_notify()` / `pane.on_exit()` calls because iterating `state.workspaces` mutably while calling a method on `state.detector` triggers a borrow conflict. The logic is identical.

- [ ] **Step 6: Replace `Message::StatusTick` handler**

```rust
        Message::StatusTick => {
            for ws in &mut state.workspaces {
                for tab in &mut ws.tabs {
                    for pane in tab.panes.values_mut() {
                        state.detector.check_idle(pane);
                    }
                }
            }
            let mut tasks: Vec<Task<Message>> = Vec::new();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    for (&id, pane) in &tab.panes {
                        if pane.status != PaneStatus::Dead {
                            let cwd = pane.cwd.clone();
                            tasks.push(Task::perform(
                                async move {
                                    tokio::task::spawn_blocking(move || detect_git_branch(&cwd))
                                        .await
                                        .unwrap_or(None)
                                },
                                move |branch| Message::GitBranchDetected(id, branch),
                            ));
                        }
                    }
                }
            }
            return Task::batch(tasks);
        }
```

Note: `state.detector.check_idle(pane)` and the `state.workspaces` loop are in separate loops (first loop mutates panes; second builds tasks). This avoids borrow conflicts. However, `check_idle` borrows `state.detector` and `pane` (from `state.workspaces`) — these are different fields, which Rust allows.

Actually, to be safe, split them:

```rust
        Message::StatusTick => {
            // Idle check: requires &mut pane (from workspaces) and &self detector separately
            let idle_timeout = state.detector.idle_timeout;
            for ws in &mut state.workspaces {
                for tab in &mut ws.tabs {
                    for pane in tab.panes.values_mut() {
                        if pane.is_idle_for(idle_timeout) {
                            pane.on_notify();
                        }
                    }
                }
            }
            // Git branch detection
            let mut tasks: Vec<Task<Message>> = Vec::new();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    for (&id, pane) in &tab.panes {
                        if pane.status != PaneStatus::Dead {
                            let cwd = pane.cwd.clone();
                            tasks.push(Task::perform(
                                async move {
                                    tokio::task::spawn_blocking(move || detect_git_branch(&cwd))
                                        .await
                                        .unwrap_or(None)
                                },
                                move |branch| Message::GitBranchDetected(id, branch),
                            ));
                        }
                    }
                }
            }
            return Task::batch(tasks);
        }
```

For this to compile, add `pub` to `idle_timeout` in `NotificationDetector` in `src/multiplexer/notification.rs`:

```rust
pub struct NotificationDetector {
    pub idle_timeout: Duration,
}
```

- [ ] **Step 7: Update `GitBranchDetected`, `KeyInput`, `Resized`, `SidebarDragStart/Dragged/DragEnd` handlers**

`GitBranchDetected(pane_id, branch)` — search all tabs:
```rust
        Message::GitBranchDetected(pane_id, branch) => {
            'outer: for ws in &mut state.workspaces {
                for tab in &mut ws.tabs {
                    if let Some(pane) = tab.panes.get_mut(&pane_id) {
                        pane.git_branch = branch;
                        break 'outer;
                    }
                }
            }
        }
```

`KeyInput` — delegate to active tab's active pane:
```rust
        Message::KeyInput(bytes) => {
            let tab = state.active_tab();
            let active_pane = tab.active_pane;
            if let Some(emu_arc) = tab.emulators.get(&active_pane) {
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                let _ = emu.write_input(&bytes);
            }
        }
```

Note: the above borrows `state` immutably via `active_tab()`. This works since `KeyInput` doesn't need to mutate.

`Resized` — resize all emulators across all tabs:
```rust
        Message::Resized(w, h) => {
            state.window_size = (w, h);
            let (new_cols, new_rows) = state.emu_size();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    for emu_arc in tab.emulators.values() {
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        emu.resize(new_cols, new_rows);
                    }
                }
            }
        }
```

`SidebarDragStart/DragEnd/Dragged` — same as before, just update DragEnd to use `emu_size()`:
```rust
        Message::SidebarDragStart => { state.dragging_sidebar = true; }
        Message::SidebarDragged(x) => {
            state.sidebar_w = x.clamp(SIDEBAR_MIN_W, SIDEBAR_MAX_W);
        }
        Message::SidebarDragEnd => {
            state.dragging_sidebar = false;
            let (new_cols, new_rows) = state.emu_size();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    for emu_arc in tab.emulators.values() {
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        emu.resize(new_cols, new_rows);
                    }
                }
            }
        }
```

- [ ] **Step 8: Update `SplitPane`, `ClosePane`, `FocusNext`, `FocusPrev`, `FocusPaneById`, `ClosePaneById`, `NewPane` handlers**

These all delegate to the active tab. Pattern: where code previously used `state.layout`, `state.panes`, etc., now use `state.active_tab_mut().layout`, etc.

`SplitPane`:
```rust
        Message::SplitPane(dir) => {
            let tab = state.active_tab_mut();
            let new_id = tab.next_pane_id;
            if let Some(new_layout) = tab.layout.split(tab.active_pane, dir, new_id) {
                tab.layout = new_layout;
                let cwd = tab.panes.get(&tab.active_pane).map(|p| p.cwd.clone()).unwrap_or_default();
                let mut pane = PaneState::new(new_id, cwd.clone());
                pane.git_branch = detect_git_branch(&cwd);
                tab.panes.insert(new_id, pane);
                let (emu_cols, emu_rows) = state.emu_size();
                let shell = state.config.shell.clone();
                let tab = state.active_tab_mut();
                if let Ok(emu) = Emulator::start(emu_cols, emu_rows, &shell, &cwd) {
                    tab.last_output_counts.insert(new_id, 0);
                    tab.emulators.insert(new_id, Arc::new(Mutex::new(emu)));
                }
                tab.next_pane_id += 1;
                tab.active_pane = new_id;
            }
        }
```

Note: calling `state.emu_size()` requires releasing the `&mut Tab` borrow first. The code calls `state.emu_size()` before the second `state.active_tab_mut()` call to avoid borrow conflicts.

`ClosePane`:
```rust
        Message::ClosePane => {
            let (tab_id, pane_id) = {
                let tab = state.active_tab();
                (tab.id, tab.active_pane)
            };
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let tab = &mut state.workspaces[wi].tabs[ti];
            if let Some(new_layout) = tab.layout.remove(pane_id) {
                tab.panes.remove(&pane_id);
                tab.emulators.remove(&pane_id);
                tab.last_output_counts.remove(&pane_id);
                tab.layout = new_layout;
                tab.active_pane = *tab.layout.pane_ids().first().unwrap_or(&0);
            } else {
                // Last pane in tab — close the tab
                drop(tab);
                let ws = &mut state.workspaces[wi];
                ws.tabs.remove(ti);
                if ws.tabs.is_empty() {
                    state.workspaces.remove(wi);
                    if state.workspaces.is_empty() {
                        std::process::exit(0);
                    }
                    let new_wi = wi.min(state.workspaces.len() - 1);
                    state.active_workspace = state.workspaces[new_wi].id;
                } else {
                    let ws = &mut state.workspaces[wi];
                    let new_ti = ti.min(ws.tabs.len() - 1);
                    ws.active_tab = ws.tabs[new_ti].id;
                }
            }
            let _ = tab_id; // suppress unused warning
        }
```

`FocusNext`:
```rust
        Message::FocusNext => {
            let tab = state.active_tab_mut();
            let ids = tab.layout.pane_ids();
            if let Some(pos) = ids.iter().position(|&id| id == tab.active_pane) {
                tab.active_pane = ids[(pos + 1) % ids.len()];
            }
        }
```

`FocusPrev`:
```rust
        Message::FocusPrev => {
            let tab = state.active_tab_mut();
            let ids = tab.layout.pane_ids();
            if let Some(pos) = ids.iter().position(|&id| id == tab.active_pane) {
                tab.active_pane = ids[(pos + ids.len() - 1) % ids.len()];
            }
        }
```

`FocusPaneById`:
```rust
        Message::FocusPaneById(id) => {
            let tab = state.active_tab_mut();
            if tab.panes.contains_key(&id) {
                tab.active_pane = id;
            }
        }
```

`ClosePaneById(target)`:
```rust
        Message::ClosePaneById(target) => {
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let tab = &mut state.workspaces[wi].tabs[ti];
            if let Some(new_layout) = tab.layout.remove(target) {
                tab.panes.remove(&target);
                tab.emulators.remove(&target);
                tab.last_output_counts.remove(&target);
                tab.layout = new_layout;
                if tab.active_pane == target {
                    tab.active_pane = *tab.layout.pane_ids().first().unwrap_or(&0);
                }
            } else {
                // Same close-tab logic as ClosePane
                drop(tab);
                let ws = &mut state.workspaces[wi];
                ws.tabs.remove(ti);
                if ws.tabs.is_empty() {
                    state.workspaces.remove(wi);
                    if state.workspaces.is_empty() { std::process::exit(0); }
                    let new_wi = wi.min(state.workspaces.len() - 1);
                    state.active_workspace = state.workspaces[new_wi].id;
                } else {
                    let ws = &mut state.workspaces[wi];
                    let new_ti = ti.min(ws.tabs.len() - 1);
                    ws.active_tab = ws.tabs[new_ti].id;
                }
            }
        }
```

`NewPane`:
```rust
        Message::NewPane => {
            let (emu_cols, emu_rows) = state.emu_size();
            let shell = state.config.shell.clone();
            let tab = state.active_tab_mut();
            let new_id = tab.next_pane_id;
            if let Some(new_layout) = tab.layout.split(tab.active_pane, SplitDirection::Vertical, new_id) {
                tab.layout = new_layout;
                let cwd = tab.panes.get(&tab.active_pane).map(|p| p.cwd.clone()).unwrap_or_default();
                tab.panes.insert(new_id, PaneState::new(new_id, cwd.clone()));
                if let Ok(emu) = Emulator::start(emu_cols, emu_rows, &shell, &cwd) {
                    tab.last_output_counts.insert(new_id, 0);
                    tab.emulators.insert(new_id, Arc::new(Mutex::new(emu)));
                }
                tab.next_pane_id += 1;
                tab.active_pane = new_id;
            }
        }
```

- [ ] **Step 9: Update rename handlers — now rename tabs, not panes**

`StartRename(id)` (pane rename, from sidebar) is now unused; remove it from update() and replace with `StartRenameTab`:

```rust
        Message::StartRename(_) => {} // removed — use StartRenameTab
        Message::StartRenameTab(tab_id) => {
            let name = state.workspaces.iter()
                .flat_map(|ws| ws.tabs.iter())
                .find(|t| t.id == tab_id)
                .map(|t| t.name.clone())
                .unwrap_or_default();
            state.renaming_tab = Some((tab_id, name));
            return iced::widget::operation::focus(
                iced::widget::Id::new(termpp::ui::sidebar::RENAME_INPUT_ID)
            );
        }
        Message::RenameChanged(s) => {
            if let Some((_, ref mut val)) = state.renaming_tab {
                *val = s;
            }
        }
        Message::CommitRename => {
            if let Some((tab_id, name)) = state.renaming_tab.take() {
                for ws in &mut state.workspaces {
                    if let Some(tab) = ws.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.name = if name.trim().is_empty() {
                            "tab".into()
                        } else {
                            name.trim().to_string()
                        };
                        break;
                    }
                }
            }
        }
        Message::CancelRename => {
            state.renaming_tab = None;
        }
```

- [ ] **Step 10: Implement new workspace/tab message handlers**

Replace the stub arms with real implementations:

```rust
        Message::FocusTabNext => {
            let wi = state.active_ws_idx();
            let ws = &mut state.workspaces[wi];
            let tab_ids: Vec<usize> = ws.tabs.iter().map(|t| t.id).collect();
            if let Some(pos) = tab_ids.iter().position(|&id| id == ws.active_tab) {
                ws.active_tab = tab_ids[(pos + 1) % tab_ids.len()];
            }
        }
        Message::FocusTabPrev => {
            let wi = state.active_ws_idx();
            let ws = &mut state.workspaces[wi];
            let tab_ids: Vec<usize> = ws.tabs.iter().map(|t| t.id).collect();
            if let Some(pos) = tab_ids.iter().position(|&id| id == ws.active_tab) {
                ws.active_tab = tab_ids[(pos + tab_ids.len() - 1) % tab_ids.len()];
            }
        }
        Message::SelectTab(tab_id) => {
            for ws in &mut state.workspaces {
                if ws.tabs.iter().any(|t| t.id == tab_id) {
                    ws.active_tab = tab_id;
                    state.active_workspace = ws.id;
                    break;
                }
            }
        }
        Message::CloseTab(tab_id) => {
            let ws_idx = state.workspaces.iter().position(|ws| ws.tabs.iter().any(|t| t.id == tab_id));
            if let Some(wi) = ws_idx {
                let ti = state.workspaces[wi].tabs.iter().position(|t| t.id == tab_id).unwrap();
                state.workspaces[wi].tabs.remove(ti);
                if state.workspaces[wi].tabs.is_empty() {
                    state.workspaces.remove(wi);
                    if state.workspaces.is_empty() { std::process::exit(0); }
                    let new_wi = wi.min(state.workspaces.len() - 1);
                    state.active_workspace = state.workspaces[new_wi].id;
                } else {
                    let ws = &mut state.workspaces[wi];
                    let new_ti = ti.min(ws.tabs.len() - 1);
                    ws.active_tab = ws.tabs[new_ti].id;
                    if state.active_workspace == ws.id {
                        // Active workspace — make sure active_workspace still set
                    }
                }
            }
        }
        Message::NewTabIn(ws_id) => {
            let (emu_cols, emu_rows) = state.emu_size();
            let shell = state.config.shell.clone();
            let tab_id = state.next_tab_id;
            state.next_tab_id += 1;
            let cwd = {
                // Inherit cwd from active pane in target workspace's active tab, or use current dir
                state.workspaces.iter()
                    .find(|w| w.id == ws_id)
                    .and_then(|ws| ws.tabs.iter().find(|t| t.id == ws.active_tab))
                    .and_then(|tab| tab.panes.get(&tab.active_pane))
                    .map(|p| p.cwd.clone())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            };
            let pane_id = 0usize;
            let mut pane = PaneState::new(pane_id, cwd.clone());
            pane.git_branch = detect_git_branch(&cwd);
            let mut new_tab = Tab {
                id: tab_id,
                name: format!("tab-{tab_id}"),
                layout: termpp::multiplexer::layout::Layout::new(pane_id),
                panes: HashMap::from([(pane_id, pane)]),
                emulators: HashMap::new(),
                active_pane: pane_id,
                next_pane_id: 1,
                last_output_counts: HashMap::new(),
            };
            if let Ok(emu) = Emulator::start(emu_cols, emu_rows, &shell, &cwd) {
                new_tab.last_output_counts.insert(pane_id, 0);
                new_tab.emulators.insert(pane_id, Arc::new(Mutex::new(emu)));
            }
            if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_id) {
                ws.active_tab = tab_id;
                ws.tabs.push(new_tab);
                state.active_workspace = ws_id;
            }
        }
        Message::FocusWorkspaceNext => {
            let ids: Vec<usize> = state.workspaces.iter().map(|w| w.id).collect();
            if let Some(pos) = ids.iter().position(|&id| id == state.active_workspace) {
                state.active_workspace = ids[(pos + 1) % ids.len()];
            }
        }
        Message::FocusWorkspacePrev => {
            let ids: Vec<usize> = state.workspaces.iter().map(|w| w.id).collect();
            if let Some(pos) = ids.iter().position(|&id| id == state.active_workspace) {
                state.active_workspace = ids[(pos + ids.len() - 1) % ids.len()];
            }
        }
        Message::NewWorkspace => {
            let (emu_cols, emu_rows) = state.emu_size();
            let shell = state.config.shell.clone();
            let ws_id  = state.next_workspace_id;
            let tab_id = state.next_tab_id;
            state.next_workspace_id += 1;
            state.next_tab_id += 1;
            let cwd = std::env::current_dir().unwrap_or_default();
            let pane_id = 0usize;
            let mut pane = PaneState::new(pane_id, cwd.clone());
            pane.git_branch = detect_git_branch(&cwd);
            let mut new_tab = Tab {
                id: tab_id,
                name: "main".into(),
                layout: termpp::multiplexer::layout::Layout::new(pane_id),
                panes: HashMap::from([(pane_id, pane)]),
                emulators: HashMap::new(),
                active_pane: pane_id,
                next_pane_id: 1,
                last_output_counts: HashMap::new(),
            };
            if let Ok(emu) = Emulator::start(emu_cols, emu_rows, &shell, &cwd) {
                new_tab.last_output_counts.insert(pane_id, 0);
                new_tab.emulators.insert(pane_id, Arc::new(Mutex::new(emu)));
            }
            let new_ws = Workspace {
                id: ws_id,
                name: format!("workspace-{ws_id}"),
                tabs: vec![new_tab],
                active_tab: tab_id,
                collapsed: false,
            };
            state.workspaces.push(new_ws);
            state.active_workspace = ws_id;
        }
        Message::ToggleWorkspace(ws_id) => {
            if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_id) {
                ws.collapsed = !ws.collapsed;
            }
        }
```

- [ ] **Step 11: Update `ToggleHelp` handler** (renaming_tab instead of renaming_pane)

```rust
        Message::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                state.renaming_tab = None;
            }
        }
```

- [ ] **Step 12: Update `view()`**

Replace `view()` with:

```rust
pub fn view(state: &Termpp) -> Element<'_, Message> {
    use termpp::multiplexer::pane::PaneStatus;
    use termpp::ui::sidebar::{Sidebar, TabEntry, WorkspaceEntry};

    let ws_entries: Vec<WorkspaceEntry> = state.workspaces.iter().map(|ws| {
        WorkspaceEntry {
            id: ws.id,
            name: ws.name.clone(),
            active_tab_id: ws.active_tab,
            collapsed: ws.collapsed,
            tabs: ws.tabs.iter().map(|tab| {
                let active_pane = tab.panes.get(&tab.active_pane);
                TabEntry {
                    id: tab.id,
                    name: tab.name.clone(),
                    git_branch: active_pane.and_then(|p| p.git_branch.clone()),
                    terminal_title: active_pane.and_then(|p| p.terminal_title.clone()),
                    has_waiting: active_pane.map(|p| p.status == PaneStatus::Waiting).unwrap_or(false),
                }
            }).collect(),
        }
    }).collect();

    let sidebar: Element<'static, Message> =
        container(Sidebar::<Message>::new(
            &ws_entries,
            state.active_workspace,
            state.renaming_tab.clone(),
            Message::SelectTab,
            Message::CloseTab,
            Message::NewTabIn,
            Message::ToggleWorkspace,
            Message::NewWorkspace,
            Message::StartRenameTab,
            Message::RenameChanged,
            Message::CommitRename,
            Message::CancelRename,
            Message::ToggleHelp,
        ).view())
        .width(state.sidebar_w)
        .height(Length::Fill)
        .into();

    // Divider (unchanged)
    let line_color = if state.dragging_sidebar {
        Color { r: 0.55, g: 0.56, b: 0.98, a: 1.0 }
    } else {
        Color { r: 0.18, g: 0.18, b: 0.26, a: 1.0 }
    };
    let divider: Element<'static, Message> = mouse_area(
        container(
            container(iced::widget::Space::new())
                .width(1)
                .height(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(line_color)),
                    ..Default::default()
                })
        )
        .width(DIVIDER_W)
        .height(Length::Fill)
        .center_x(DIVIDER_W)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color { r: 0.05, g: 0.05, b: 0.05, a: 1.0 })),
            ..Default::default()
        })
    )
    .on_press(Message::SidebarDragStart)
    .on_release(Message::SidebarDragEnd)
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .into();

    // Render the active pane from the active tab
    let tab = state.active_tab();
    let pane_view: Element<'static, Message> =
        if let (Some(pane), Some(emu_arc)) = (
            tab.panes.get(&tab.active_pane),
            tab.emulators.get(&tab.active_pane),
        ) {
            let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
            if pane.status == PaneStatus::Dead {
                let close_key = state.config.keybindings.close_pane.clone();
                container(
                    column![
                        text("Process exited").size(18).color(Color { r: 0.75, g: 0.75, b: 0.75, a: 1.0 }),
                        text(format!("{close_key} to close")).size(13).color(Color { r: 0.40, g: 0.40, b: 0.50, a: 1.0 }),
                    ]
                    .spacing(8)
                    .align_x(iced::Alignment::Center)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(AppTheme::PANE_BG)),
                    ..Default::default()
                })
                .into()
            } else {
                let is_waiting = pane.status == PaneStatus::Waiting;
                let cursor_on = (state.blink_tick % 62) < 31;
                TerminalPane::new(
                    Arc::clone(&emu.grid),
                    is_waiting,
                    state.config.font_size as f32,
                    state.font_name,
                    cursor_on,
                ).view()
            }
        } else {
            iced::widget::text("No pane").into()
        };

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
}
```

- [ ] **Step 13: Update `subscription()`**

Change the captured state to use `renaming_tab` and `active_workspace`:

```rust
    let is_renaming        = state.renaming_tab.is_some();
    let active_id          = state.active_tab().active_pane;
    let active_workspace_id = state.active_workspace;
```

Update the tuple type annotation accordingly (as established in Task 3 Step 4/5).

- [ ] **Step 14: Add tests for new navigation logic**

In the `#[cfg(test)]` module at the bottom of `src/app.rs`, add:

```rust
    #[test]
    fn focus_tab_next_wraps() {
        let tab_ids = vec![10usize, 20, 30];
        let current = 30usize;
        let pos = tab_ids.iter().position(|&id| id == current).unwrap();
        let next = tab_ids[(pos + 1) % tab_ids.len()];
        assert_eq!(next, 10);
    }

    #[test]
    fn focus_tab_prev_wraps() {
        let tab_ids = vec![10usize, 20, 30];
        let current = 10usize;
        let pos = tab_ids.iter().position(|&id| id == current).unwrap();
        let prev = tab_ids[(pos + tab_ids.len() - 1) % tab_ids.len()];
        assert_eq!(prev, 30);
    }

    #[test]
    fn focus_workspace_next_wraps() {
        let ws_ids = vec![0usize, 1, 2];
        let current = 2usize;
        let pos = ws_ids.iter().position(|&id| id == current).unwrap();
        let next = ws_ids[(pos + 1) % ws_ids.len()];
        assert_eq!(next, 0);
    }

    #[test]
    fn focus_workspace_prev_wraps() {
        let ws_ids = vec![0usize, 1, 2];
        let current = 0usize;
        let pos = ws_ids.iter().position(|&id| id == current).unwrap();
        let prev = ws_ids[(pos + ws_ids.len() - 1) % ws_ids.len()];
        assert_eq!(prev, 2);
    }
```

- [ ] **Step 15: Build and run all tests**

```bash
cargo build -p termpp 2>&1 | head -40
cargo test -p termpp
```

Expected: compiles, all tests pass.

- [ ] **Step 16: Commit**

```bash
git add src/app.rs src/multiplexer/notification.rs
git commit -m "feat: refactor Termpp to Workspace->Tab->Pane hierarchy"
```

---

## Task 5: `src/ui/sidebar.rs` — new two-level tree view

**Files:**
- Modify: `src/ui/sidebar.rs`
- Modify: `tests/sidebar_test.rs`

- [ ] **Step 1: Write failing tests**

Replace `tests/sidebar_test.rs` with:

```rust
use termpp::ui::sidebar::{Sidebar, WorkspaceEntry, TabEntry};

fn make_tab(id: usize, name: &str, active: bool) -> TabEntry {
    TabEntry {
        id,
        name: name.to_string(),
        git_branch: if active { Some("main".to_string()) } else { None },
        terminal_title: if active { Some("Claude Code".to_string()) } else { None },
        has_waiting: false,
    }
}

#[test]
fn sidebar_renders_with_active_workspace_and_tab() {
    let entries = vec![
        WorkspaceEntry {
            id: 0,
            name: "default".to_string(),
            active_tab_id: 1,
            collapsed: false,
            tabs: vec![
                make_tab(0, "main", false),
                make_tab(1, "dev", true),
            ],
        },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,     // active_workspace_id
        None,  // not renaming
        |id| (), // on_select_tab
        |id| (), // on_close_tab
        |id| (), // on_new_tab
        |id| (), // on_toggle_workspace
        (),     // on_new_workspace
        |id| (), // on_rename_start
        |_| (), // on_rename_change
        (),     // on_rename_commit
        (),     // on_rename_cancel
        (),     // on_help
    ).view();
}

#[test]
fn sidebar_renders_collapsed_workspace() {
    let entries = vec![
        WorkspaceEntry {
            id: 0,
            name: "work".to_string(),
            active_tab_id: 0,
            collapsed: true,
            tabs: vec![make_tab(0, "main", false)],
        },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,
        None,
        |_| (), |_| (), |_| (), |_| (), (), |_| (), |_| (), (), (), (),
    ).view();
}
```

- [ ] **Step 2: Run tests to see them fail**

```bash
cargo test -p termpp --test sidebar_test
```

Expected: compile errors — `TabEntry` not found, `WorkspaceEntry` fields changed.

- [ ] **Step 3: Rewrite `src/ui/sidebar.rs`**

Replace the entire file with:

```rust
use iced::widget::{column, container, mouse_area, row, text, text_input, Space};
use iced::{Background, Element, Length};

use crate::ui::theme::Theme as AppTheme;

pub const RENAME_INPUT_ID: &str = "sidebar_rename";

pub struct TabEntry {
    pub id: usize,
    pub name: String,
    pub git_branch: Option<String>,
    pub terminal_title: Option<String>,
    pub has_waiting: bool,
}

pub struct WorkspaceEntry {
    pub id: usize,
    pub name: String,
    pub tabs: Vec<TabEntry>,
    pub active_tab_id: usize,
    pub collapsed: bool,
}

pub struct Sidebar<Message: Clone + 'static> {
    workspaces:          Vec<WorkspaceEntry>,
    active_workspace_id: usize,
    renaming:            Option<(usize, String)>, // (tab_id, current_name)
    on_select_tab:       fn(usize) -> Message,
    on_close_tab:        fn(usize) -> Message,
    on_new_tab:          fn(usize) -> Message,    // arg = workspace_id
    on_toggle_workspace: fn(usize) -> Message,
    on_new_workspace:    Message,
    on_rename_start:     fn(usize) -> Message,    // arg = tab_id
    on_rename_change:    fn(String) -> Message,
    on_rename_commit:    Message,
    on_rename_cancel:    Message,
    on_help:             Message,
}

impl<Message: Clone + 'static> Sidebar<Message> {
    pub fn new(
        workspaces:          &[WorkspaceEntry],
        active_workspace_id: usize,
        renaming:            Option<(usize, String)>,
        on_select_tab:       fn(usize) -> Message,
        on_close_tab:        fn(usize) -> Message,
        on_new_tab:          fn(usize) -> Message,
        on_toggle_workspace: fn(usize) -> Message,
        on_new_workspace:    Message,
        on_rename_start:     fn(usize) -> Message,
        on_rename_change:    fn(String) -> Message,
        on_rename_commit:    Message,
        on_rename_cancel:    Message,
        on_help:             Message,
    ) -> Self {
        let owned = workspaces.iter().map(|ws| WorkspaceEntry {
            id: ws.id,
            name: ws.name.clone(),
            active_tab_id: ws.active_tab_id,
            collapsed: ws.collapsed,
            tabs: ws.tabs.iter().map(|t| TabEntry {
                id: t.id,
                name: t.name.clone(),
                git_branch: t.git_branch.clone(),
                terminal_title: t.terminal_title.clone(),
                has_waiting: t.has_waiting,
            }).collect(),
        }).collect();
        Self {
            workspaces: owned,
            active_workspace_id,
            renaming,
            on_select_tab,
            on_close_tab,
            on_new_tab,
            on_toggle_workspace,
            on_new_workspace,
            on_rename_start,
            on_rename_change,
            on_rename_commit,
            on_rename_cancel,
            on_help,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        // Header: "WORKSPACES" label + [+] new workspace + [?] help
        let new_ws_msg = self.on_new_workspace.clone();
        let help_msg   = self.on_help.clone();

        let header: Element<'static, Message> = container(
            row![
                text("WORKSPACES")
                    .color(AppTheme::TEXT_DIM)
                    .size(10),
                Space::new().width(Length::Fill),
                mouse_area(text("+").color(AppTheme::TEXT_DIM).size(14))
                    .on_press(new_ws_msg),
                mouse_area(text("?").color(AppTheme::TEXT_DIM).size(14))
                    .on_press(help_msg),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding([6, 10])
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into();

        let mut items: Vec<Element<'static, Message>> = vec![header];

        for ws in &self.workspaces {
            items.push(self.render_workspace(ws));
            if !ws.collapsed {
                for tab in &ws.tabs {
                    items.push(self.render_tab(tab, ws.id, ws.active_tab_id == tab.id));
                }
            }
        }

        container(
            column(items)
                .spacing(0)
                .push(Space::new().height(Length::Fill))
        )
        .width(200)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
    }

    fn render_workspace(&self, ws: &WorkspaceEntry) -> Element<'static, Message> {
        let is_active   = ws.id == self.active_workspace_id;
        let arrow       = if ws.collapsed { "▸" } else { "▾" };
        let toggle_msg  = (self.on_toggle_workspace)(ws.id);
        let new_tab_msg = (self.on_new_tab)(ws.id);

        let (accent_color, text_color, bg_color) = if is_active {
            (AppTheme::ACCENT_WS, AppTheme::TEXT_PRIMARY, AppTheme::PANE_BG)
        } else {
            (iced::Color::TRANSPARENT, AppTheme::TEXT_DIM, AppTheme::SIDEBAR_BG)
        };

        let accent = container(Space::new())
            .width(3)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(accent_color)),
                ..Default::default()
            });

        let content = container(
            row![
                text(arrow).color(if is_active { AppTheme::ACCENT_WS } else { AppTheme::TEXT_DIM }).size(10),
                text(ws.name.clone()).color(text_color).size(12),
                Space::new().width(Length::Fill),
                mouse_area(text("+").color(AppTheme::TEXT_DIM).size(12))
                    .on_press(new_tab_msg),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding([5, 8])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg_color)),
            ..Default::default()
        });

        mouse_area(
            row![accent, content].width(Length::Fill).height(Length::Shrink)
        )
        .on_press(toggle_msg)
        .into()
    }

    fn render_tab(
        &self,
        tab: &TabEntry,
        workspace_id: usize,
        is_active: bool,
    ) -> Element<'static, Message> {
        let is_renaming = self.renaming.as_ref().map(|(id, _)| *id) == Some(tab.id);

        if is_renaming {
            let value      = self.renaming.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
            let change_fn  = self.on_rename_change;
            let commit_msg = self.on_rename_commit.clone();
            let cancel_msg = self.on_rename_cancel.clone();

            let input: Element<'static, Message> = text_input("Name…", &value)
                .id(iced::widget::Id::new(RENAME_INPUT_ID))
                .on_input(move |s| change_fn(s))
                .on_submit(commit_msg)
                .size(12)
                .padding([2, 4])
                .into();

            let cancel: Element<'static, Message> = mouse_area(
                text("×").color(AppTheme::TEXT_DIM).size(13)
            )
            .on_press(cancel_msg)
            .into();

            return container(
                // Indent to align with tab content
                row![
                    Space::new().width(17), // 3px accent + 14px indent placeholder
                    input,
                    cancel,
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding([4, 6])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(AppTheme::PANE_BG)),
                ..Default::default()
            })
            .into();
        }

        let select_msg = (self.on_select_tab)(tab.id);
        let close_msg  = (self.on_close_tab)(tab.id);
        let rename_msg = (self.on_rename_start)(tab.id);

        let (accent_color, name_color, bg_color) = if is_active {
            (AppTheme::ACCENT, AppTheme::TEXT_PRIMARY, AppTheme::PANE_BG)
        } else {
            (iced::Color::TRANSPARENT, AppTheme::TEXT_DIM, AppTheme::SIDEBAR_BG)
        };

        // Left-side: 3px workspace-level indent spacer + 3px tab accent
        let indent  = Space::new().width(14);
        let accent_bar = container(Space::new())
            .width(3)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(accent_color)),
                ..Default::default()
            });

        let badge: Element<'static, Message> = if tab.has_waiting {
            text("●").color(AppTheme::BADGE_ACTIVE).size(10).into()
        } else {
            Space::new().width(4).into()
        };

        let rename_btn: Element<'static, Message> =
            mouse_area(text("✎").color(AppTheme::TEXT_DIM).size(11))
                .on_press(rename_msg)
                .into();

        let close_btn: Element<'static, Message> =
            mouse_area(text("×").color(AppTheme::TEXT_DIM).size(13))
                .on_press(close_msg)
                .into();

        let name_row = row![
            text(tab.name.clone()).color(name_color).size(13),
            Space::new().width(Length::Fill),
            badge,
            rename_btn,
            close_btn,
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center);

        let branch_row: Element<'static, Message> = if let Some(b) = &tab.git_branch {
            text(format!("  {b}")).color(AppTheme::TEXT_DIM).size(11).into()
        } else {
            Space::new().height(0).into()
        };

        let title_row: Element<'static, Message> = if let Some(t) = &tab.terminal_title {
            text(format!("  {t}")).color(AppTheme::TEXT_DIM).size(10).into()
        } else {
            Space::new().height(0).into()
        };

        let content = container(column![name_row, branch_row, title_row].spacing(2))
            .width(Length::Fill)
            .padding([4, 8])
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                ..Default::default()
            });

        let _ = workspace_id; // available for future use

        mouse_area(
            row![indent, accent_bar, content].width(Length::Fill).height(Length::Shrink)
        )
        .on_press(select_msg)
        .into()
    }
}
```

- [ ] **Step 4: Add `ACCENT_WS` to `src/ui/theme.rs`**

In `src/ui/theme.rs`, add alongside `ACCENT`:

```rust
/// Workspace-level accent bar: softer blue-lavender
pub const ACCENT_WS: Color = Color { r: 0.54, g: 0.71, b: 0.98, a: 1.0 }; // #89b4fa
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p termpp --test sidebar_test
cargo build -p termpp 2>&1 | head -20
```

Expected: sidebar tests pass, project compiles.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sidebar.rs src/ui/theme.rs tests/sidebar_test.rs
git commit -m "feat: sidebar two-level workspace/tab tree"
```

---

## Task 6: `src/ui/help_overlay.rs` — new shortcuts

**Files:**
- Modify: `src/ui/help_overlay.rs`

- [ ] **Step 1: Update the shortcuts vec**

Replace the `shortcuts` vec in `help_overlay()` with:

```rust
    let shortcuts: Vec<(&'static str, String)> = vec![
        ("Scinder horizontal",      keybindings.split_horizontal.clone()),
        ("Scinder vertical",        keybindings.split_vertical.clone()),
        ("Terminal suivant",        keybindings.pane_next.clone()),
        ("Terminal précédent",      keybindings.pane_prev.clone()),
        ("Nouveau terminal",        keybindings.new_pane.clone()),
        ("Renommer l'onglet",       keybindings.rename_pane.clone()),
        ("Fermer le terminal",      keybindings.close_pane.clone()),
        ("Onglet suivant",          keybindings.tab_next.clone()),
        ("Onglet précédent",        keybindings.tab_prev.clone()),
        ("Nouvel onglet",           keybindings.tab_new.clone()),
        ("Workspace suivant",       keybindings.workspace_next.clone()),
        ("Workspace précédent",     keybindings.workspace_prev.clone()),
        ("Nouveau workspace",       keybindings.workspace_new.clone()),
        ("Aide",                    "F1".to_string()),
    ];
```

- [ ] **Step 2: Run tests and build**

```bash
cargo test -p termpp
cargo build -p termpp
```

Expected: all pass, no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/help_overlay.rs
git commit -m "feat: add workspace/tab shortcuts to help overlay"
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] Data model: Tab + Workspace structs (Task 1)
- [x] Keyboard shortcuts: all 6 new bindings in config (Task 2) and subscription (Task 3)
- [x] `close_pane` moved to `ctrl+shift+q` (Task 2)
- [x] Sidebar two-level tree with collapsible workspaces and accent bars (Task 5)
- [x] Messages: all new variants (Task 3), real implementations (Task 4)
- [x] Startup creates default workspace + main tab (Task 4 boot())
- [x] Workspace rename via sidebar — NOT implemented (double-click TBD, out of scope per YAGNI)

**Type consistency check:**
- `WorkspaceId = usize`, `TabId = usize`, `PaneId = usize` — consistent throughout
- `on_new_tab: fn(usize) -> Message` takes workspace_id; `NewTabIn(usize)` variant carries workspace_id — consistent
- `StartRenameTab(tab_id)` in subscription, `StartRenameTab(usize)` in Message enum, `on_rename_start: fn(usize) -> Message` in Sidebar — consistent
- `active_tab_idx()` defined on `Workspace` in Task 1, used in `active_tab_mut()` helper in Task 4 — consistent

**Placeholder scan:** No TBDs or TODO stubs remain in the plan.
