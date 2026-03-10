use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum PaneStatus {
    Running,
    Waiting,
    Dead,
}

#[derive(Debug, Clone)]
pub struct PaneState {
    pub id: usize,
    pub status: PaneStatus,
    pub last_output_at: Instant,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
}

impl PaneState {
    pub fn new(id: usize, cwd: PathBuf) -> Self {
        Self {
            id,
            status: PaneStatus::Running,
            last_output_at: Instant::now(),
            cwd,
            git_branch: None,
        }
    }

    pub fn on_output(&mut self) {
        self.last_output_at = Instant::now();
        if self.status == PaneStatus::Waiting {
            self.status = PaneStatus::Running;
        }
    }

    pub fn on_notify(&mut self) {
        if self.status == PaneStatus::Running {
            self.status = PaneStatus::Waiting;
        }
    }

    pub fn on_exit(&mut self) {
        self.status = PaneStatus::Dead;
    }

    pub fn is_idle_for(&self, duration: Duration) -> bool {
        self.status == PaneStatus::Running
            && self.last_output_at.elapsed() >= duration
    }
}
