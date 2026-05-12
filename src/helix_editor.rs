/// Plain in-app text buffer. Real editing happens in Helix (Ctrl+E).
pub struct HelixEditor {
    text: String,
    dirty: bool,
    id: egui::Id,
    focus_requested: bool,
}

#[derive(Default)]
pub struct EditorOutput {
    pub save: bool,
}

impl Default for HelixEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            dirty: false,
            id: egui::Id::new("vellum-editor"),
            focus_requested: false,
        }
    }
}

impl HelixEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.dirty = false;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn request_focus(&mut self, _ctx: &egui::Context) {
        self.focus_requested = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> EditorOutput {
        let mut out = EditorOutput::default();
        let id = self.id;
        let has_focus = ctx.memory(|m| m.has_focus(id));

        let save_pressed =
            ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S));
        if has_focus && save_pressed {
            out.save = true;
        }

        let resp = egui::TextEdit::multiline(&mut self.text)
            .id(id)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(20)
            .show(ui);

        if resp.response.changed() {
            self.dirty = true;
        }

        if self.focus_requested {
            ctx.memory_mut(|m| m.request_focus(id));
            self.focus_requested = false;
        }

        out
    }
}
