use crate::editor_backend::FileWatcher;
use crate::git::GitSync;
use crate::helix_editor::HelixEditor;
use crate::search::{self, BacklinkIndex};
use crate::typst_compiler::{self, PreviewState};
use crate::ui;
use crate::vault::Vault;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Split,
    Tab,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabFocus {
    Editor,
    Preview,
}

pub enum AppAction {
    OpenNote(PathBuf),
    CreateNote(String),
    SaveCurrent,
    ReloadCurrent,
    OpenPdfExternally,
    OpenInHelix,
}

pub struct App {
    pub vault: Vault,
    pub selected: Option<PathBuf>,
    pub editor: HelixEditor,
    pub new_note_name: String,
    pub search_query: String,
    pub preview: PreviewState,
    pub backlinks: BacklinkIndex,
    pub layout: LayoutMode,
    pub tab_focus: TabFocus,
    pub watcher: Option<FileWatcher>,
    pub git: GitSync,
    pub status: String,
}

impl App {
    pub fn new(vault: Vault) -> Self {
        let watcher = FileWatcher::new(&vault.root).ok();
        let backlinks = search::build_backlinks(&vault);
        Self {
            vault,
            selected: None,
            editor: HelixEditor::new(),
            new_note_name: String::new(),
            search_query: String::new(),
            preview: PreviewState::new(),
            backlinks,
            layout: LayoutMode::Split,
            tab_focus: TabFocus::Editor,
            watcher,
            git: GitSync::default(),
            status: String::new(),
        }
    }

    pub fn dirty(&self) -> bool {
        self.editor.dirty()
    }

    fn open_note(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.editor.dirty() {
            let _ = self.save_current(ctx);
        }
        match self.vault.read_note(&path) {
            Ok(text) => {
                self.editor.set_text(text);
                self.editor.request_focus(ctx);
                self.selected = Some(path.clone());
                self.preview.compile(ctx, &self.vault.root, &path);
                self.status = "opened".into();
            }
            Err(e) => {
                self.status = format!("open failed: {e}");
            }
        }
    }

    fn save_current(&mut self, ctx: &egui::Context) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        match self.vault.write_note(&path, self.editor.text()) {
            Ok(()) => {
                self.editor.clear_dirty();
                self.preview.compile(ctx, &self.vault.root, &path);
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
                self.editor.set_text(text);
                self.editor.request_focus(ctx);
                self.preview.compile(ctx, &self.vault.root, &path);
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
                self.save_current(ctx);
            }
            AppAction::ReloadCurrent => self.reload_current(ctx),
            AppAction::OpenPdfExternally => self.open_pdf_externally(),
            AppAction::OpenInHelix => self.open_in_helix(ctx),
        }
    }

    fn open_in_helix(&mut self, ctx: &egui::Context) {
        let Some(path) = self.selected.clone() else {
            self.status = "no note selected".into();
            return;
        };
        if self.editor.dirty() {
            self.save_current(ctx);
        }
        match crate::editor_backend::open_in_helix(&path) {
            Ok(()) => self.status = "opened in Helix".into(),
            Err(e) => self.status = format!("helix failed: {e}"),
        }
    }

    fn open_pdf_externally(&mut self) {
        let Some(pdf) = self.preview.pdf_path.clone() else {
            self.status = "no PDF yet".into();
            return;
        };
        match typst_compiler::open_externally(&pdf) {
            Ok(()) => self.status = "opened PDF".into(),
            Err(e) => self.status = format!("open PDF failed: {e}"),
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
                    self.editor.set_text(text);
                    self.preview.compile(ctx, &self.vault.root, &current);
                    self.status = "external change reloaded".into();
                }
            }
        }
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

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.layout, LayoutMode::Split, "Split");
                ui.selectable_value(&mut self.layout, LayoutMode::Tab, "Tabs");
                ui.separator();
                if self.layout == LayoutMode::Tab {
                    ui.selectable_value(&mut self.tab_focus, TabFocus::Editor, "Editor");
                    ui.selectable_value(&mut self.tab_focus, TabFocus::Preview, "Preview");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status.is_empty() {
                        ui.label(&self.status);
                    }
                    if self.editor.dirty() {
                        ui.colored_label(egui::Color32::YELLOW, "● unsaved");
                    }
                });
            });
        });

        egui::SidePanel::left("vault")
            .default_width(240.0)
            .show(ctx, |ui| {
                if let Some(a) = ui::vault_explorer::show(self, ui) {
                    actions.push(a);
                }
            });

        match self.layout {
            LayoutMode::Split => {
                egui::SidePanel::right("preview")
                    .default_width(400.0)
                    .show(ctx, |ui| {
                        if let Some(a) = ui::preview_pane::show(self, ui) {
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
            }
            LayoutMode::Tab => {
                egui::CentralPanel::default().show(ctx, |ui| match self.tab_focus {
                    TabFocus::Editor => {
                        if let Some(a) = ui::editor_view::show(self, ctx, ui) {
                            actions.push(a);
                        }
                    }
                    TabFocus::Preview => {
                        if let Some(a) = ui::preview_pane::show(self, ui) {
                            actions.push(a);
                        }
                    }
                });
            }
        }

        for action in actions {
            self.perform(ctx, action);
        }
    }
}
