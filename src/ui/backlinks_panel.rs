use crate::app::{App, AppAction};
use crate::search;

pub fn show(app: &App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;
    ui.heading("Backlinks");
    ui.separator();
    let Some(current) = app.selected.as_ref() else {
        ui.label("(no note selected)");
        return None;
    };
    match search::backlinks_for(&app.backlinks, current) {
        Some(refs) if !refs.is_empty() => {
            for path in refs {
                let label = app.vault.display_name(path);
                if ui.selectable_label(false, label).clicked() {
                    action = Some(AppAction::OpenNote(path.clone()));
                }
            }
        }
        _ => {
            ui.label("No notes link here yet.");
        }
    }
    action
}
