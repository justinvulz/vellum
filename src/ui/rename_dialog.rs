//! Modal asking for the new stem when the user invokes `Rename` from
//! a note's context menu. Emits one of `RenameNote` / `CancelRename`.

use crate::app::{App, AppAction};
use egui_phosphor::regular as icon;

pub fn show(app: &mut App, ctx: &egui::Context) -> Option<AppAction> {
    let Some(dialog) = app.rename.as_mut() else {
        return None;
    };

    let mut action: Option<AppAction> = None;
    let from = dialog.from.clone();
    let display = app.vault.display_name(&from);

    let mut open = true;
    egui::Window::new("Rename note")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Renaming: {display}"));
            ui.add_space(4.0);

            let resp = ui.add(
                egui::TextEdit::singleline(&mut dialog.input)
                    .hint_text("new name")
                    .desired_width(240.0),
            );
            resp.request_focus();

            let submit = resp.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button(format!("{}  Rename", icon::PENCIL_SIMPLE))
                    .clicked()
                    || submit
                {
                    let new_stem = dialog.input.trim().to_string();
                    if !new_stem.is_empty() {
                        action = Some(AppAction::RenameNote {
                            from: from.clone(),
                            new_stem,
                        });
                    }
                }
                if ui.button("Cancel").clicked() || cancel {
                    action = Some(AppAction::CancelRename);
                }
            });
        });

    // Close button on the Window frame.
    if !open && action.is_none() {
        action = Some(AppAction::CancelRename);
    }
    action
}
