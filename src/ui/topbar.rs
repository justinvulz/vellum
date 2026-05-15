//! Top bar: sidebar toggle on the left, dirty marker and status on the right.

use crate::app::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let icon = if app.sidebar_open { "◀" } else { "▶" };
        if ui.button(icon).clicked() {
            app.sidebar_open = !app.sidebar_open;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !app.status.is_empty() {
                ui.label(&app.status);
            }
            if app.mixed.dirty {
                ui.colored_label(egui::Color32::YELLOW, "● unsaved");
            }
        });
    });
}
