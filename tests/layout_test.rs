use termpp::multiplexer::layout::{Layout, SplitDirection};

#[test]
fn new_layout_is_single_leaf() {
    let l = Layout::new(0);
    assert_eq!(l.pane_ids(), vec![0]);
    assert_eq!(l.depth(), 0);
}

#[test]
fn split_adds_new_pane() {
    let l = Layout::new(0).split(0, SplitDirection::Vertical, 1).unwrap();
    assert_eq!(l.pane_ids(), vec![0, 1]);
    assert_eq!(l.depth(), 1);
}

#[test]
fn split_blocked_at_max_depth() {
    let mut l = Layout::new(0);
    for i in 1..=Layout::MAX_DEPTH {
        let target = *l.pane_ids().last().unwrap();
        l = l.split(target, SplitDirection::Vertical, i).unwrap();
    }
    let target = *l.pane_ids().last().unwrap();
    assert!(l.split(target, SplitDirection::Vertical, 99).is_none());
}

#[test]
fn remove_pane_leaves_sibling() {
    let l = Layout::new(0).split(0, SplitDirection::Vertical, 1).unwrap();
    let l = l.remove(0).unwrap();
    assert_eq!(l.pane_ids(), vec![1]);
}

#[test]
fn cannot_remove_last_pane() {
    assert!(Layout::new(0).remove(0).is_none());
}
