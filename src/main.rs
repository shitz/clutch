mod app;
mod rpc;
mod screens;

fn main() -> iced::Result {
    iced::application("Clutch", app::update, app::view)
        .subscription(app::subscription)
        .run_with(|| (app::AppState::new(), iced::Task::none()))
}
