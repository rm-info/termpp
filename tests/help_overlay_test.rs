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
    let mut kb = Keybindings::default();
    kb.split_horizontal = "ctrl+shift+test".to_string();
    // Must not borrow from kb — widget must own all strings
    let _el: iced::Element<'static, ()> = help_overlay(&kb, ());
    drop(kb); // kb is dropped before _el — would fail if _el borrowed from kb
    drop(_el);
}
