use crate::app::{App, AppAction};
use crate::search;
use crate::style;
use egui_phosphor::regular as icon;

pub fn show(app: &App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;
    ui.add_space(8.0);
    ui.heading(format!("{}  Backlinks", icon::LINK));
    style::soft_separator(ui);
    if let Some(current) = app.selected.as_ref() {
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
    } else {
        ui.label("(no note selected)");
    }
    ui.add_space(8.0);
    action
}
