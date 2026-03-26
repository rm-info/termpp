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
    }
    Task::none()
}

pub fn subscription(_state: &Termpp) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
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
