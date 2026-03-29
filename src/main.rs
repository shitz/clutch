mod app;
mod format;
mod profile;
mod rpc;
mod screens;
mod theme;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    iced::application(app::AppState::new, app::update, app::view)
        .title("Clutch")
        .subscription(app::subscription)
        .theme(app::AppState::current_theme)
        // Material Icons font for toolbar and inspector glyphs
        .font(theme::MATERIAL_ICONS_BYTES)
        // iced_aw's internal font (required for Tabs widget rendering)
        .font(iced_aw::ICED_AW_FONT_BYTES)
        // Minimum window size: wide enough for all 9 torrent list columns
        .window(iced::window::Settings {
            min_size: Some(iced::Size {
                width: 900.0,
                height: 500.0,
            }),
            ..Default::default()
        })
        .run()
}
