use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Duration;
use iced::{Element, Subscription, Task};
use iced::widget::{container, row};
use iced::Length;

use termpp::config::Config;
use termpp::multiplexer::layout::{Layout, SplitDirection};
use termpp::multiplexer::notification::NotificationDetector;
use termpp::multiplexer::pane::{PaneState, PaneStatus, detect_git_branch};
use termpp::terminal::emulator::Emulator;
use termpp::ui::pane_grid::TerminalPane;
use termpp::ui::sidebar::{Sidebar, WorkspaceEntry};

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
    renaming_pane:      Option<(usize, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SplitPane(SplitDirection),
    ClosePane,
    FocusNext,
    ToggleHelp,
    CancelRename,
    KeyInput(Vec<u8>),
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
        renaming_pane:      None,
    };

    // Emulator::start() is sync — uses tokio::spawn internally
    match Emulator::start(220, 50) {
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
            // Fix 1: Drain TermEvents from every emulator and forward to detector.
            // Fix 2: Detect new PTY output via output_count and call on_output().
            // We use try_lock() so a slow reader thread can't stall the UI tick.
            for (&pane_id, emu_arc) in &state.emulators {
                if let Ok(mut emu) = emu_arc.try_lock() {
                    // Fix 2: check if PTY output_count advanced since last tick
                    let current_count = emu.output_count.load(Ordering::Relaxed);
                    let last_count = state.last_output_counts.get(&pane_id).copied().unwrap_or(0);
                    if current_count != last_count {
                        state.last_output_counts.insert(pane_id, current_count);
                        if let Some(pane) = state.panes.get_mut(&pane_id) {
                            pane.on_output();
                        }
                    }

                    // Fix 1: drain queued TermEvents (Bell, OscNotify, Exited)
                    while let Ok(event) = emu.event_rx.try_recv() {
                        if let Some(pane) = state.panes.get_mut(&pane_id) {
                            state.detector.process_event(event, pane);
                        }
                    }
                }
            }

            for pane in state.panes.values_mut() {
                state.detector.check_idle(pane);
                if pane.status != PaneStatus::Dead {
                    pane.git_branch = detect_git_branch(&pane.cwd);
                }
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
                match Emulator::start(220, 50) {
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
        Message::CancelRename => {
            state.renaming_pane = None;
        }
        Message::KeyInput(bytes) => {
            if let Some(emu_arc) = state.emulators.get(&state.active) {
                if let Ok(emu) = emu_arc.try_lock() {
                    let _ = emu.write_input(&bytes);
                }
            }
        }
    }
    Task::none()
}

/// Parse a binding string like "ctrl+shift+h" and match against a key event.
fn matches_binding(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    binding: &str,
) -> bool {
    let parts: Vec<&str> = binding.split('+').collect();
    let mut want_ctrl  = false;
    let mut want_shift = false;
    let mut want_alt   = false;
    let mut key_char: Option<char> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl"  => want_ctrl  = true,
            "shift" => want_shift = true,
            "alt"   => want_alt   = true,
            s if s.len() == 1 => key_char = s.chars().next(),
            _ => {}
        }
    }

    if want_ctrl  != modifiers.control() { return false; }
    if want_shift != modifiers.shift()   { return false; }
    if want_alt   != modifiers.alt()     { return false; }

    if let Some(ch) = key_char {
        matches!(key, iced::keyboard::Key::Character(c) if c.as_str().eq_ignore_ascii_case(&ch.to_string()))
    } else {
        false
    }
}

/// Convert a key event into the byte sequence to send to the PTY.
fn key_to_bytes(
    key: &iced::keyboard::Key,
    _modifiers: iced::keyboard::Modifiers,
    text: Option<&str>,
) -> Vec<u8> {
    use iced::keyboard::key::Named;

    // Printable text (covers regular characters, shifted symbols, etc.)
    if let Some(t) = text {
        if !t.is_empty() && !t.chars().all(|c| c.is_control()) {
            return t.as_bytes().to_vec();
        }
    }

    // Named keys → terminal escape sequences
    match key {
        iced::keyboard::Key::Named(Named::Enter)     => b"\r".to_vec(),
        iced::keyboard::Key::Named(Named::Backspace)  => b"\x7f".to_vec(),
        iced::keyboard::Key::Named(Named::Tab)        => b"\t".to_vec(),
        iced::keyboard::Key::Named(Named::Escape)     => b"\x1b".to_vec(),
        iced::keyboard::Key::Named(Named::ArrowUp)    => b"\x1b[A".to_vec(),
        iced::keyboard::Key::Named(Named::ArrowDown)  => b"\x1b[B".to_vec(),
        iced::keyboard::Key::Named(Named::ArrowRight) => b"\x1b[C".to_vec(),
        iced::keyboard::Key::Named(Named::ArrowLeft)  => b"\x1b[D".to_vec(),
        iced::keyboard::Key::Named(Named::Home)       => b"\x1b[H".to_vec(),
        iced::keyboard::Key::Named(Named::End)        => b"\x1b[F".to_vec(),
        iced::keyboard::Key::Named(Named::Delete)     => b"\x1b[3~".to_vec(),
        iced::keyboard::Key::Named(Named::PageUp)     => b"\x1b[5~".to_vec(),
        iced::keyboard::Key::Named(Named::PageDown)   => b"\x1b[6~".to_vec(),
        _ => vec![],
    }
}

pub fn subscription(state: &Termpp) -> Subscription<Message> {
    let tick = iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick);

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

    Subscription::batch([tick, keyboard])
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
        Sidebar::<Message>::new(&workspace_entries, state.active).view();

    // TerminalPane::view() returns Element<'static, Message> (Arc clone, no borrows).
    let pane_view: Element<'static, Message> =
        if let (Some(pane), Some(emu_arc)) = (
            state.panes.get(&state.active),
            state.emulators.get(&state.active),
        ) {
            let emu: std::sync::MutexGuard<'_, Emulator> =
                emu_arc.lock().unwrap_or_else(|e| e.into_inner());
            let is_waiting = pane.status == PaneStatus::Waiting;
            TerminalPane::new(
                Arc::clone(&emu.grid),
                is_waiting,
                state.config.font_size as f32,
            ).view()
        } else if state.panes.contains_key(&state.active) {
            iced::widget::text("Starting...").into()
        } else {
            iced::widget::text("No pane").into()
        };

    container(row![sidebar, pane_view])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
