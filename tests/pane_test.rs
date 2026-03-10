use termpp::multiplexer::pane::{PaneState, PaneStatus};
use std::time::Duration;

#[test]
fn new_pane_is_running() {
    let p = PaneState::new(0, std::path::PathBuf::from("/tmp"));
    assert_eq!(p.status, PaneStatus::Running);
}

#[test]
fn output_resets_waiting_to_running() {
    let mut p = PaneState::new(0, std::path::PathBuf::from("/tmp"));
    p.on_notify();
    assert_eq!(p.status, PaneStatus::Waiting);
    p.on_output();
    assert_eq!(p.status, PaneStatus::Running);
}

#[test]
fn exit_transitions_to_dead() {
    let mut p = PaneState::new(0, std::path::PathBuf::from("/tmp"));
    p.on_exit();
    assert_eq!(p.status, PaneStatus::Dead);
}

#[test]
fn is_idle_with_zero_threshold() {
    let mut p = PaneState::new(0, std::path::PathBuf::from("/tmp"));
    p.on_output();
    assert!(p.is_idle_for(Duration::from_secs(0)));
}

#[test]
fn waiting_pane_is_not_idle() {
    let mut p = PaneState::new(0, std::path::PathBuf::from("/tmp"));
    p.on_notify();
    assert!(!p.is_idle_for(Duration::from_secs(0)));
}
