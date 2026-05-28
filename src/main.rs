mod app;
mod config;
mod editor;
mod external_editor;
mod file_watcher;
mod search;
mod style;
mod ui;
mod vault;

use anyhow::Result;
use app::App;
use vault::Vault;

fn main() -> Result<()> {
    // Default: info for everything, debug for the `vellum` crate. Override
    // with e.g. `RUST_LOG=trace` or `RUST_LOG=vellum::editor=trace`.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,vellum=debug"),
    )
    .init();
    log::info!("vellum starting");

    // Load on-disk config *before* anything reads the style/config
    // accessors so the first call to `config::current()` returns the
    // user's overrides rather than baking in the defaults.
    let cfg = config::load();
    let root = cfg.vault_dir();
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
            style::install(&cc.egui_ctx);
            Ok(Box::new(App::new(vault)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
