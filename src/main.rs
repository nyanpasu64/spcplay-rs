mod app;
mod spcplay;

use anyhow::Result;

fn main() -> Result<()> {
    let app = app::SpcPlayApp::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    Ok(())
}
