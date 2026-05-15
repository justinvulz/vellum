//! App-wide font and text-size setup. Loads a system sans-serif into egui
//! and exposes the same family name to the Typst theme template so plain
//! text and rendered blocks share a single font.

/// Chrome size (topbar, sidebar, buttons, status line), in points.
pub const UI_PT: f32 = 14.0;

/// Mixed-editor body size, in points. Threaded into the Typst theme
/// template (via `template.with(size: …)`) and applied to the editor's
/// `TextEdit`s — single source of truth for plain and rendered blocks.
pub const EDITOR_PT: f32 = 20.0;

/// Width of the editor content column, in points. Threaded into the
/// Typst theme template (via `template.with(width: …)`) and enforced on
/// the surrounding egui layout so plain paragraphs share the column.
pub const CONTENT_WIDTH_PT: f32 = 800.0;

/// Sans-serif families to try, in priority order. The Typst theme uses
/// the same list, so whichever family the host system provides is the
/// one both renderers pick up.
pub const SANS_FAMILIES: &[&str] = &[
    "Inter",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
    "Ubuntu",
    "Helvetica",
    "Arial",
];

/// Accent stroke used to mark the segment that is currently being edited.
pub const EDIT_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(0x4a, 0x9e, 0xff);

/// Draw a soft accent outline around a widget — the visual cue that
/// the surrounded segment is in source-edit mode.
pub fn paint_edit_outline(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(
        rect.expand(3.0),
        egui::Rounding::same(4.0),
        egui::Stroke::new(1.5, EDIT_OUTLINE_COLOR),
    );
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    for &family in SANS_FAMILIES {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        let Some(id) = db.query(&query) else { continue };
        let Some(face) = db.face(id) else { continue };
        let data = match &face.source {
            fontdb::Source::File(path) => std::fs::read(path).ok(),
            fontdb::Source::Binary(bytes) | fontdb::Source::SharedFile(_, bytes) => {
                Some(bytes.as_ref().as_ref().to_vec())
            }
        };
        let Some(data) = data else { continue };

        let key = format!("sans-{family}");
        fonts
            .font_data
            .insert(key.clone(), egui::FontData::from_owned(data));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, key);
        break;
    }

    ctx.set_fonts(fonts);

    use egui::{FontFamily, FontId, TextStyle};
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(UI_PT * 1.4, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(UI_PT, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(UI_PT, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(UI_PT, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new((UI_PT - 2.0).max(10.0), FontFamily::Proportional),
    );
    ctx.set_style(style);
}
