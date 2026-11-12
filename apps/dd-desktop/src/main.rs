// No console window behind the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod app;
mod audio;
mod live;
mod mixer_ui;
mod panels;
mod roll;
mod snapshot;
mod sound;
mod theme;
mod update;
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
    let name = if std::env::var_os("KIBBLE_SCREENSHOT").is_some() { "Kibble-481 (screenshot)" } else { "Kibble-481" };
    carry_over_settings(name);
    eframe::run_native(name, options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

/// Settings saved under the app's old name (DDream) move to the new one the first time
/// Kibble-481 runs, so the palette, window and recent choices carry over.
fn carry_over_settings(name: &str) {
    let old_name = name.replace("Kibble-481", "DDream");
    let (Some(new), Some(old)) = (eframe::storage_dir(name), eframe::storage_dir(&old_name)) else { return };
    if new.exists() || !old.is_dir() {
        return;
    }
    let copy = || -> std::io::Result<()> {
        std::fs::create_dir_all(&new)?;
        for entry in std::fs::read_dir(&old)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::copy(entry.path(), new.join(entry.file_name()))?;
            }
        }
        Ok(())
    };
    if let Err(e) = copy() {
        eprintln!("couldn't carry over the settings from {}: {e}", old.display());
    }
}
