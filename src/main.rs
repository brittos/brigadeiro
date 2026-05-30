// SPDX-License-Identifier: GPL-3.0

mod app;
mod config;
mod database;
mod i18n;
mod model;
mod player;
mod utils;
mod views;
mod widgets;

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(360.0)
                .min_height(180.0),
        )
        .size(cosmic::iced::Size::new(1240.0, 768.0))
        // Window decorations (Close, Minimize, Maximize buttons)
        // true = Client Side Decorations (COSMIC style) - Default
        // false = Server Side Decorations (System style) or None depending on environment
        // We disable this to implement a custom frameless header
        .client_decorations(true)
        // Enable transparency to allow the app background to blend with the window frame
        .transparent(false);

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
