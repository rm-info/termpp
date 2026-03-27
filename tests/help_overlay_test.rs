use termpp::config::Keybindings;
use termpp::ui::help_overlay::help_overlay;

#[test]
fn help_overlay_builds_without_panic() {
    let kb = Keybindings::default();
    // Construct the widget tree — no runtime needed for construction.
    // If this panics, the widget has a logic error.
    let _el: iced::Element<'static, ()> = help_overlay(&kb, ());
}

#[test]
fn help_overlay_uses_keybinding_strings() {
    // Assign ALL fields to ensure the widget clones every keybinding string.
    // If any field is borrowed (not cloned), dropping kb before _el would fail
    // to compile because Element<'static, _> must not hold a borrow from kb.
    let kb = Keybindings {
        split_horizontal: "ctrl+shift+test_h".to_string(),
        split_vertical:   "ctrl+shift+test_v".to_string(),
        pane_next:        "ctrl+shift+test_n".to_string(),
        pane_prev:        "ctrl+shift+test_p".to_string(),
        new_pane:         "ctrl+shift+test_np".to_string(),
        close_pane:       "ctrl+shift+test_w".to_string(),
        rename_pane:      "ctrl+shift+test_r".to_string(),
    };
    let _el: iced::Element<'static, ()> = help_overlay(&kb, ());
    drop(kb); // kb is dropped before _el — would fail if _el borrowed from kb
    drop(_el);
}
