mod app;
use app::InputWindow;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([860.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CriticalFlow",
        options,
        Box::new(|cc| Ok(Box::new(InputWindow::new(cc)))),
    )
}
