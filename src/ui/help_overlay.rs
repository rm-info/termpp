use iced::Element;
use crate::config::Keybindings;

pub fn help_overlay<Message: Clone + 'static>(
    _keybindings: &Keybindings,
    _on_close: Message,
) -> Element<'static, Message> {
    iced::widget::text("TODO").into()
}
