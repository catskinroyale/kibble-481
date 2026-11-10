// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod live;
mod mixer_ui;
mod panels;
mod roll;
mod snapshot;
mod sound;
mod theme;
mod widgets;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(theme::APP_NAME)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 680.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../../packaging/macos/icon_1024.png")).expect("valid icon png"),
            ),
        ..Default::default()
    };
    // the settings are stored under the app's first name, so they carry over
    let name = if std::env::var_os("DDREAM_SCREENSHOT").is_some() { "DDream (screenshot)" } else { "DDream" };
    eframe::run_native(name, options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
