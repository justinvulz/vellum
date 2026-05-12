use crate::app::{App, AppAction};
use crate::search;

pub fn show(app: &mut App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;

    ui.heading("Vellum");
    ui.label(app.vault.root.display().to_string());
    ui.separator();

    ui.horizontal(|ui| {
        let resp = ui.text_edit_singleline(&mut app.new_note_name);
        if (ui.button("New").clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
            && !app.new_note_name.trim().is_empty()
        {
            action = Some(AppAction::CreateNote(app.new_note_name.trim().to_string()));
            app.new_note_name.clear();
        }
    });

    ui.add_space(4.0);
    ui.label("Search");
    ui.text_edit_singleline(&mut app.search_query);

    ui.separator();
    egui::ScrollArea::vertical()
        .id_source("vault-list")
        .show(ui, |ui| {
            let matches = search::filename_search(&app.vault, &app.search_query);
            for path in &matches {
                let label = app.vault.display_name(path);
                let is_selected = app.selected.as_ref() == Some(path);
                if ui.selectable_label(is_selected, label).clicked() {
                    action = Some(AppAction::OpenNote(path.clone()));
                }
            }

            if !app.search_query.is_empty() {
                ui.separator();
                ui.label("Content matches");
                for hit in search::content_search(&app.vault, &app.search_query, 25) {
                    let lbl = format!(
                        "{}:{}  {}",
                        app.vault.display_name(&hit.path),
                        hit.line,
                        truncate(&hit.snippet, 60)
                    );
                    if ui.selectable_label(false, lbl).clicked() {
                        action = Some(AppAction::OpenNote(hit.path.clone()));
                    }
                }
            }
        });

    action
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}
