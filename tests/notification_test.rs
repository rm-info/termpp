use termpp::multiplexer::notification::NotificationDetector;
use termpp::terminal::grid::TermEvent;
use termpp::multiplexer::pane::{PaneState, PaneStatus};
use std::time::Duration;
use std::path::PathBuf;

fn make_pane() -> PaneState {
    PaneState::new(0, PathBuf::from("/tmp"))
}

#[test]
fn bel_event_triggers_notify() {
    let detector = NotificationDetector::new(Duration::from_secs(5));
    let mut pane = make_pane();
    detector.process_event(TermEvent::Bell, &mut pane);
    assert_eq!(pane.status, PaneStatus::Waiting);
}

#[test]
fn osc_notify_triggers_notify() {
    let detector = NotificationDetector::new(Duration::from_secs(5));
    let mut pane = make_pane();
    detector.process_event(TermEvent::OscNotify("test".into()), &mut pane);
    assert_eq!(pane.status, PaneStatus::Waiting);
}

#[test]
fn idle_timeout_triggers_notify() {
    let detector = NotificationDetector::new(Duration::from_secs(0));
    let mut pane = make_pane();
    pane.on_output();
    detector.check_idle(&mut pane);
    assert_eq!(pane.status, PaneStatus::Waiting);
}

#[test]
fn exited_event_transitions_to_dead() {
    let detector = NotificationDetector::new(Duration::from_secs(5));
    let mut pane = make_pane();
    detector.process_event(TermEvent::Exited, &mut pane);
    assert_eq!(pane.status, PaneStatus::Dead);
}
