use crate::editor_backend::FileWatcher;
use crate::git::GitSync;
use crate::helix_editor::HelixEditor;
use crate::mixed_editor::MixedEditor;
use crate::search::{self, BacklinkIndex};
use crate::typst_engine::TypstEngine;
use crate::ui;
use crate::vault::Vault;
use std::path::PathBuf;

pub enum AppAction {
    OpenNote(PathBuf),
    CreateNote(String),
    SaveCurrent,
    ReloadCurrent,
    OpenInHelix,
}

pub struct App {
    pub vault: Vault,
    pub selected: Option<PathBuf>,
    pub editor: HelixEditor,
    pub mixed: MixedEditor,
    pub engine: TypstEngine,
    pub new_note_name: String,
    pub search_query: String,
    pub backlinks: BacklinkIndex,
    pub sidebar_open: bool,
    pub watcher: Option<FileWatcher>,
    pub git: GitSync,
    pub status: String,
}

impl App {
    pub fn new(vault: Vault) -> Self {
        let watcher = FileWatcher::new(&vault.root).ok();
        let backlinks = search::build_backlinks(&vault);
        let engine = TypstEngine::new(vault.root.clone())
            .expect("failed to initialize Typst engine");
        Self {
            vault,
            selected: None,
            editor: HelixEditor::new(),
            mixed: MixedEditor::new(),
            engine,
            new_note_name: String::new(),
            search_query: String::new(),
            backlinks,
            sidebar_open: true,
            watcher,
            git: GitSync::default(),
            status: String::new(),
        }
    }

    fn open_note(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.editor.dirty() {
            let _ = self.save_current();
        }
        match self.vault.read_note(&path) {
            Ok(text) => {
                self.mixed.load(&text);
                self.editor.set_text(text);
                self.editor.request_focus(ctx);
                self.selected = Some(path);
                self.status = "opened".into();
            }
            Err(e) => {
                self.status = format!("open failed: {e}");
            }
        }
    }

    fn save_current(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        if self.mixed.dirty {
            self.editor.replace_text(self.mixed.source());
            self.mixed.dirty = false;
        }
        match self.vault.write_note(&path, self.editor.text()) {
            Ok(()) => {
                self.editor.clear_dirty();
                self.backlinks = search::build_backlinks(&self.vault);
                self.status = "saved".into();
                true
            }
            Err(e) => {
                self.status = format!("save failed: {e}");
                false
            }
        }
    }

    fn reload_current(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.selected.clone() {
            if let Ok(text) = self.vault.read_note(&path) {
                self.mixed.load(&text);
                self.editor.set_text(text);
                self.editor.request_focus(ctx);
                self.status = "reloaded".into();
            }
        }
    }

    fn create_note(&mut self, ctx: &egui::Context, name: String) {
        match self.vault.create_note(&name) {
            Ok(path) => {
                self.open_note(ctx, path);
                self.status = "created".into();
            }
            Err(e) => self.status = format!("create failed: {e}"),
        }
    }

    fn perform(&mut self, ctx: &egui::Context, action: AppAction) {
        match action {
            AppAction::OpenNote(p) => self.open_note(ctx, p),
            AppAction::CreateNote(name) => self.create_note(ctx, name),
            AppAction::SaveCurrent => {
                self.save_current();
            }
            AppAction::ReloadCurrent => self.reload_current(ctx),
            AppAction::OpenInHelix => self.open_in_helix(),
        }
    }

    fn open_in_helix(&mut self) {
        let Some(path) = self.selected.clone() else {
            self.status = "no note selected".into();
            return;
        };
        if self.editor.dirty() {
            self.save_current();
        }
        match crate::editor_backend::open_in_helix(&path) {
            Ok(()) => self.status = "opened in Helix".into(),
            Err(e) => self.status = format!("helix failed: {e}"),
        }
    }

    fn poll_watcher(&mut self, ctx: &egui::Context) {
        let Some(watcher) = &self.watcher else {
            return;
        };
        let changes = watcher.drain_changes();
        if changes.is_empty() {
            return;
        }
        self.vault.rescan();
        self.backlinks = search::build_backlinks(&self.vault);

        if let Some(current) = self.selected.clone() {
            if changes.iter().any(|p| p == &current) && !self.editor.dirty() {
                if let Ok(text) = self.vault.read_note(&current) {
                    self.mixed.load(&text);
                    self.editor.set_text(text);
                    self.status = "external change reloaded".into();
                }
            }
        }
        let _ = ctx;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_watcher(ctx);

        let mut actions: Vec<AppAction> = Vec::new();

        let ctrl_e = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::E));
        if ctrl_e && self.selected.is_some() {
            actions.push(AppAction::OpenInHelix);
        }
        let ctrl_s = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S));
        if ctrl_s && self.selected.is_some() {
            actions.push(AppAction::SaveCurrent);
        }

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let icon = if self.sidebar_open { "◀" } else { "▶" };
                if ui.button(icon).clicked() {
                    self.sidebar_open = !self.sidebar_open;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status.is_empty() {
                        ui.label(&self.status);
                    }
                    if self.editor.dirty() || self.mixed.dirty {
                        ui.colored_label(egui::Color32::YELLOW, "● unsaved");
                    }
                });
            });
        });

        egui::SidePanel::left("vault")
            .default_width(240.0)
            .width_range(24.0..=600.0)
            .show_animated(ctx, self.sidebar_open, |ui| {
                if let Some(a) = ui::vault_explorer::show(self, ui) {
                    actions.push(a);
                }
            });

        egui::TopBottomPanel::bottom("backlinks")
            .default_height(140.0)
            .show(ctx, |ui| {
                if let Some(a) = ui::backlinks_panel::show(self, ui) {
                    actions.push(a);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(a) = ui::editor_view::show(self, ctx, ui) {
                actions.push(a);
            }
        });

        for action in actions {
            self.perform(ctx, action);
        }
    }
}
