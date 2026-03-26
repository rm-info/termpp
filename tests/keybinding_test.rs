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
fn default_new_pane_is_ctrl_shift_n() {
    let kb = Keybindings::default();
    assert_eq!(kb.new_pane, "ctrl+shift+n");
}
