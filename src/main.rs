mod app;

use anyhow::Result;

fn main() -> Result<()> {
    let app = app::TemplateApp::new()?;
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    Ok(())
}
