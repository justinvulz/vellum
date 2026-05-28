use crate::app::{App, AppAction};
use crate::style;
use egui_phosphor::regular as icon;

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
            if ui
                .button(format!("{}  Save", icon::FLOPPY_DISK))
                .on_hover_text("Ctrl+S")
                .clicked()
            {
                action = Some(AppAction::SaveCurrent);
            }
            if ui
                .button(format!("{}  Helix", icon::TERMINAL))
                .on_hover_text("Ctrl+E")
                .clicked()
            {
                action = Some(AppAction::OpenInHelix);
            }
            if ui
                .button(format!("{}  Reload", icon::ARROW_CLOCKWISE))
                .clicked()
            {
                action = Some(AppAction::ReloadCurrent);
            }
        });
    });
    style::soft_separator(ui);

    let App { mixed, engine, .. } = app;
    if let Some(target) = mixed.show(ctx, ui, engine) {
        action = Some(AppAction::OpenNoteByName(target));
    }

    action
}
