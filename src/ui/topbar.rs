//! Top bar: sidebar toggle on the left, dirty marker and status on the right.

use crate::app::App;
use crate::style;
use egui_phosphor::regular as icon;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui
            .add(egui::Button::selectable(app.sidebar_open, icon::SIDEBAR_SIMPLE))
            .on_hover_text("Toggle sidebar")
            .clicked()
        {
            app.sidebar_open = !app.sidebar_open;
        }
        if ui
            .add(egui::Button::selectable(app.backlinks_open, icon::LINK))
            .on_hover_text("Toggle backlinks")
            .clicked()
        {
            app.backlinks_open = !app.backlinks_open;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !app.status.is_empty() {
                ui.label(&app.status);
            }
            if app.mixed.dirty {
                ui.colored_label(style::accent(), format!("{} unsaved", icon::DOT));
            }
        });
    });
}
