use crate::app::{App, AppAction};

pub fn show(app: &App, ui: &mut egui::Ui) -> Option<AppAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading("Preview");
        if app.preview.pages.len() > 1 {
            ui.label(format!("{} pages", app.preview.pages.len()));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = app.preview.pdf_path.is_some();
            if ui
                .add_enabled(enabled, egui::Button::new("Open PDF"))
                .clicked()
            {
                action = Some(AppAction::OpenPdfExternally);
            }
            if ui
                .add_enabled(
                    app.selected.is_some(),
                    egui::Button::new("Open in Helix (Ctrl+E)"),
                )
                .clicked()
            {
                action = Some(AppAction::OpenInHelix);
            }
        });
    });
    ui.separator();

    if let Some(err) = &app.preview.error {
        ui.colored_label(egui::Color32::LIGHT_RED, "compile failed");
        egui::ScrollArea::vertical()
            .id_source("preview-error-scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(err).monospace().small())
                        .wrap(true),
                );
            });
        ui.separator();
        ui.label("Source:");
        egui::ScrollArea::vertical()
            .id_source("preview-fallback-scroll")
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&app.preview.fallback_text).monospace(),
                    )
                    .wrap(true),
                );
            });
        return action;
    }

    if app.preview.pages.is_empty() {
        ui.label(if app.preview.source_path.is_some() {
            "Compiling…"
        } else {
            "No note loaded."
        });
        return action;
    }

    let avail_w = ui.available_width().max(1.0);
    egui::ScrollArea::both()
        .id_source("preview-scroll")
        .show(ui, |ui| {
            for page in &app.preview.pages {
                let [w, h] = page.size;
                let scale = (avail_w / w as f32).min(1.0);
                let display_size = egui::vec2(w as f32 * scale, h as f32 * scale);
                ui.add(egui::Image::new(&page.texture).fit_to_exact_size(display_size));
                ui.add_space(8.0);
            }
        });

    action
}
