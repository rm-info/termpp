use termpp::config::Keybindings;

#[test]
fn default_pane_next_is_ctrl_tab() {
    let kb = Keybindings::default();
    assert_eq!(kb.pane_next, "ctrl+tab");
}

#[test]
fn default_pane_prev_is_ctrl_shift_tab() {
    let kb = Keybindings::default();
    assert_eq!(kb.pane_prev, "ctrl+shift+tab");
}

#[test]
fn default_rename_pane_is_ctrl_shift_r() {
    let kb = Keybindings::default();
    assert_eq!(kb.rename_pane, "ctrl+shift+r");
}

#[test]
fn default_close_pane_is_ctrl_shift_q() {
    let kb = Keybindings::default();
    assert_eq!(kb.close_pane, "ctrl+shift+q");
}

#[test]
fn default_tab_next_is_ctrl_pagedown() {
    let kb = Keybindings::default();
    assert_eq!(kb.tab_next, "ctrl+pagedown");
}

#[test]
fn default_workspace_new_is_ctrl_shift_w() {
    let kb = Keybindings::default();
    assert_eq!(kb.workspace_new, "ctrl+shift+w");
}
