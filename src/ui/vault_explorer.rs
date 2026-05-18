use crate::app::{App, AppAction};
use crate::search;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
    ui.add(
        egui::TextEdit::singleline(&mut app.search_query)
            .hint_text("search…")
            .desired_width(f32::INFINITY),
    );

    ui.separator();

    let query = app.search_query.to_ascii_lowercase();
    let notes_root = app.vault.root.join("note");
    let force_open = folders_with_matches(app, &notes_root, &query);

    egui::ScrollArea::vertical()
        .id_source("vault-list")
        .show(ui, |ui| {
            let dragging =
                egui::DragAndDrop::has_payload_of_type::<PathBuf>(ui.ctx());

            if dragging {
                let (_, dropped) = ui.dnd_drop_zone::<PathBuf, _>(
                    egui::Frame::default().inner_margin(2.0),
                    |ui| {
                        ui.add(egui::Label::new("📂 (root)"));
                    },
                );
                if let Some(payload) = dropped {
                    action = Some(AppAction::MoveNote {
                        from: (*payload).clone(),
                        to_folder: None,
                    });
                }
            }

            render_tree(
                ui,
                app,
                &notes_root,
                0,
                &query,
                &force_open,
                &mut action,
            );

            if !query.is_empty() {
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

/// Walk ancestors of every note whose stem matches the query and
/// collect the set of folders we need to force-open so the matching
/// notes are visible in the tree.
fn folders_with_matches(app: &App, notes_root: &Path, query: &str) -> HashSet<PathBuf> {
    let mut s = HashSet::new();
    if query.is_empty() {
        return s;
    }
    for note in &app.vault.notes {
        let stem_match = note
            .file_stem()
            .map(|s| s.to_string_lossy().to_ascii_lowercase().contains(query))
            .unwrap_or(false);
        if !stem_match {
            continue;
        }
        let mut current = note.parent();
        while let Some(parent) = current {
            if !parent.starts_with(notes_root) || parent == notes_root {
                break;
            }
            s.insert(parent.to_path_buf());
            current = parent.parent();
        }
    }
    s
}

fn render_tree(
    ui: &mut egui::Ui,
    app: &App,
    parent: &Path,
    depth: usize,
    query: &str,
    force_open: &HashSet<PathBuf>,
    action: &mut Option<AppAction>,
) {
    let folders: Vec<PathBuf> = app
        .vault
        .folders
        .iter()
        .filter(|f| f.parent() == Some(parent))
        .cloned()
        .collect();
    let notes: Vec<PathBuf> = app
        .vault
        .notes
        .iter()
        .filter(|n| n.parent() == Some(parent))
        .cloned()
        .collect();

    for folder in folders {
        render_folder(ui, app, &folder, depth, query, force_open, action);
    }
    for note in notes {
        render_note(ui, app, &note, depth, query, action);
    }
}

fn render_folder(
    ui: &mut egui::Ui,
    app: &App,
    folder: &Path,
    depth: usize,
    query: &str,
    force_open: &HashSet<PathBuf>,
    action: &mut Option<AppAction>,
) {
    let name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string());
    let open_id = egui::Id::new(("vault-folder-open", folder));
    let persisted = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let is_open = persisted || force_open.contains(folder);

    let (inner, dropped) = ui.dnd_drop_zone::<PathBuf, _>(
        egui::Frame::default(),
        |ui| {
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * INDENT_PX);
                let chevron = if is_open { "▼" } else { "▶" };
                ui.add(
                    egui::Label::new(format!("{chevron}  {name}"))
                        .sense(egui::Sense::click()),
                )
            })
            .inner
        },
    );
    let row_resp = inner.inner;

    if row_resp.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(open_id, !persisted));
    }

    let folder_owned = folder.to_path_buf();
    row_resp.context_menu(|ui| {
        if ui.button("Delete (must be empty)").clicked() {
            *action = Some(AppAction::DeleteFolder(folder_owned.clone()));
            ui.close_menu();
        }
    });

    if let Some(payload) = dropped {
        *action = Some(AppAction::MoveNote {
            from: (*payload).clone(),
            to_folder: Some(folder_owned),
        });
    }

    if is_open {
        render_tree(ui, app, folder, depth + 1, query, force_open, action);
    }
}

fn render_note(
    ui: &mut egui::Ui,
    app: &App,
    path: &Path,
    depth: usize,
    query: &str,
    action: &mut Option<AppAction>,
) {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    if !query.is_empty() && !name.to_ascii_lowercase().contains(query) {
        return;
    }

    let is_selected = app.selected.as_deref() == Some(path);
    let label_resp = ui
        .horizontal(|ui| {
            // Notes sit one chevron-width deeper than their folder so the
            // text aligns with the folder name, not the chevron.
            ui.add_space(depth as f32 * INDENT_PX + CHEVRON_PX);
            ui.add(egui::SelectableLabel::new(is_selected, name))
        })
        .inner;

    let drag_id = egui::Id::new(("vault-note-drag", path));
    let resp = ui.interact(
        label_resp.rect,
        drag_id,
        egui::Sense::click_and_drag(),
    );
    resp.dnd_set_drag_payload(path.to_path_buf());
    if resp.clicked() {
        *action = Some(AppAction::OpenNote(path.to_path_buf()));
    }
    resp.context_menu(|ui| {
        if ui.button("Delete").clicked() {
            *action = Some(AppAction::DeleteNote(path.to_path_buf()));
            ui.close_menu();
        }
    });
}

const INDENT_PX: f32 = 14.0;
const CHEVRON_PX: f32 = 14.0;

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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}
