# Spec: Workspaces + Tabs hierarchy

**Date:** 2026-03-27
**Status:** Approved

## Overview

Introduce a two-level hierarchy above the existing terminal pane: **Workspace → Tab → Pane**. This replaces the current flat pane list and enables logical grouping (by project, context, etc.) as well as multiple tab-level views within each workspace.

The current split-view rendering (multiple panes visible simultaneously) is out of scope for this spec and will be addressed in a follow-up spec.

---

## 1. Data Model

```
App
├── workspaces: Vec<Workspace>
├── active_workspace: WorkspaceId
├── next_workspace_id: usize
└── next_tab_id: usize          ← global, avoids id collisions across workspaces

Workspace
├── id: WorkspaceId (usize)
├── name: String
├── tabs: Vec<Tab>
└── active_tab: TabId

Tab
├── id: TabId (usize)
├── name: String
├── layout: Layout
├── panes: HashMap<PaneId, PaneState>
├── emulators: HashMap<PaneId, Emulator>
├── active_pane: PaneId
└── next_pane_id: usize
```

All fields currently on `App` that belong to a single terminal session (`layout`, `panes`, `emulators`, `active`, `next_id`) move into `Tab`. `App` becomes a thin orchestrator holding workspaces and routing input to the active tab.

**Type aliases** (in a new `src/multiplexer/ids.rs` or inline in `app.rs`):
```rust
pub type WorkspaceId = usize;
pub type TabId = usize;
pub type PaneId = usize;
```

---

## 2. Keyboard Shortcuts

| Action | Binding | Config key |
|---|---|---|
| Next terminal (pane) | Ctrl+Tab | `pane_next` |
| Prev terminal (pane) | Ctrl+Shift+Tab | `pane_prev` |
| Next tab | Ctrl+PageDown | `tab_next` |
| Prev tab | Ctrl+PageUp | `tab_prev` |
| Next workspace | Ctrl+Shift+PageDown | `workspace_next` |
| Prev workspace | Ctrl+Shift+PageUp | `workspace_prev` |
| New terminal (split H) | Ctrl+Shift+H | `split_horizontal` |
| New terminal (split V) | Ctrl+Shift+V | `split_vertical` |
| New tab | Ctrl+Shift+T | `tab_new` |
| New workspace | Ctrl+Shift+W | `workspace_new` |
| Close terminal | Ctrl+Shift+Q | `close_pane` |
| Rename active tab | Ctrl+Shift+R | `rename_pane` |

Navigation wraps around at both ends.

**Breaking change from current config:** `close_pane` moves from `Ctrl+Shift+W` to `Ctrl+Shift+Q`. `Ctrl+Shift+W` is reassigned to `workspace_new`. Users with a custom `close_pane` binding in their config file are unaffected (the serde default only applies when the key is absent).

`Ctrl+Shift+R` renames the active tab. Workspace renaming is done via a context action in the sidebar (double-click on the workspace name) — no dedicated keybinding.

---

## 3. Sidebar Design

The sidebar gains a two-level tree:

```
WORKSPACES                        [+] [?]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
▌ ▾ infra                          [+]     ← active workspace: 3px accent (#89b4fa) + dim bg
    ▌ deploy                               ← active tab: 3px accent (#54b4ff) + dim bg
        main
        Claude Code
      server                               ← inactive tab: dim text, no accent
        feat/api
  ▸ perso                          [+]     ← inactive workspace: no accent, collapsed
  ▸ work                           [+]
```

**Visual rules:**
- Active workspace: 3px left accent bar `#89b4fa` (blue-ish), slightly lighter background, arrow in accent color
- Inactive workspace: no accent, dim text, collapsed by default
- Active tab: 3px left accent bar `#54b4ff` (same blue as current pane accent), slightly different background
- Inactive tab: no accent, dim text
- Tab indent: tabs are indented relative to the workspace row (`margin-left: ~14px` on the accent-bar column)
- Git branch shown in green below tab name (same as current)
- Terminal title shown in dim gray below branch (same as current)
- `[+]` button on each workspace row → `NewTab` for that workspace
- `[+]` in header → `NewWorkspace`
- `[?]` in header → help overlay (unchanged)

Renaming: clicking a workspace/tab name while holding the rename shortcut (or via Ctrl+Shift+R) activates an inline text input, same pattern as current pane rename.

---

## 4. Messages / Actions

New `Message` variants to add in `app.rs`:

```rust
// Workspace level
NewWorkspace,
CloseWorkspace(WorkspaceId),
StartRenameWorkspace(WorkspaceId),
RenameWorkspaceChange(String),
RenameWorkspaceCommit,
RenameWorkspaceCancel,
FocusWorkspaceNext,
FocusWorkspacePrev,

// Tab level
NewTab,
CloseTab(TabId),
SelectTab(TabId),
StartRenameTab(TabId),
RenameTabChange(String),
RenameTabCommit,
RenameTabCancel,
FocusTabNext,
FocusTabPrev,
```

Existing messages (`NewPane`, `ClosePane`, `StartRename`, `RenameChange`, `RenameCommit`, `RenameCancel`, `FocusNext`, `FocusPrev`, `SelectPane`) remain unchanged — they operate on panes within the active tab.

The active tab is `app.workspaces[active_workspace].tabs[active_tab]`. All pane-level messages delegate to the active tab's fields.

---

## 5. Startup / Migration

No persistent state exists today, so there is no migration. On startup, the app creates:
- 1 workspace named `"default"` (id=0)
- 1 tab named `"main"` (id=0) containing the initial pane

State persistence (saving/restoring workspaces+tabs across sessions) is a future feature and is explicitly out of scope for this spec.

---

## Out of Scope

- **Split-view rendering** (multiple panes visible simultaneously) — separate spec
- **State persistence** — future feature
- **Drag-and-drop** tab/workspace reordering
- **Per-workspace color themes**
