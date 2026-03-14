mod app;
mod rpc;
mod screens;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    iced::application(app::AppState::new, app::update, app::view)
        .title("Clutch")
        .subscription(app::subscription)
        .run()
}
