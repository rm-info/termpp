mod app;

fn main() {
    env_logger::init();
    let code = match iced::application(app::boot, app::update, app::view)
        .title(app::title)
        .subscription(app::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(app::WINDOW_W, app::WINDOW_H),
            ..Default::default()
        })
        .run()
    {
        Ok(()) => 0,
        Err(e) => { eprintln!("iced error: {e}"); 1 }
    };
    // Force-kill PTY reader threads that block tokio runtime shutdown.
    std::process::exit(code);
}
