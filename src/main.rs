mod app;
mod editor_backend;
mod git;
mod helix_editor;
mod mixed_editor;
mod search;
mod segment;
mod typst_engine;
mod ui;
mod vault;

use anyhow::Result;
use app::App;
use vault::Vault;

fn main() -> Result<()> {
    let root = vault::default_vault_dir();
    let vault = Vault::open_or_init(root)?;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Vellum"),
        ..Default::default()
    };

    eframe::run_native(
        "Vellum",
        native_options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            let bg = egui::Color32::from_rgb(0x0d, 0x0d, 0x0d);
            visuals.panel_fill = bg;
            visuals.window_fill = bg;
            visuals.extreme_bg_color = bg;
            cc.egui_ctx.set_visuals(visuals);
            Box::new(App::new(vault))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
