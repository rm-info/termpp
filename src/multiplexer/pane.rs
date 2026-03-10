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

/// Detect current git branch in `cwd` via git rev-parse subprocess.
/// Returns None if directory is not in a git repo.
pub fn detect_git_branch(cwd: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() || branch == "HEAD" { None } else { Some(branch) }
    } else {
        None
    }
}
