use iced::widget::text;
use iced::{Element, Task};

#[derive(Default)]
pub struct Termpp;

#[derive(Debug, Clone)]
pub enum Message {}

pub fn boot() -> (Termpp, Task<Message>) {
    (Termpp, Task::none())
}

pub fn title(_state: &Termpp) -> String {
    "termpp".to_string()
}

pub fn update(_state: &mut Termpp, _message: Message) -> Task<Message> {
    Task::none()
}

pub fn view(_state: &Termpp) -> Element<'_, Message> {
    text("termpp — starting").into()
}
