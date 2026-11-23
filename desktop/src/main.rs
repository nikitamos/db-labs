mod api;
mod app;
mod model;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Project Management Desktop"),
        ..Default::default()
    };

    eframe::run_native(
        "Project Management Desktop",
        options,
        Box::new(|cc| Ok(Box::new(app::DesktopApp::new(cc)))),
    )
}
