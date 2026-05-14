//! App-wide font and text-size setup. Loads New Computer Modern from
//! `typst-assets` into egui so plain text matches Typst-rendered text in
//! both family and size.

use typst::foundations::Bytes;
use typst::text::{Font, FontStyle, FontWeight};

/// Body text size, in points. Used for both the egui default and the
/// Typst theme template — keep them in sync.
pub const BODY_PT: f32 = 16.0;

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for data in typst_assets::fonts() {
        let bytes = Bytes::new(data.to_vec());
        let Some(font) = Font::new(bytes, 0) else {
            continue;
        };
        let info = font.info();
        if info.family == "New Computer Modern"
            && info.variant.style == FontStyle::Normal
            && info.variant.weight == FontWeight::REGULAR
        {
            let key = "ncm".to_string();
            fonts
                .font_data
                .insert(key.clone(), egui::FontData::from_static(data));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, key);
            break;
        }
    }
    ctx.set_fonts(fonts);

    use egui::{FontFamily, FontId, TextStyle};
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(BODY_PT * 1.4, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(BODY_PT, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(BODY_PT, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(BODY_PT, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(BODY_PT - 3.0, FontFamily::Proportional),
    );
    ctx.set_style(style);
}
