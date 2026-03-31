mod app;
mod auth;
mod crypto;
mod format;
mod profile;
mod rpc;
mod screens;
mod theme;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    let icon = iced::window::icon::from_file_data(crate::theme::ICON_256_BYTES, None).ok();

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
                height: 600.0,
            }),
            icon,
            ..Default::default()
        })
        .run()
}
