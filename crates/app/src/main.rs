mod app;
use app::InputWindow;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CriticalFlow",
        options,
        Box::new(|cc| Ok(Box::new(InputWindow::new(cc)))),
    )
}
