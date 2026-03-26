use std::time::Duration;
use crate::terminal::grid::TermEvent;
use crate::multiplexer::pane::PaneState;

pub struct NotificationDetector {
    idle_timeout: Duration,
}

impl NotificationDetector {
    pub fn new(idle_timeout: Duration) -> Self {
        Self { idle_timeout }
    }

    pub fn process_event(&self, event: TermEvent, pane: &mut PaneState) {
        match event {
            TermEvent::Bell | TermEvent::OscNotify(_) => pane.on_notify(),
            TermEvent::CwdChange(_) => {} // cwd update is handled upstream in app.rs
            TermEvent::Exited => pane.on_exit(),
        }
    }

    pub fn check_idle(&self, pane: &mut PaneState) {
        if pane.is_idle_for(self.idle_timeout) {
            pane.on_notify();
        }
    }
}
