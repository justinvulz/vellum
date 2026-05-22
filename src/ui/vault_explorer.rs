//! Sidebar file tree, search, and create / delete / move actions.
//!
//! The tree itself is rendered by [`egui_ltreeview`]; this module shapes
//! the vault into the tree, translates the tree's `Action` values into
//! `AppAction`s, and threads search-driven force-open and content-match
//! results around it.

use crate::app::{App, AppAction};
use crate::search;
use egui_ltreeview::{Action, NodeBuilder, TreeView, TreeViewState};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn show(app: &mut App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action: Option<AppAction> = None;

    ui.add(egui::Label::new(egui::RichText::new("Vellum").heading()).truncate());
    ui.add(egui::Label::new(app.vault.root.display().to_string()).truncate());
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
        .id_salt("vault-list")
        .show(ui, |ui| {
            if let Some(a) = show_tree(app, ui, &notes_root, &query, &force_open) {
                action = Some(a);
            }

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
                    if ui.add(egui::Button::selectable(false, lbl)).clicked() {
                        action = Some(AppAction::OpenNote(hit.path.clone()));
                    }
                }
            }
        });

    action
}

/// Render the `egui_ltreeview` tree and translate the actions it emits
/// (click-to-open, drop-to-move, context-menu delete) into `AppAction`s.
///
/// The notes root (`<vault>/note`) is added as a *flattened* dir node so
/// it doesn't show up as an extra level in the UI but is still a valid
/// drop target — that's how a note dragged out of a subfolder lands
/// back in the vault root.
fn show_tree(
    app: &App,
    ui: &mut egui::Ui,
    notes_root: &Path,
    query: &str,
    force_open: &HashSet<PathBuf>,
) -> Option<AppAction> {
    let tree_id = egui::Id::new("vellum-vault-tree");
    let mut state =
        TreeViewState::<PathBuf>::load(ui, tree_id).unwrap_or_default();
    // While a search is active, force-open every folder along the path
    // to any matching note. `set_openness` overrides the user's stored
    // state for this frame only — collapse persists once the search
    // clears because nothing re-sets these folders to true.
    for folder in force_open {
        state.set_openness(folder.clone(), true);
    }
    // Seed the selection from the currently open note so the tree
    // highlights it; we ignore the resulting SetSelected echo below.
    if let Some(current) = app.selected.clone() {
        if state.selected().as_slice() != [current.clone()] {
            state.set_one_selected(current);
        }
    }

    let pending: RefCell<Option<AppAction>> = RefCell::new(None);

    let (_resp, actions) = TreeView::new(tree_id)
        .allow_multi_selection(false)
        .show_state(ui, &mut state, |builder| {
            // Invisible root that owns the entire tree — droppable, not
            // visible. Drops with `target == notes_root` mean "back to
            // root".
            builder.node(
                NodeBuilder::dir(notes_root.to_path_buf()).flatten(true),
            );
            build_subtree(builder, app, notes_root, query, force_open, &pending);
            builder.close_dir();
        });

    state.store(ui, tree_id);

    let mut result = pending.into_inner();

    for a in actions {
        match a {
            Action::SetSelected(selected) => {
                // Single-click on a note → open it. Folder selections
                // are ignored (folders aren't activatable). We also
                // skip the echo when the click selected what was
                // already the open note.
                if selected.len() == 1
                    && app.vault.notes.contains(&selected[0])
                    && app.selected.as_ref() != Some(&selected[0])
                {
                    result = Some(AppAction::OpenNote(selected[0].clone()));
                }
            }
            Action::Move(d) => {
                // Only let users drag notes, never folders.
                let Some(from) = d.source.into_iter().next() else { continue };
                if !app.vault.notes.contains(&from) {
                    continue;
                }
                let to_folder = if d.target == notes_root {
                    None
                } else {
                    Some(d.target)
                };
                result = Some(AppAction::MoveNote { from, to_folder });
            }
            // Ignored: Activate (we open on single-click already),
            // Drag (in-progress), DragExternal / MoveExternal (no
            // panel outside the tree accepts drops).
            Action::Activate(_)
            | Action::Drag(_)
            | Action::DragExternal(_)
            | Action::MoveExternal(_) => {}
        }
    }

    result
}

fn build_subtree(
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, PathBuf>,
    app: &App,
    parent: &Path,
    query: &str,
    force_open: &HashSet<PathBuf>,
    pending: &RefCell<Option<AppAction>>,
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
        let name = folder
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| folder.display().to_string());
        let folder_for_ctx = folder.clone();
        builder.node(
            NodeBuilder::dir(folder.clone())
                .label(format!("📁 {name}"))
                .default_open(false)
                .context_menu(move |ui| {
                    if ui.button("Delete (must be empty)").clicked() {
                        *pending.borrow_mut() =
                            Some(AppAction::DeleteFolder(folder_for_ctx.clone()));
                        ui.close();
                    }
                }),
        );
        build_subtree(builder, app, &folder, query, force_open, pending);
        builder.close_dir();
    }

    for note in notes {
        let stem = note
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| note.display().to_string());
        if !query.is_empty() && !stem.to_ascii_lowercase().contains(query) {
            continue;
        }
        let note_for_ctx = note.clone();
        builder.node(
            NodeBuilder::leaf(note.clone())
                .label(stem)
                .context_menu(move |ui| {
                    if ui.button("Delete").clicked() {
                        *pending.borrow_mut() =
                            Some(AppAction::DeleteNote(note_for_ctx.clone()));
                        ui.close();
                    }
                }),
        );
    }
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
