use crate::editor::mixed::MixedEditor;
use crate::editor::typst_engine::TypstEngine;
use crate::file_watcher::FileWatcher;
use crate::search::{self, BacklinkIndex};
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
    pub mixed: MixedEditor,
    pub engine: TypstEngine,
    pub new_note_name: String,
    pub search_query: String,
    pub backlinks: BacklinkIndex,
    pub sidebar_open: bool,
    pub watcher: Option<FileWatcher>,
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
            mixed: MixedEditor::new(),
            engine,
            new_note_name: String::new(),
            search_query: String::new(),
            backlinks,
            sidebar_open: true,
            watcher,
            status: String::new(),
        }
    }

    fn open_note(&mut self, path: PathBuf) {
        if self.mixed.dirty {
            let _ = self.save_current();
        }
        match self.vault.read_note(&path) {
            Ok(text) => {
                self.mixed.load(&text);
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
        match self.vault.write_note(&path, &self.mixed.source()) {
            Ok(()) => {
                self.mixed.dirty = false;
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

    fn reload_current(&mut self) {
        if let Some(path) = self.selected.clone() {
            if let Ok(text) = self.vault.read_note(&path) {
                self.mixed.load(&text);
                self.status = "reloaded".into();
            }
        }
    }

    fn create_note(&mut self, name: String) {
        match self.vault.create_note(&name) {
            Ok(path) => {
                self.open_note(path);
                self.status = "created".into();
            }
            Err(e) => self.status = format!("create failed: {e}"),
        }
    }

    fn open_in_helix(&mut self) {
        let Some(path) = self.selected.clone() else {
            self.status = "no note selected".into();
            return;
        };
        if self.mixed.dirty {
            self.save_current();
        }
        match crate::external_editor::open_in_helix(&path) {
            Ok(()) => self.status = "opened in Helix".into(),
            Err(e) => self.status = format!("helix failed: {e}"),
        }
    }

    fn perform(&mut self, action: AppAction) {
        match action {
            AppAction::OpenNote(p) => self.open_note(p),
            AppAction::CreateNote(name) => self.create_note(name),
            AppAction::SaveCurrent => {
                self.save_current();
            }
            AppAction::ReloadCurrent => self.reload_current(),
            AppAction::OpenInHelix => self.open_in_helix(),
        }
    }

    fn poll_watcher(&mut self) {
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
            if changes.iter().any(|p| p == &current) && !self.mixed.dirty {
                if let Ok(text) = self.vault.read_note(&current) {
                    self.mixed.load(&text);
                    self.status = "external change reloaded".into();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_watcher();

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
                    if self.mixed.dirty {
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
            self.perform(action);
        }
    }
}
