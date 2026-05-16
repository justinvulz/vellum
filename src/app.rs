//! Top-level application state and the eframe `update` loop.
//!
//! `App` owns the vault, editor, file watcher, and search caches.
//! UI panels and keyboard shortcuts hand back `AppAction` values
//! rather than mutating `App` directly — `perform` is the single
//! place that turns intent into state changes.

use crate::editor::mixed::MixedEditor;
use crate::editor::typst_engine::TypstEngine;
use crate::file_watcher::FileWatcher;
use crate::search::{self, BacklinkIndex};
use crate::ui;
use crate::vault::Vault;
use std::path::PathBuf;

pub enum AppAction {
    OpenNote(PathBuf),
    /// Open a note identified by its filename stem
    /// (e.g. `#line-note("foo")` clicks emit `OpenNoteByName("foo")`).
    OpenNoteByName(String),
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
        log::info!("note: open {}", self.vault.display_name(&path));
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
                log::warn!("note: open failed: {e}");
                self.status = format!("open failed: {e}");
            }
        }
    }

    fn save_current(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        log::info!("note: save {}", self.vault.display_name(&path));
        match self.vault.write_note(&path, &self.mixed.source()) {
            Ok(()) => {
                self.mixed.dirty = false;
                self.backlinks = search::build_backlinks(&self.vault);
                self.status = "saved".into();
                true
            }
            Err(e) => {
                log::warn!("note: save failed: {e}");
                self.status = format!("save failed: {e}");
                false
            }
        }
    }

    fn reload_current(&mut self) {
        let Some(path) = self.selected.clone() else { return };
        log::info!("note: reload {}", self.vault.display_name(&path));
        if let Ok(text) = self.vault.read_note(&path) {
            self.mixed.load(&text);
            self.status = "reloaded".into();
        }
    }

    fn create_note(&mut self, name: String) {
        log::info!("note: create {}", name);
        match self.vault.create_note(&name) {
            Ok(path) => {
                self.open_note(path);
                self.status = "created".into();
            }
            Err(e) => {
                log::warn!("note: create failed: {e}");
                self.status = format!("create failed: {e}");
            }
        }
    }

    fn open_note_by_name(&mut self, name: String) {
        log::info!("note: follow link to {}", name);
        match search::find_note_by_stem(&self.vault, &name) {
            Some(path) => self.open_note(path),
            None => {
                log::warn!("note: link target not found: {}", name);
                self.status = format!("note not found: {name}");
            }
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
            Err(e) => {
                log::warn!("note: helix failed: {e}");
                self.status = format!("helix failed: {e}");
            }
        }
    }

    fn perform(&mut self, action: AppAction) {
        match action {
            AppAction::OpenNote(p) => self.open_note(p),
            AppAction::OpenNoteByName(name) => self.open_note_by_name(name),
            AppAction::CreateNote(name) => self.create_note(name),
            AppAction::SaveCurrent => {
                self.save_current();
            }
            AppAction::ReloadCurrent => self.reload_current(),
            AppAction::OpenInHelix => self.open_in_helix(),
        }
    }

    /// Drain file-watcher events, rescan the vault, and (when the
    /// current note changed externally and the buffer is clean)
    /// reload it.
    fn poll_watcher(&mut self) {
        let Some(watcher) = &self.watcher else { return };
        let changes = watcher.drain_changes();
        if changes.is_empty() {
            return;
        }
        self.vault.rescan();
        self.backlinks = search::build_backlinks(&self.vault);

        if let Some(current) = self.selected.clone() {
            if changes.iter().any(|p| p == &current) && !self.mixed.dirty {
                if let Ok(text) = self.vault.read_note(&current) {
                    log::info!(
                        "note: external reload {}",
                        self.vault.display_name(&current)
                    );
                    self.mixed.load(&text);
                    self.status = "external change reloaded".into();
                }
            }
        }
    }

    /// Translate the frame's keyboard input into pending actions.
    /// Shortcuts that need a selected note are suppressed when none is open.
    fn shortcut_actions(&self, ctx: &egui::Context) -> Vec<AppAction> {
        if self.selected.is_none() {
            return Vec::new();
        }
        let (ctrl_e, ctrl_s) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.key_pressed(egui::Key::E),
                i.modifiers.ctrl && i.key_pressed(egui::Key::S),
            )
        });
        let mut actions = Vec::new();
        if ctrl_e {
            actions.push(AppAction::OpenInHelix);
        }
        if ctrl_s {
            actions.push(AppAction::SaveCurrent);
        }
        actions
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_watcher();

        let mut actions = self.shortcut_actions(ctx);

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui::topbar::show(self, ui);
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
