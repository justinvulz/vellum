use crate::app::{App, AppAction};

pub fn show(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;

    let Some(path) = app.selected.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Select or create a note in the sidebar.");
        });
        return None;
    };

    ui.horizontal(|ui| {
        ui.label(app.vault.display_name(&path));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Save (Ctrl+S)").clicked() {
                action = Some(AppAction::SaveCurrent);
            }
            if ui.button("Open in Helix (Ctrl+E)").clicked() {
                action = Some(AppAction::OpenInHelix);
            }
            if ui.button("Reload").clicked() {
                action = Some(AppAction::ReloadCurrent);
            }
        });
    });
    ui.separator();

    let App { mixed, engine, .. } = app;
    mixed.show(ctx, ui, engine);

    action
}
