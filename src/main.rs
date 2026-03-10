mod app;

fn main() -> iced::Result {
    env_logger::init();
    iced::application(app::boot, app::update, app::view)
        .title(app::title)
        .subscription(app::subscription)
        .run()
}
