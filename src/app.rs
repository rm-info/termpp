use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Duration;
use iced::{Background, Color, Element, Subscription, Task};
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::Length;

use termpp::config::Config;
use termpp::ui::theme::Theme as AppTheme;
use termpp::multiplexer::layout::SplitDirection;
use termpp::multiplexer::notification::NotificationDetector;
use termpp::multiplexer::pane::{PaneState, PaneStatus, detect_git_branch};
use termpp::multiplexer::workspace::{Tab, Workspace};
use termpp::terminal::emulator::Emulator;
use termpp::ui::help_overlay::help_overlay;
use termpp::ui::pane_grid::{TerminalPane, TERM_PADDING};
use termpp::ui::sidebar::{Sidebar, TabEntry, WorkspaceEntry};

pub const WINDOW_W: f32   = 1200.0;
pub const WINDOW_H: f32   = 768.0;
const SIDEBAR_INIT_W: f32 = 200.0;
const SIDEBAR_MIN_W: f32  = 80.0;
const SIDEBAR_MAX_W: f32  = 500.0;
const DIVIDER_W: f32      = 4.0;
/// Monospace character advance ratio: width ≈ font_size × 0.6
const CHAR_W_RATIO: f32   = 0.6;
/// Line height ratio: height ≈ font_size × 1.4
const LINE_H_RATIO: f32   = 1.2;
/// Width of the per-pane focus indicator bar on the left edge (pixels).
const ACCENT_BAR_W: f32   = 3.0;
/// Split-divider separator thickness in pixels (mirrors Layout::SEP_PX).
const SEP_PX: f32         = 4.0;

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
    /// Workspace currently being renamed: (ws_id, current_input_value).
    renaming_workspace: Option<(usize, String)>,
    /// Font name, leaked once at startup for iced's `Family::Name` which needs `&'static str`.
    font_name:          &'static str,
    /// Incremented every fast Tick; drives cursor blink (period = 62 ticks ≈ 1 s).
    blink_tick:         u8,
    /// Split divider being dragged: (divider_id, is_vertical, Option<(start_mouse, start_ratio)>).
    dragging_split:     Option<(usize, bool, Option<(f32, f32)>)>,
    /// Last known cursor position (absolute window coords), updated via on_move.
    mouse_pos:    iced::Point,
    /// Some(pane_id) while user is drag-selecting; None otherwise.
    is_selecting: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Fast tick (~50 ms): drain PTY events, detect output, trigger redraw.
    Tick,
    /// Slow tick (2 s): git branch detection, notification idle check.
    StatusTick,
    GitBranchDetected(usize, Option<String>),
    KeyInput(Vec<u8>),
    SplitPane(SplitDirection),
    ClosePane,
    FocusNext,
    FocusPrev,
    PaneScrolled(usize, f32),   // (pane_id, y_delta): positive = scroll up into history
    MouseMoved(iced::Point),           // tracks cursor position for selection start
    SelectionStart(usize),             // pane_id; uses state.mouse_pos for start coords
    SelectionDrag(f32, f32),           // absolute cursor pos during drag
    SelectionEnd,                       // finalise selection (copy in Task 6)
    PasteFromClipboard(usize),         // pane_id
    SplitDividerDragStart(usize, bool),  // (divider_id, is_vertical)
    SplitDividerDragged(f32, f32),       // (mouse_x, mouse_y)
    SplitDividerDragEnd,
    ToggleHelp,
    Resized(f32, f32),
    SidebarDragStart,
    SidebarDragged(f32),
    SidebarDragEnd,
    RenameChanged(String),
    CommitRename,
    CancelRename,
    // Tab-level
    FocusTabNext,
    FocusTabPrev,
    SelectTab(usize),
    CloseTab(usize),
    NewTabIn(usize),          // arg = workspace_id to create tab in
    StartRenameTab(usize),    // arg = tab_id
    // Workspace-level
    FocusWorkspaceNext,
    FocusWorkspacePrev,
    NewWorkspace,
    ToggleWorkspace(usize),   // arg = workspace_id, toggles collapsed
    // Workspace rename
    StartRenameWorkspace(usize),
    RenameWorkspaceChanged(String),
    CommitRenameWorkspace,
    CancelRenameWorkspace,
}

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
        let cols = ((ww - self.sidebar_w - DIVIDER_W - ACCENT_BAR_W - TERM_PADDING * 2.0)
            / (self.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
        let rows = ((wh - TERM_PADDING * 2.0)
            / (self.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
        (cols, rows)
    }

    /// Returns the available pane area in pixels (width, height).
    fn pane_area_px(&self) -> (f32, f32) {
        (self.window_size.0 - self.sidebar_w - DIVIDER_W, self.window_size.1)
    }

    /// Converts pixel dimensions (per-pane) to (cols, rows) for an emulator.
    fn px_to_emu(w_px: f32, h_px: f32, font_size: f32) -> (u16, u16) {
        let cols = ((w_px - ACCENT_BAR_W - TERM_PADDING * 2.0) / (font_size * CHAR_W_RATIO)).floor() as u16;
        let rows = ((h_px - TERM_PADDING * 2.0) / (font_size * LINE_H_RATIO)).floor() as u16;
        (cols.max(1), rows.max(1))
    }
}

pub fn boot() -> (Termpp, Task<Message>) {
    let config = Config::load_or_default().unwrap_or_else(|e| {
        eprintln!("Config error: {e}. Using defaults.");
        Config::default()
    });

    let pane_id = 0usize;
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut pane = PaneState::new(pane_id, cwd.clone());
    pane.git_branch = detect_git_branch(&cwd);

    let timeout  = Duration::from_secs(config.notification_timeout);
    let detector = NotificationDetector::new(timeout);
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
        renaming_tab:       None,
        renaming_workspace: None,
        font_name,
        blink_tick:        0,
        dragging_split:    None,
        mouse_pos:         iced::Point::ORIGIN,
        is_selecting:      None,
    };

    (app, Task::none())
}

pub fn title(state: &Termpp) -> String {
    let ws_name = state.workspaces.iter()
        .find(|w| w.id == state.active_workspace)
        .map(|w| w.name.as_str())
        .unwrap_or("default");
    let tab = state.active_tab();
    let term_title = tab.panes.get(&tab.active_pane)
        .and_then(|p| p.terminal_title.as_deref())
        .unwrap_or("");
    if term_title.is_empty() {
        format!("terminal++ — {ws_name} — {}", tab.name)
    } else {
        format!("terminal++ — {ws_name} — {} — {term_title}", tab.name)
    }
}

pub fn update(state: &mut Termpp, message: Message) -> Task<Message> {
    match message {
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
                let tab_pos = state.workspaces[wi].tabs.iter().position(|t| t.panes.contains_key(&pane_id));
                if let Some(ti) = tab_pos {
                    let tab = &mut state.workspaces[wi].tabs[ti];
                    if let Some(new_layout) = tab.layout.remove(pane_id) {
                        tab.panes.remove(&pane_id);
                        tab.emulators.remove(&pane_id);
                        tab.last_output_counts.remove(&pane_id);
                        tab.layout = new_layout;
                        tab.active_pane = *tab.layout.pane_ids().first().unwrap_or(&0);
                    } else {
                        let tab_id = state.workspaces[wi].tabs[ti].id;
                        state.workspaces[wi].tabs.remove(ti);
                        if state.workspaces[wi].tabs.is_empty() {
                            state.workspaces.remove(wi);
                            if state.workspaces.is_empty() {
                                std::process::exit(0);
                            }
                            let new_wi = wi.min(state.workspaces.len() - 1);
                            state.active_workspace = state.workspaces[new_wi].id;
                        } else {
                            let new_ti = ti.min(state.workspaces[wi].tabs.len() - 1);
                            state.workspaces[wi].active_tab = state.workspaces[wi].tabs[new_ti].id;
                        }
                        let _ = tab_id;
                    }
                }
            }
        }
        Message::StatusTick => {
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
        Message::KeyInput(bytes) => {
            let (active_pane, emu_arc) = {
                let tab = state.active_tab();
                let pane_id = tab.active_pane;
                (pane_id, tab.emulators.get(&pane_id).cloned())
            };
            if let Some(emu_arc) = emu_arc {
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                let _ = emu.write_input(&bytes);
            }
            let _ = active_pane;
        }
        Message::Resized(w, h) => {
            state.window_size = (w, h);
            let font_size = state.config.font_size as f32;
            let (pw, ph)  = state.pane_area_px();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    let px_sizes = tab.layout.pane_pixel_sizes(pw, ph);
                    for (&pid, emu_arc) in &tab.emulators {
                        let (cols, rows) = px_sizes.get(&pid)
                            .map(|&(wpx, hpx)| Termpp::px_to_emu(wpx, hpx, font_size))
                            .unwrap_or((80, 24));
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        emu.resize(cols, rows);
                    }
                }
            }
        }
        Message::SidebarDragStart => {
            state.dragging_sidebar = true;
        }
        Message::SidebarDragged(x) => {
            state.sidebar_w = x.clamp(SIDEBAR_MIN_W, SIDEBAR_MAX_W);
        }
        Message::SidebarDragEnd => {
            state.dragging_sidebar = false;
            let font_size = state.config.font_size as f32;
            let (pw, ph)  = state.pane_area_px();
            for ws in &state.workspaces {
                for tab in &ws.tabs {
                    let px_sizes = tab.layout.pane_pixel_sizes(pw, ph);
                    for (&pid, emu_arc) in &tab.emulators {
                        let (cols, rows) = px_sizes.get(&pid)
                            .map(|&(wpx, hpx)| Termpp::px_to_emu(wpx, hpx, font_size))
                            .unwrap_or((80, 24));
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        emu.resize(cols, rows);
                    }
                }
            }
        }
        Message::SplitPane(dir) => {
            let shell      = state.config.shell.clone();
            let font_size  = state.config.font_size as f32;
            let (pw, ph)   = state.pane_area_px();
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let tab = &mut state.workspaces[wi].tabs[ti];
            let new_id = tab.next_pane_id;
            if let Some(new_layout) = tab.layout.split(tab.active_pane, dir, new_id) {
                let cwd = tab.panes.get(&tab.active_pane).map(|p| p.cwd.clone()).unwrap_or_default();
                let mut pane = PaneState::new(new_id, cwd.clone());
                pane.git_branch = detect_git_branch(&cwd);
                tab.panes.insert(new_id, pane);
                tab.next_pane_id += 1;
                tab.layout     = new_layout;
                tab.active_pane = new_id;
                // Resize all existing emulators to their new (smaller) area
                let px_sizes = tab.layout.pane_pixel_sizes(pw, ph);
                for (&pid, emu_arc) in &tab.emulators {
                    if let Some(&(wpx, hpx)) = px_sizes.get(&pid) {
                        let (cols, rows) = Termpp::px_to_emu(wpx, hpx, font_size);
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        emu.resize(cols, rows);
                    }
                }
                // Start the new emulator at its correct size
                // px_sizes already includes new_id (layout was updated above)
                let new_size = px_sizes.get(&new_id)
                    .map(|&(w, h)| Termpp::px_to_emu(w, h, font_size))
                    .unwrap_or((80, 24));
                if let Ok(emu) = Emulator::start(new_size.0, new_size.1, &shell, &cwd) {
                    tab.last_output_counts.insert(new_id, 0);
                    tab.emulators.insert(new_id, Arc::new(Mutex::new(emu)));
                }
            }
        }
        Message::ClosePane => {
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let pane_id = state.workspaces[wi].tabs[ti].active_pane;
            let tab = &mut state.workspaces[wi].tabs[ti];
            if let Some(new_layout) = tab.layout.remove(pane_id) {
                tab.panes.remove(&pane_id);
                tab.emulators.remove(&pane_id);
                tab.last_output_counts.remove(&pane_id);
                tab.layout = new_layout;
                tab.active_pane = *tab.layout.pane_ids().first().unwrap_or(&0);
            } else {
                let ti2 = ti;
                state.workspaces[wi].tabs.remove(ti2);
                if state.workspaces[wi].tabs.is_empty() {
                    state.workspaces.remove(wi);
                    if state.workspaces.is_empty() {
                        std::process::exit(0);
                    }
                    let new_wi = wi.min(state.workspaces.len() - 1);
                    state.active_workspace = state.workspaces[new_wi].id;
                } else {
                    let new_ti = ti.min(state.workspaces[wi].tabs.len() - 1);
                    state.workspaces[wi].active_tab = state.workspaces[wi].tabs[new_ti].id;
                }
            }
        }
        Message::FocusNext => {
            let tab = state.active_tab_mut();
            let ids = tab.layout.pane_ids();
            if let Some(pos) = ids.iter().position(|&id| id == tab.active_pane) {
                tab.active_pane = ids[(pos + 1) % ids.len()];
            }
        }
        Message::FocusPrev => {
            let tab = state.active_tab_mut();
            let ids = tab.layout.pane_ids();
            if let Some(pos) = ids.iter().position(|&id| id == tab.active_pane) {
                tab.active_pane = ids[(pos + ids.len() - 1) % ids.len()];
            }
        }
        Message::PaneScrolled(pane_id, delta) => {
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let tab = &state.workspaces[wi].tabs[ti];
            if let Some(emu_arc) = tab.emulators.get(&pane_id) {
                let emu  = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                let mut grid = emu.grid.lock().unwrap_or_else(|e| e.into_inner());
                let lines = (delta.abs() * 3.0).round() as usize;
                if delta > 0.0 {
                    grid.scroll_up_by(lines);
                } else {
                    grid.scroll_down_by(lines);
                }
            }
        }
        Message::MouseMoved(pos) => {
            state.mouse_pos = pos;
        }

        Message::SelectionStart(pane_id) => {
            // Focus the pane
            state.active_tab_mut().active_pane = pane_id;
            // Clear any existing selection in all panes of the active tab
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            for pane in state.workspaces[wi].tabs[ti].panes.values_mut() {
                pane.selection = None;
            }
            // Compute start cell from current mouse position
            let (pw, ph) = state.pane_area_px();
            let pane_area_x = state.sidebar_w + DIVIDER_W;
            let tab = &state.workspaces[wi].tabs[ti];
            if let Some((ox, oy)) = pane_origin(&tab.layout, pane_id, pane_area_x, 0.0, pw, ph) {
                let emu_arc = tab.emulators.get(&pane_id).cloned();
                if let Some(emu_arc) = emu_arc {
                    let emu  = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                    let grid = emu.grid.lock().unwrap_or_else(|e| e.into_inner());
                    let cell = pixel_to_cell(
                        state.mouse_pos.x, state.mouse_pos.y,
                        ox, oy,
                        state.config.font_size as f32,
                        grid.cols(), grid.rows(),
                    );
                    drop(grid); drop(emu);
                    let tab = &mut state.workspaces[wi].tabs[ti];
                    if let Some(pane) = tab.panes.get_mut(&pane_id) {
                        pane.selection = Some((cell, cell));
                    }
                }
            }
            state.is_selecting = Some(pane_id);
        }

        Message::SelectionDrag(x, y) => {
            if let Some(pane_id) = state.is_selecting {
                let (pw, ph) = state.pane_area_px();
                let pane_area_x = state.sidebar_w + DIVIDER_W;
                let wi = state.active_ws_idx();
                let ti = state.workspaces[wi].active_tab_idx();
                let tab = &state.workspaces[wi].tabs[ti];
                if let Some((ox, oy)) = pane_origin(&tab.layout, pane_id, pane_area_x, 0.0, pw, ph) {
                    let emu_arc = tab.emulators.get(&pane_id).cloned();
                    if let Some(emu_arc) = emu_arc {
                        let emu  = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        let grid = emu.grid.lock().unwrap_or_else(|e| e.into_inner());
                        let end_cell = pixel_to_cell(
                            x, y, ox, oy,
                            state.config.font_size as f32,
                            grid.cols(), grid.rows(),
                        );
                        drop(grid); drop(emu);
                        let tab = &mut state.workspaces[wi].tabs[ti];
                        if let Some(pane) = tab.panes.get_mut(&pane_id) {
                            if let Some(sel) = pane.selection.as_mut() {
                                sel.1 = end_cell;
                            }
                        }
                    }
                }
            }
        }

        Message::SelectionEnd => {
            state.is_selecting = None;
            // Copy selected text to clipboard
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let active_id = state.workspaces[wi].tabs[ti].active_pane;
            let tab = &state.workspaces[wi].tabs[ti];
            if let Some(pane) = tab.panes.get(&active_id) {
                if let Some(sel) = pane.selection {
                    if let Some(emu_arc) = tab.emulators.get(&active_id) {
                        let emu  = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        let grid = emu.grid.lock().unwrap_or_else(|e| e.into_inner());
                        let text = extract_selection_text(&grid, sel);
                        drop(grid); drop(emu);
                        if !text.is_empty() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(text);
                            }
                        }
                    }
                }
            }
        }

        Message::PasteFromClipboard(pane_id) => {
            // Also focus the pane
            state.active_tab_mut().active_pane = pane_id;
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    let wi = state.active_ws_idx();
                    let ti = state.workspaces[wi].active_tab_idx();
                    let tab = &state.workspaces[wi].tabs[ti];
                    if let Some(emu_arc) = tab.emulators.get(&pane_id) {
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        let _ = emu.write_input(text.as_bytes());
                    }
                }
            }
        }

        Message::SplitDividerDragStart(divider_id, is_vertical) => {
            state.dragging_split = Some((divider_id, is_vertical, None));
        }
        Message::SplitDividerDragged(x, y) => {
            let (divider_id, is_vertical, anchor) = match state.dragging_split {
                Some(s) => s,
                None    => return Task::none(),
            };
            let pos   = if is_vertical { x } else { y };
            let total = if is_vertical {
                state.window_size.0 - state.sidebar_w - DIVIDER_W
            } else {
                state.window_size.1
            };
            // On first move, record anchor (start mouse pos + start ratio)
            let (start_pos, start_ratio) = if let Some(a) = anchor {
                a
            } else {
                let wi = state.active_ws_idx();
                let ti = state.workspaces[wi].active_tab_idx();
                let ratio = state.workspaces[wi].tabs[ti].layout
                    .get_ratio(divider_id).unwrap_or(0.5);
                state.dragging_split = Some((divider_id, is_vertical, Some((pos, ratio))));
                (pos, ratio)
            };
            let new_ratio = (start_ratio + (pos - start_pos) / total).clamp(0.1, 0.9);
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            state.workspaces[wi].tabs[ti].layout.set_ratio(divider_id, new_ratio);
        }
        Message::SplitDividerDragEnd => {
            state.dragging_split = None;
            // Resize all emulators in the active tab to their new correct sizes
            let font_size = state.config.font_size as f32;
            let (pw, ph)  = state.pane_area_px();
            let wi = state.active_ws_idx();
            let ti = state.workspaces[wi].active_tab_idx();
            let tab = &mut state.workspaces[wi].tabs[ti];
            let px_sizes = tab.layout.pane_pixel_sizes(pw, ph);
            for (&pid, emu_arc) in &tab.emulators {
                let (cols, rows) = px_sizes.get(&pid)
                    .map(|&(wpx, hpx)| Termpp::px_to_emu(wpx, hpx, font_size))
                    .unwrap_or((80, 24));
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                emu.resize(cols, rows);
            }
        }
        Message::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                state.renaming_tab = None;
                state.renaming_workspace = None;
            }
        }
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
        Message::StartRenameWorkspace(ws_id) => {
            let name = state.workspaces.iter().find(|w| w.id == ws_id)
                .map(|w| w.name.clone()).unwrap_or_default();
            state.renaming_workspace = Some((ws_id, name));
            return iced::widget::operation::focus(
                iced::widget::Id::new(termpp::ui::sidebar::RENAME_WS_INPUT_ID)
            );
        }
        Message::RenameWorkspaceChanged(s) => {
            if let Some((_, ref mut val)) = state.renaming_workspace {
                *val = s;
            }
        }
        Message::CommitRenameWorkspace => {
            if let Some((ws_id, name)) = state.renaming_workspace.take() {
                if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id == ws_id) {
                    ws.name = if name.trim().is_empty() { "workspace".into() } else { name.trim().to_string() };
                }
            }
        }
        Message::CancelRenameWorkspace => {
            state.renaming_workspace = None;
        }
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
                }
            }
        }
        Message::NewTabIn(ws_id) => {
            let (emu_cols, emu_rows) = state.emu_size();
            let shell = state.config.shell.clone();
            let tab_id = state.next_tab_id;
            state.next_tab_id += 1;
            let cwd = {
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
    }
    Task::none()
}

fn matches_binding(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    binding: &str,
) -> bool {
    use iced::keyboard::Key;
    let mut needs_ctrl  = false;
    let mut needs_shift = false;
    let mut needs_alt   = false;
    let mut key_str     = String::new();

    for part in binding.split('+') {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => needs_ctrl  = true,
            "shift"            => needs_shift = true,
            "alt"              => needs_alt   = true,
            k                  => key_str = k.to_string(),
        }
    }

    if modifiers.control() != needs_ctrl  { return false; }
    if modifiers.shift()   != needs_shift { return false; }
    if modifiers.alt()     != needs_alt   { return false; }
    if key_str.is_empty() { return false; }

    match key {
        Key::Character(c) => c.as_str().eq_ignore_ascii_case(&key_str),
        Key::Named(n)     => format!("{n:?}").to_ascii_lowercase() == key_str,
        _                 => false,
    }
}

fn key_to_bytes(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    text: Option<&str>,
) -> Vec<u8> {
    use iced::keyboard::Key;
    use iced::keyboard::key::Named;

    // Ctrl+letter → control byte 0x01–0x1a
    if modifiers.control() && !modifiers.alt() {
        if let Key::Character(c) = key {
            let ch = c.as_str().chars().next().unwrap_or('\0').to_ascii_lowercase();
            if ('a'..='z').contains(&ch) {
                return vec![ch as u8 - b'a' + 1];
            }
        }
    }

    // Printable text from the OS keyboard layout (handles accents, shift, etc.)
    // Skip if the text is purely control characters — those must go through
    // the Named key mapping below so we send the correct terminal sequence.
    if let Some(t) = text {
        if !t.is_empty() && t.chars().any(|c| c >= ' ' && c != '\x7f') {
            return t.as_bytes().to_vec();
        }
    }

    // Special keys → VT/xterm escape sequences
    match key {
        Key::Named(Named::Enter)     => b"\r".to_vec(),
        Key::Named(Named::Backspace) => b"\x7f".to_vec(),
        Key::Named(Named::Tab)       => if modifiers.shift() { b"\x1b[Z".to_vec() } else { b"\t".to_vec() },
        Key::Named(Named::Escape)    => b"\x1b".to_vec(),
        Key::Named(Named::ArrowUp)   => b"\x1b[A".to_vec(),
        Key::Named(Named::ArrowDown) => b"\x1b[B".to_vec(),
        Key::Named(Named::ArrowRight)=> b"\x1b[C".to_vec(),
        Key::Named(Named::ArrowLeft) => b"\x1b[D".to_vec(),
        Key::Named(Named::Home)      => b"\x1b[H".to_vec(),
        Key::Named(Named::End)       => b"\x1b[F".to_vec(),
        Key::Named(Named::Delete)    => b"\x1b[3~".to_vec(),
        Key::Named(Named::Insert)    => b"\x1b[2~".to_vec(),
        Key::Named(Named::PageUp)    => b"\x1b[5~".to_vec(),
        Key::Named(Named::PageDown)  => b"\x1b[6~".to_vec(),
        Key::Named(Named::F1)        => b"\x1bOP".to_vec(),
        Key::Named(Named::F2)        => b"\x1bOQ".to_vec(),
        Key::Named(Named::F3)        => b"\x1bOR".to_vec(),
        Key::Named(Named::F4)        => b"\x1bOS".to_vec(),
        Key::Named(Named::F5)        => b"\x1b[15~".to_vec(),
        Key::Named(Named::F6)        => b"\x1b[17~".to_vec(),
        Key::Named(Named::F7)        => b"\x1b[18~".to_vec(),
        Key::Named(Named::F8)        => b"\x1b[19~".to_vec(),
        Key::Named(Named::F9)        => b"\x1b[20~".to_vec(),
        Key::Named(Named::F10)       => b"\x1b[21~".to_vec(),
        Key::Named(Named::F11)       => b"\x1b[23~".to_vec(),
        Key::Named(Named::F12)       => b"\x1b[24~".to_vec(),
        _ => vec![],
    }
}

pub fn subscription(state: &Termpp) -> Subscription<Message> {
    let tick        = iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick);
    let status_tick = iced::time::every(Duration::from_secs(2)).map(|_| Message::StatusTick);
    // iced::event::listen_with requires a fn pointer (no captures).
    // Use Subscription::with() to attach keybindings as cloned data into the stream,
    // then filter_map with a plain `fn` pointer (zero-sized, no captures) that
    // receives (bindings, event) and produces the correct Message.
    let bindings        = state.config.keybindings.clone();
    let is_renaming_tab = state.renaming_tab.is_some();
    let is_renaming_ws  = state.renaming_workspace.is_some();
    let show_help       = state.show_help;
    let active_tab_id       = state.active_tab().id;
    let active_workspace_id = state.active_workspace;

    let keyboard = iced::event::listen()
        .with((bindings, is_renaming_tab, is_renaming_ws, show_help, active_tab_id, active_workspace_id))
        .filter_map(|((bindings, is_renaming_tab, is_renaming_ws, show_help, active_tab_id, active_workspace_id), event): ((termpp::config::Keybindings, bool, bool, bool, usize, usize), iced::Event)| -> Option<Message> {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, text, .. }) = event {
                use iced::keyboard::key::Named;

                // 1. F1 always opens/closes help — checked before any other guard
                if matches!(key, iced::keyboard::Key::Named(Named::F1)) {
                    return Some(Message::ToggleHelp);
                }

                // 2. During rename: only Escape passes through
                if is_renaming_tab || is_renaming_ws {
                    if matches!(key, iced::keyboard::Key::Named(Named::Escape)) {
                        return if is_renaming_ws {
                            Some(Message::CancelRenameWorkspace)
                        } else {
                            Some(Message::CancelRename)
                        };
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
                    return Some(Message::SplitPane(SplitDirection::Vertical));
                }
                if matches_binding(&key, modifiers, &bindings.split_vertical) {
                    return Some(Message::SplitPane(SplitDirection::Horizontal));
                }
                if matches_binding(&key, modifiers, &bindings.pane_next) {
                    return Some(Message::FocusNext);
                }
                if matches_binding(&key, modifiers, &bindings.pane_prev) {
                    return Some(Message::FocusPrev);
                }
                if matches_binding(&key, modifiers, &bindings.close_pane) {
                    return Some(Message::ClosePane);
                }
                if matches_binding(&key, modifiers, &bindings.rename_pane) {
                    return Some(Message::StartRenameTab(active_tab_id));
                }
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
                let bytes = key_to_bytes(&key, modifiers, text.as_deref());
                if bytes.is_empty() { None } else { Some(Message::KeyInput(bytes)) }
            } else {
                None
            }
        });

    let resize = iced::event::listen_with(|event, _status, _id| {
        if let iced::Event::Window(iced::window::Event::Resized(size)) = event {
            Some(Message::Resized(size.width, size.height))
        } else {
            None
        }
    });
    let is_dragging_split = state.dragging_split.is_some();
    if state.dragging_sidebar {
        let drag = iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::SidebarDragged(position.x))
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                Some(Message::SidebarDragEnd)
            }
            _ => None,
        });
        Subscription::batch([tick, status_tick, keyboard, resize, drag])
    } else if is_dragging_split {
        let split_drag = iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::SplitDividerDragged(position.x, position.y))
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                Some(Message::SplitDividerDragEnd)
            }
            _ => None,
        });
        Subscription::batch([tick, status_tick, keyboard, resize, split_drag])
    } else if state.is_selecting.is_some() {
        let sel_drag = iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::SelectionDrag(position.x, position.y))
            }
            iced::Event::Mouse(
                iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
            ) => Some(Message::SelectionEnd),
            _ => None,
        });
        Subscription::batch([tick, status_tick, keyboard, resize, sel_drag])
    } else {
        Subscription::batch([tick, status_tick, keyboard, resize])
    }
}

/// Returns the absolute (x, y) top-left corner of `target` pane within the layout.
fn pane_origin(
    layout: &termpp::multiplexer::layout::Layout,
    target: usize,
    x: f32, y: f32, w: f32, h: f32,
) -> Option<(f32, f32)> {
    use termpp::multiplexer::layout::{Layout, SplitDirection};
    match layout {
        Layout::Leaf(id) if *id == target => Some((x, y)),
        Layout::Leaf(_) => None,
        Layout::Split { direction, left, right, ratio, .. } => {
            let content = match direction {
                SplitDirection::Vertical   => w - SEP_PX,
                SplitDirection::Horizontal => h - SEP_PX,
            };
            match direction {
                SplitDirection::Vertical => {
                    let lw = (content * ratio).max(1.0);
                    let rw = (content * (1.0 - ratio)).max(1.0);
                    pane_origin(left, target, x, y, lw, h)
                        .or_else(|| pane_origin(right, target, x + lw + SEP_PX, y, rw, h))
                }
                SplitDirection::Horizontal => {
                    let lh = (content * ratio).max(1.0);
                    let rh = (content * (1.0 - ratio)).max(1.0);
                    pane_origin(left, target, x, y, w, lh)
                        .or_else(|| pane_origin(right, target, x, y + lh + SEP_PX, w, rh))
                }
            }
        }
    }
}

/// Converts absolute window coordinates to (col, row) in the terminal grid.
fn pixel_to_cell(
    abs_x: f32, abs_y: f32,
    origin_x: f32, origin_y: f32,
    font_size: f32,
    cols: usize, rows: usize,
) -> (usize, usize) {
    let rel_x = (abs_x - origin_x - ACCENT_BAR_W - TERM_PADDING).max(0.0);
    let rel_y = (abs_y - origin_y - TERM_PADDING).max(0.0);
    let col = (rel_x / (font_size * CHAR_W_RATIO)).floor() as usize;
    let row = (rel_y / (font_size * LINE_H_RATIO)).floor() as usize;
    (col.min(cols.saturating_sub(1)), row.min(rows.saturating_sub(1)))
}

/// Normalises a selection so start <= end in reading order (row-major).
fn normalize_selection(
    sel: ((usize, usize), (usize, usize)),
) -> ((usize, usize), (usize, usize)) {
    let ((sc, sr), (ec, er)) = sel;
    if sr < er || (sr == er && sc <= ec) { sel } else { ((ec, er), (sc, sr)) }
}

/// Extracts the selected text from the grid (using visible_row for scrollback awareness).
fn extract_selection_text(
    grid: &termpp::terminal::grid::GridPerformer,
    sel: ((usize, usize), (usize, usize)),
) -> String {
    let ((sc, sr), (ec, er)) = normalize_selection(sel);
    let mut lines = Vec::new();
    for row in sr..=er {
        let start_col = if row == sr { sc } else { 0 };
        let end_col   = if row == er { ec } else { grid.cols().saturating_sub(1) };
        let row_cells = grid.visible_row(row);
        let text: String = row_cells
            .iter()
            .enumerate()
            .filter(|&(c, _)| c >= start_col && c <= end_col)
            .filter(|(_, cell)| cell.ch != '\0')
            .map(|(_, cell)| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string();
        lines.push(text);
    }
    lines.join("\n")
}

fn render_layout(
    layout:      &termpp::multiplexer::layout::Layout,
    panes:       &std::collections::HashMap<usize, PaneState>,
    emulators:   &std::collections::HashMap<usize, Arc<Mutex<Emulator>>>,
    active_pane: usize,
    w_px:        f32,
    h_px:        f32,
    font_size:   f32,
    font_name:   &'static str,
    cursor_on:   bool,
    close_key:   &str,
) -> Element<'static, Message> {
    use termpp::multiplexer::layout::{Layout, SplitDirection};

    match layout {
        Layout::Leaf(pane_id) => {
            let pane_id = *pane_id;
            let is_active = pane_id == active_pane;

            let content: Element<'static, Message> =
                if let (Some(pane), Some(emu_arc)) = (panes.get(&pane_id), emulators.get(&pane_id)) {
                    if pane.status == PaneStatus::Dead {
                        let close_key = close_key.to_string();
                        container(
                            column![
                                text("Process exited")
                                    .size(18).color(Color { r: 0.75, g: 0.75, b: 0.75, a: 1.0 }),
                                text(format!("{close_key} to close"))
                                    .size(13).color(Color { r: 0.40, g: 0.40, b: 0.50, a: 1.0 }),
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
                        let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                        TerminalPane::new(
                            Arc::clone(&emu.grid),
                            panes.get(&pane_id).and_then(|p| p.selection),
                            font_size,
                            font_name,
                            cursor_on,
                        ).view()
                    }
                } else {
                    iced::widget::text("No pane").into()
                };

            // 3-px left-edge accent bar: bright when active, invisible when not
            let bar_color = if is_active { AppTheme::ACCENT } else { AppTheme::PANE_BG };
            let accent_bar: Element<'static, Message> = container(Space::new())
                .width(ACCENT_BAR_W)
                .height(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(bar_color)),
                    ..Default::default()
                })
                .into();

            mouse_area(
                container(
                    iced::widget::row![accent_bar, content].width(Length::Fill).height(Length::Fill)
                )
                .width(w_px)
                .height(h_px)
            )
            .on_move(move |pos| Message::MouseMoved(pos))
            .on_press(Message::SelectionStart(pane_id))
            .on_release(Message::SelectionEnd)
            .on_right_press(Message::PasteFromClipboard(pane_id))
            .on_scroll(move |delta| {
                let y = match delta {
                    iced::mouse::ScrollDelta::Lines  { y, .. } => y,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => y / 20.0,
                };
                Message::PaneScrolled(pane_id, y)
            })
            .into()
        }
        Layout::Split { split_id, direction, left, right, ratio } => {
            let divider_id  = *split_id;
            let sep_color   = Color { r: 0.18, g: 0.18, b: 0.26, a: 1.0 };
            match direction {
                SplitDirection::Vertical => {
                    let content_w = w_px - SEP_PX;
                    let lw = (content_w * ratio).max(1.0);
                    let rw = (content_w * (1.0 - ratio)).max(1.0);
                    let left_el  = render_layout(left,  panes, emulators, active_pane, lw, h_px, font_size, font_name, cursor_on, close_key);
                    let right_el = render_layout(right, panes, emulators, active_pane, rw, h_px, font_size, font_name, cursor_on, close_key);
                    let sep: Element<'static, Message> = mouse_area(
                        container(Space::new())
                            .width(SEP_PX)
                            .height(Length::Fill)
                            .style(move |_| iced::widget::container::Style {
                                background: Some(Background::Color(sep_color)),
                                ..Default::default()
                            })
                    )
                    .on_press(Message::SplitDividerDragStart(divider_id, true))
                    .on_release(Message::SplitDividerDragEnd)
                    .interaction(iced::mouse::Interaction::ResizingHorizontally)
                    .into();
                    row![left_el, sep, right_el]
                        .width(w_px)
                        .height(h_px)
                        .into()
                }
                SplitDirection::Horizontal => {
                    let content_h = h_px - SEP_PX;
                    let th = (content_h * ratio).max(1.0);
                    let bh = (content_h * (1.0 - ratio)).max(1.0);
                    let top_el = render_layout(left,  panes, emulators, active_pane, w_px, th, font_size, font_name, cursor_on, close_key);
                    let bot_el = render_layout(right, panes, emulators, active_pane, w_px, bh, font_size, font_name, cursor_on, close_key);
                    let sep: Element<'static, Message> = mouse_area(
                        container(Space::new())
                            .width(w_px)
                            .height(SEP_PX)
                            .style(move |_| iced::widget::container::Style {
                                background: Some(Background::Color(sep_color)),
                                ..Default::default()
                            })
                    )
                    .on_press(Message::SplitDividerDragStart(divider_id, false))
                    .on_release(Message::SplitDividerDragEnd)
                    .interaction(iced::mouse::Interaction::ResizingVertically)
                    .into();
                    column![top_el, sep, bot_el]
                        .width(w_px)
                        .height(h_px)
                        .into()
                }
            }
        }
    }
}

pub fn view(state: &Termpp) -> Element<'_, Message> {
    use termpp::multiplexer::pane::PaneStatus;

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
            state.renaming_workspace.clone(),
            Message::SelectTab,
            Message::CloseTab,
            Message::NewTabIn,
            Message::ToggleWorkspace,
            Message::NewWorkspace,
            Message::StartRenameTab,
            Message::RenameChanged,
            Message::CommitRename,
            Message::CancelRename,
            Message::StartRenameWorkspace,
            Message::RenameWorkspaceChanged,
            Message::CommitRenameWorkspace,
            Message::CancelRenameWorkspace,
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

    let tab = state.active_tab();
    let cursor_on = (state.blink_tick % 62) < 31;
    let (pane_w, pane_h) = state.pane_area_px();
    let pane_view: Element<'static, Message> = render_layout(
        &tab.layout,
        &tab.panes,
        &tab.emulators,
        tab.active_pane,
        pane_w,
        pane_h,
        state.config.font_size as f32,
        state.font_name,
        cursor_on,
        &state.config.keybindings.close_pane,
    );

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

    #[test]
    fn matches_ctrl_pagedown_for_tab_next() {
        use iced::keyboard::key::Named;
        assert!(matches_binding(&Key::Named(Named::PageDown), ctrl(), "ctrl+pagedown"));
    }

    #[test]
    fn matches_ctrl_shift_w_for_workspace_new() {
        assert!(matches_binding(&Key::Character("w".into()), ctrl_shift(), "ctrl+shift+w"));
    }

    #[test]
    fn matches_ctrl_shift_t_for_tab_new() {
        assert!(matches_binding(&Key::Character("t".into()), ctrl_shift(), "ctrl+shift+t"));
    }

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
}
