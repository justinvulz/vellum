use crate::app::{App, AppAction};
use crate::search;
use crate::vault::Vault;
use std::path::Path;

pub fn show(app: &mut App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;

    ui.add(egui::Label::new(egui::RichText::new("Vellum").heading()).truncate(true));
    ui.add(egui::Label::new(app.vault.root.display().to_string()).truncate(true));
    ui.separator();

    let narrow = ui.available_width() < 140.0;

    if let Some(a) = create_row(
        ui,
        &mut app.new_note_name,
        "new note…",
        "New",
        narrow,
        AppAction::CreateNote,
    ) {
        action = Some(a);
    }
    if let Some(a) = create_row(
        ui,
        &mut app.new_folder_name,
        "new folder…",
        "Folder",
        narrow,
        AppAction::CreateFolder,
    ) {
        action = Some(a);
    }

    ui.add_space(4.0);
    ui.add(egui::TextEdit::singleline(&mut app.search_query)
        .hint_text("search…")
        .desired_width(f32::INFINITY));

    ui.separator();
    egui::ScrollArea::vertical()
        .id_source("vault-list")
        .show(ui, |ui| {
            let query = app.search_query.to_ascii_lowercase();
            for folder in &app.vault.folders {
                let label = rel_to_notes(&app.vault, folder);
                if !query.is_empty() && !label.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let resp = ui.add(
                    egui::Label::new(format!("📁 {label}"))
                        .sense(egui::Sense::click()),
                );
                resp.context_menu(|ui| {
                    if ui.button("Delete (must be empty)").clicked() {
                        action = Some(AppAction::DeleteFolder(folder.clone()));
                        ui.close_menu();
                    }
                });
            }

            let matches = search::filename_search(&app.vault, &app.search_query);
            for path in &matches {
                let label = rel_to_notes(&app.vault, path);
                let is_selected = app.selected.as_ref() == Some(path);
                let resp = ui.selectable_label(is_selected, label);
                if resp.clicked() {
                    action = Some(AppAction::OpenNote(path.clone()));
                }
                resp.context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        action = Some(AppAction::DeleteNote(path.clone()));
                        ui.close_menu();
                    }
                });
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

/// One "[text input] [button]" row that emits `make(value)` when the
/// user presses Enter in the input or clicks the button. Stacks the
/// button under the input when the sidebar is narrow.
fn create_row(
    ui: &mut egui::Ui,
    buffer: &mut String,
    hint: &str,
    button_label: &str,
    narrow: bool,
    make: fn(String) -> AppAction,
) -> Option<AppAction> {
    let mut submit = false;
    let resp = ui.add(
        egui::TextEdit::singleline(buffer)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        submit = true;
    }
    if narrow {
        if ui
            .add(
                egui::Button::new(button_label)
                    .min_size(egui::vec2(ui.available_width(), 0.0)),
            )
            .clicked()
        {
            submit = true;
        }
    } else if ui.button(button_label).clicked() {
        submit = true;
    }
    if submit && !buffer.trim().is_empty() {
        let name = buffer.trim().to_string();
        buffer.clear();
        return Some(make(name));
    }
    None
}

/// Strip the `<vault>/note/` prefix for sidebar labels, falling back
/// to the full path if the prefix isn't present.
fn rel_to_notes(vault: &Vault, path: &Path) -> String {
    let notes_dir = vault.root.join("note");
    path.strip_prefix(&notes_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
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
