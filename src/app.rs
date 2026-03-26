use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Duration;
use iced::{Background, Color, Element, Subscription, Task};
use iced::widget::{column, container, mouse_area, row, text};
use iced::Length;

use termpp::config::Config;
use termpp::ui::theme::Theme as AppTheme;
use termpp::multiplexer::layout::{Layout, SplitDirection};
use termpp::multiplexer::notification::NotificationDetector;
use termpp::multiplexer::pane::{PaneState, PaneStatus, detect_git_branch};
use termpp::terminal::emulator::Emulator;
use termpp::ui::help_overlay::help_overlay;
use termpp::ui::pane_grid::{TerminalPane, TERM_PADDING};
use termpp::ui::sidebar::{Sidebar, WorkspaceEntry};

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

pub struct Termpp {
    config:             Config,
    layout:             Layout,
    panes:              HashMap<usize, PaneState>,
    emulators:          HashMap<usize, Arc<Mutex<Emulator>>>,
    active:             usize,
    next_id:            usize,
    detector:           NotificationDetector,
    /// Last observed output_count per pane, used to detect new PTY output on Tick.
    last_output_counts: HashMap<usize, u64>,
    show_help:          bool,
    window_size:        (f32, f32),
    sidebar_w:          f32,
    dragging_sidebar:   bool,
    /// Pane currently being renamed: (pane_id, current_input_value).
    renaming_pane:      Option<(usize, String)>,
    /// Font name, leaked once at startup for iced's `Family::Name` which needs `&'static str`.
    font_name:          &'static str,
    /// Incremented every fast Tick; drives cursor blink (period = 62 ticks ≈ 1 s).
    blink_tick:         u8,
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
    ToggleHelp,
    Resized(f32, f32),
    SidebarDragStart,
    SidebarDragged(f32),
    SidebarDragEnd,
    FocusPaneById(usize),
    ClosePaneById(usize),
    NewPane,
    StartRename(usize),
    RenameChanged(String),
    CommitRename,
    CancelRename,
}

pub fn boot() -> (Termpp, Task<Message>) {
    let config = Config::load_or_default().unwrap_or_else(|e| {
        eprintln!("Config error: {e}. Using defaults.");
        Config::default()
    });

    let id = 0;
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut pane = PaneState::new(id, cwd.clone());
    pane.git_branch = detect_git_branch(&cwd);

    let timeout = Duration::from_secs(config.notification_timeout);
    let detector = NotificationDetector::new(timeout);
    let emu_cols = ((WINDOW_W - SIDEBAR_INIT_W - DIVIDER_W - TERM_PADDING * 2.0) / (config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
    let emu_rows = ((WINDOW_H - TERM_PADDING * 2.0) / (config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
    let font_name: &'static str = Box::leak(config.font_name.clone().into_boxed_str());

    let mut app = Termpp {
        layout:             Layout::new(id),
        panes:              HashMap::from([(id, pane)]),
        emulators:          HashMap::new(),
        active:             id,
        next_id:            1,
        detector,
        config,
        last_output_counts: HashMap::new(),
        show_help:          false,
        window_size:        (WINDOW_W, WINDOW_H),
        sidebar_w:          SIDEBAR_INIT_W,
        dragging_sidebar:   false,
        renaming_pane:      None,
        font_name,
        blink_tick:         0,
    };

    // Emulator::start() is sync — uses tokio::spawn internally
    match Emulator::start(emu_cols, emu_rows, &app.config.shell, &cwd) {
        Ok(emu) => {
            app.last_output_counts.insert(id, 0);
            app.emulators.insert(id, Arc::new(Mutex::new(emu)));
        }
        Err(e) => { eprintln!("Failed to start emulator: {e}"); }
    }

    (app, Task::none())
}

pub fn title(_state: &Termpp) -> String {
    "terminal++".to_string()
}

pub fn update(state: &mut Termpp, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            state.blink_tick = state.blink_tick.wrapping_add(1);
            // Drain PTY events and detect new output for every emulator.
            // try_lock() prevents a slow reader thread from stalling the UI.
            let mut auto_close_ids: Vec<usize> = Vec::new();
            for (&pane_id, emu_arc) in &state.emulators {
                if let Ok(mut emu) = emu_arc.try_lock() {
                    let current_count = emu.output_count.load(Ordering::Relaxed);
                    let last_count = state.last_output_counts.get(&pane_id).copied().unwrap_or(0);
                    if current_count != last_count {
                        state.last_output_counts.insert(pane_id, current_count);
                        if let Some(pane) = state.panes.get_mut(&pane_id) {
                            pane.on_output();
                        }
                    }
                    while let Ok(event) = emu.event_rx.try_recv() {
                        if let Some(pane) = state.panes.get_mut(&pane_id) {
                            if let termpp::terminal::grid::TermEvent::CwdChange(path) = event {
                                pane.cwd = std::path::PathBuf::from(path);
                            } else {
                                state.detector.process_event(event, pane);
                            }
                        }
                    }
                    // Fallback: poll child exit directly — ConPTY on Windows doesn't
                    // always send EOF on the reader when the process exits.
                    if emu.is_exited() {
                        let mut just_exited = false;
                        if let Some(pane) = state.panes.get_mut(&pane_id) {
                            if pane.status != PaneStatus::Dead {
                                pane.on_exit();
                                just_exited = true;
                            }
                        }
                        if just_exited && state.config.auto_close_on_exit {
                            auto_close_ids.push(pane_id);
                        }
                    }
                }
            }
            // Process auto-close after the emulators loop to avoid borrow conflicts.
            for pane_id in auto_close_ids {
                if let Some(new_layout) = state.layout.remove(pane_id) {
                    state.panes.remove(&pane_id);
                    state.emulators.remove(&pane_id);
                    state.last_output_counts.remove(&pane_id);
                    state.layout = new_layout;
                    state.active = *state.layout.pane_ids().first().unwrap_or(&0);
                } else {
                    std::process::exit(0);
                }
            }
        }
        Message::StatusTick => {
            for pane in state.panes.values_mut() {
                state.detector.check_idle(pane);
            }
            // Spawn non-blocking git detection for each live pane
            let tasks: Vec<Task<Message>> = state.panes.iter()
                .filter(|(_, pane)| pane.status != PaneStatus::Dead)
                .map(|(&id, pane)| {
                    let cwd = pane.cwd.clone();
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || detect_git_branch(&cwd))
                                .await
                                .unwrap_or(None)
                        },
                        move |branch| Message::GitBranchDetected(id, branch),
                    )
                })
                .collect();
            return Task::batch(tasks);
        }
        Message::GitBranchDetected(pane_id, branch) => {
            if let Some(pane) = state.panes.get_mut(&pane_id) {
                pane.git_branch = branch;
            }
        }
        Message::KeyInput(bytes) => {
            if let Some(emu_arc) = state.emulators.get(&state.active) {
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                let _ = emu.write_input(&bytes);
            }
        }
        Message::Resized(w, h) => {
            state.window_size = (w, h);
            let new_cols = ((w - state.sidebar_w - DIVIDER_W - TERM_PADDING * 2.0) / (state.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
            let new_rows = ((h - TERM_PADDING * 2.0) / (state.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
            for emu_arc in state.emulators.values() {
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                emu.resize(new_cols, new_rows);
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
            let (ww, wh) = state.window_size;
            let new_cols = ((ww - state.sidebar_w - DIVIDER_W - TERM_PADDING * 2.0) / (state.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
            let new_rows = ((wh - TERM_PADDING * 2.0) / (state.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
            for emu_arc in state.emulators.values() {
                let emu = emu_arc.lock().unwrap_or_else(|e| e.into_inner());
                emu.resize(new_cols, new_rows);
            }
        }
        Message::SplitPane(dir) => {
            let new_id = state.next_id;
            if let Some(new_layout) = state.layout.split(state.active, dir, new_id) {
                state.layout = new_layout;
                let cwd = state.panes.get(&state.active)
                    .map(|p| p.cwd.clone())
                    .unwrap_or_default();
                let mut pane = PaneState::new(new_id, cwd.clone());
                pane.git_branch = detect_git_branch(&cwd);
                state.panes.insert(new_id, pane);
                let (ww, wh) = state.window_size;
                let emu_cols = ((ww - state.sidebar_w - DIVIDER_W - TERM_PADDING * 2.0) / (state.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
                let emu_rows = ((wh - TERM_PADDING * 2.0) / (state.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
                match Emulator::start(emu_cols, emu_rows, &state.config.shell, &cwd) {
                    Ok(emu) => {
                        state.last_output_counts.insert(new_id, 0);
                        state.emulators.insert(new_id, Arc::new(Mutex::new(emu)));
                    }
                    Err(e) => { eprintln!("Failed to start emulator for pane {new_id}: {e}"); }
                }
                state.next_id += 1;
                state.active = new_id;
            }
        }
        Message::ClosePane => {
            if let Some(new_layout) = state.layout.remove(state.active) {
                state.panes.remove(&state.active);
                state.emulators.remove(&state.active);
                state.last_output_counts.remove(&state.active);
                state.layout = new_layout;
                state.active = *state.layout.pane_ids().first().unwrap_or(&0);
            } else {
                // Last pane — close the app
                std::process::exit(0);
            }
        }
        Message::FocusNext => {
            let ids = state.layout.pane_ids();
            if let Some(pos) = ids.iter().position(|&id| id == state.active) {
                state.active = ids[(pos + 1) % ids.len()];
            }
        }
        Message::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                // Dismiss any active rename when opening the overlay
                state.renaming_pane = None;
            }
        }
        Message::FocusPaneById(id) => {
            if state.panes.contains_key(&id) {
                state.active = id;
            }
        }
        Message::ClosePaneById(target) => {
            if let Some(new_layout) = state.layout.remove(target) {
                state.panes.remove(&target);
                state.emulators.remove(&target);
                state.last_output_counts.remove(&target);
                state.layout = new_layout;
                if state.active == target {
                    state.active = *state.layout.pane_ids().first().unwrap_or(&0);
                }
            } else {
                std::process::exit(0);
            }
        }
        Message::NewPane => {
            // Open a new pane as a vertical split of the active pane
            let new_id = state.next_id;
            if let Some(new_layout) = state.layout.split(state.active, SplitDirection::Vertical, new_id) {
                state.layout = new_layout;
                let cwd = state.panes.get(&state.active).map(|p| p.cwd.clone()).unwrap_or_default();
                state.panes.insert(new_id, PaneState::new(new_id, cwd.clone()));
                let (ww, wh) = state.window_size;
                let emu_cols = ((ww - state.sidebar_w - DIVIDER_W - TERM_PADDING * 2.0) / (state.config.font_size as f32 * CHAR_W_RATIO)).floor() as u16;
                let emu_rows = ((wh - TERM_PADDING * 2.0) / (state.config.font_size as f32 * LINE_H_RATIO)).floor() as u16;
                if let Ok(emu) = Emulator::start(emu_cols, emu_rows, &state.config.shell, &cwd) {
                    state.last_output_counts.insert(new_id, 0);
                    state.emulators.insert(new_id, Arc::new(Mutex::new(emu)));
                }
                state.next_id += 1;
                state.active = new_id;
            }
        }
        Message::StartRename(id) => {
            let current = state.panes.get(&id)
                .and_then(|p| p.pane_name.clone())
                .unwrap_or_else(|| {
                    state.panes.get(&id)
                        .and_then(|p| p.cwd.file_name().and_then(|n| n.to_str()).map(str::to_string))
                        .unwrap_or_default()
                });
            state.renaming_pane = Some((id, current));
            return iced::widget::operation::focus(
                iced::widget::Id::new(termpp::ui::sidebar::RENAME_INPUT_ID)
            );
        }
        Message::RenameChanged(s) => {
            if let Some((_, ref mut val)) = state.renaming_pane {
                *val = s;
            }
        }
        Message::CommitRename => {
            if let Some((id, name)) = state.renaming_pane.take() {
                if let Some(pane) = state.panes.get_mut(&id) {
                    pane.pane_name = if name.trim().is_empty() { None } else { Some(name.trim().to_string()) };
                }
            }
        }
        Message::CancelRename => {
            state.renaming_pane = None;
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
    let bindings    = state.config.keybindings.clone();
    let is_renaming = state.renaming_pane.is_some();
    let show_help   = state.show_help;

    let keyboard = iced::event::listen()
        .with((bindings, is_renaming, show_help))
        .filter_map(|((bindings, is_renaming, show_help), event): ((termpp::config::Keybindings, bool, bool), iced::Event)| -> Option<Message> {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, text, .. }) = event {
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
    } else {
        Subscription::batch([tick, status_tick, keyboard, resize])
    }
}

pub fn view(state: &Termpp) -> Element<'_, Message> {
    let workspace_entries: Vec<WorkspaceEntry> = state.layout.pane_ids()
        .iter()
        .filter_map(|id| state.panes.get(id))
        .map(WorkspaceEntry::from_pane)
        .collect();

    // Sidebar::new owns its data; returned Element<'static, Message> does not
    // borrow workspace_entries.
    let sidebar: Element<'static, Message> =
        container(Sidebar::<Message>::new(
            &workspace_entries,
            state.active,
            state.renaming_pane.clone(),
            Message::FocusPaneById,
            Message::ClosePaneById,
            Message::NewPane,
            Message::StartRename,
            Message::RenameChanged,
            Message::CommitRename,
            Message::CancelRename,
            Message::ToggleHelp,
        ).view())
            .width(state.sidebar_w)
            .height(Length::Fill)
            .into();

    // Divider: transparent outer area (easy to grab) + 1px visible line in center.
    let line_color = if state.dragging_sidebar {
        Color { r: 0.55, g: 0.56, b: 0.98, a: 1.0 } // accent when dragging
    } else {
        Color { r: 0.18, g: 0.18, b: 0.26, a: 1.0 } // subtle at rest
    };
    let divider: Element<'static, Message> = mouse_area(
        container(
            container(iced::widget::Space::new())
                .width(1) // instead of 1
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
            // Transparent padding area to make dragging easier.
            // Cursor change handled by the parent mouse_area's Interaction.
            background: Some(iced::Background::Color(Color { r: 0.05, g: 0.05, b: 0.05, a: 1.0 })),
            ..Default::default()
        })
    )
    .on_press(Message::SidebarDragStart)
    .on_release(Message::SidebarDragEnd)
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .into();

    // TerminalPane::view() returns Element<'static, Message> (Arc clone, no borrows).
    let pane_view: Element<'static, Message> =
        if let (Some(pane), Some(emu_arc)) = (
            state.panes.get(&state.active),
            state.emulators.get(&state.active),
        ) {
            let emu: std::sync::MutexGuard<'_, Emulator> =
                emu_arc.lock().unwrap_or_else(|e| e.into_inner());
            let is_waiting = pane.status == PaneStatus::Waiting;
            if pane.status == PaneStatus::Dead {
                let close_key = state.config.keybindings.close_pane.clone();
                container(
                    column![
                        text("Process exited")
                            .size(18)
                            .color(Color { r: 0.75, g: 0.75, b: 0.75, a: 1.0 }),
                        text(format!("{close_key} to close"))
                            .size(13)
                            .color(Color { r: 0.40, g: 0.40, b: 0.50, a: 1.0 }),
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
                let cursor_on = (state.blink_tick % 62) < 31;
                TerminalPane::new(
                    Arc::clone(&emu.grid),
                    is_waiting,
                    state.config.font_size as f32,
                    state.font_name,
                    cursor_on,
                ).view()
            }
        } else if state.panes.contains_key(&state.active) {
            iced::widget::text("Starting...").into()
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
