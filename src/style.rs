//! App-wide font and text-size setup. Loads a system sans-serif into egui
//! and exposes the same family name to the Typst theme template so plain
//! text and rendered blocks share a single font.
//!
//! Sizing accessors ([`ui_pt`], [`editor_pt`], [`content_width_pt`]) read
//! from the user config loaded by [`crate::config`], so overrides in
//! `~/.config/vellum/config.toml` flow through to every consumer.

use serde::{Deserialize, Serialize};

/// Chrome size (topbar, sidebar, buttons, status line), in points.
pub fn ui_pt() -> f32 {
    crate::config::current().ui_pt
}

/// Mixed-editor body size, in points. Threaded into the Typst theme
/// template (via `template.with(size: …)`) and applied to the editor's
/// `TextEdit`s — single source of truth for plain and rendered blocks.
pub fn editor_pt() -> f32 {
    crate::config::current().editor_pt
}

/// Width of the editor content column, in points. Threaded into the
/// Typst theme template (via `template.with(width: …)`) and enforced on
/// the surrounding egui layout so plain paragraphs share the column.
pub fn content_width_pt() -> f32 {
    crate::config::current().content_width_pt
}

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

/// CJK fallback families. Every match is appended to both
/// `Proportional` and `Monospace` so egui can resolve CJK glyphs the
/// primary sans / `Hack` monospace lack. Ordered roughly by coverage
/// and ubiquity across Linux / macOS / Windows.
pub const CJK_FAMILIES: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans CJK TC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans SC",
    "Noto Sans TC",
    "Noto Sans JP",
    "Noto Sans KR",
    "Source Han Sans SC",
    "Source Han Sans TC",
    "Source Han Sans",
    "PingFang SC",
    "PingFang TC",
    "Hiragino Sans",
    "Microsoft YaHei",
    "Microsoft JhengHei",
    "SimSun",
    "WenQuanYi Micro Hei",
    "WenQuanYi Zen Hei",
];

/// Accent stroke used to mark the segment that is currently being edited.
pub const EDIT_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(0x4a, 0x9e, 0xff);

/// Draw a soft accent outline around a widget — the visual cue that
/// the surrounded segment is in source-edit mode.
pub fn paint_edit_outline(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(
        rect.expand(3.0),
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.5, EDIT_OUTLINE_COLOR),
        egui::StrokeKind::Outside,
    );
}

/// Tunables for the source `TextEdit` shown when a segment is in
/// edit mode. `MixedEditor::config` exposes one instance; modify
/// fields after `MixedEditor::new()` to retheme or resize.
#[derive(Clone, Debug)]
pub struct EditorConfig {
    /// Font size used both for layout and the caret height.
    pub font_size: f32,
    /// Font family for the source view (typically `Monospace`).
    pub font_family: egui::FontFamily,
    /// Extra space added between lines, in points. The resulting row
    /// distance (baseline-to-baseline) is `font_size + line_space`.
    /// `None` leaves egui's natural row height untouched.
    ///
    /// **Caret trade-off**: in egui 0.27 the caret height tracks the
    /// row span, so any `Some(_)` value that pushes the row above the
    /// font's natural height also produces a taller caret. The
    /// default is `None` for a font-sized caret. Opt in to wider
    /// spacing — e.g. `Some(0.65 × font_size)` to approximate Typst's
    /// default `par.leading` — when you prefer the spacing match.
    pub line_space: Option<f32>,
    /// Per-token-kind colours applied by the syntax highlighter.
    pub colors: SyntaxColors,
}

impl Default for EditorConfig {
    fn default() -> Self {
        let size = editor_pt();
        Self {
            font_size: size,
            font_family: egui::FontFamily::Monospace,
            // 0.65 × font_size mirrors Typst's default `par.leading`.
            // The caret stays at `font_size` regardless — `MixedEditor`
            // suppresses egui's full-row caret and paints its own.
            line_space: Some(size * 0.65),
            colors: crate::config::current().colors.clone(),
        }
    }
}

/// Foreground colours for individual Typst syntax-tree leaf kinds.
/// `editor::highlight` walks the parser's tree and looks each leaf up
/// here; anything not specifically matched falls back to `default`.
///
/// Serialised as hex strings (`"#rrggbb"`) so the on-disk config file
/// stays human-readable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SyntaxColors {
    #[serde(with = "color_hex")]
    pub default: egui::Color32,
    /// `$` math delimiters.
    #[serde(with = "color_hex")]
    pub dollar: egui::Color32,
    /// `#` markup-to-code transition.
    #[serde(with = "color_hex")]
    pub hash: egui::Color32,
    /// `=` / `==` / … heading markers.
    #[serde(with = "color_hex")]
    pub heading_marker: egui::Color32,
    /// `// …` and `/* … */`.
    #[serde(with = "color_hex")]
    pub comment: egui::Color32,
    /// `"…"` string literals (in code mode).
    #[serde(with = "color_hex")]
    pub string: egui::Color32,
    /// Numeric literals (int / float / numeric / bool).
    #[serde(with = "color_hex")]
    pub number: egui::Color32,
    /// Keywords: `let`, `set`, `show`, `if`, `for`, `import`, …
    #[serde(with = "color_hex")]
    pub keyword: egui::Color32,
    /// Identifiers in code / math mode.
    #[serde(with = "color_hex")]
    pub ident: egui::Color32,
    /// Brackets, commas, semicolons.
    #[serde(with = "color_hex")]
    pub punct: egui::Color32,
    /// `*` and `_` markers (markup emphasis).
    #[serde(with = "color_hex")]
    pub emphasis: egui::Color32,
    /// `-` / `+` / `/` list / enum / term markers.
    #[serde(with = "color_hex")]
    pub list_marker: egui::Color32,
}

/// Serde adaptor: `egui::Color32` ⇄ `"#rrggbb"` hex strings.
/// `"rrggbb"` (without the leading `#`) is also accepted on input.
mod color_hex {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(c: &Color32, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color32, D::Error> {
        let raw = String::deserialize(d)?;
        let hex = raw.trim().trim_start_matches('#');
        if hex.len() != 6 {
            return Err(serde::de::Error::custom(format!(
                "expected 6 hex digits, got {:?}",
                raw
            )));
        }
        let parse = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| serde::de::Error::custom(format!("invalid hex {:?}: {}", &hex[i..i + 2], e)))
        };
        Ok(Color32::from_rgb(parse(0)?, parse(2)?, parse(4)?))
    }
}


impl Default for SyntaxColors {
    fn default() -> Self {
        // Dark palette inspired by VS Code Dark+, tweaked so the
        // dollar sign reads as purple per the editor's design.
        Self {
            default:        egui::Color32::from_rgb(0xd4, 0xd4, 0xd4),
            dollar:         egui::Color32::from_rgb(0xc5, 0x86, 0xc0),
            hash:           egui::Color32::from_rgb(0x4e, 0xc9, 0xb0),
            heading_marker: egui::Color32::from_rgb(0xdc, 0xdc, 0xaa),
            comment:        egui::Color32::from_rgb(0x6a, 0x99, 0x55),
            string:         egui::Color32::from_rgb(0xce, 0x91, 0x78),
            number:         egui::Color32::from_rgb(0xb5, 0xce, 0xa8),
            keyword:        egui::Color32::from_rgb(0x56, 0x9c, 0xd6),
            ident:          egui::Color32::from_rgb(0x9c, 0xdc, 0xfe),
            punct:          egui::Color32::from_rgb(0x80, 0x80, 0x80),
            emphasis:       egui::Color32::from_rgb(0xff, 0xd7, 0x00),
            list_marker:    egui::Color32::from_rgb(0xff, 0x8c, 0x42),
        }
    }
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    for &family in SANS_FAMILIES {
        if let Some(key) = load_face(&db, &mut fonts, family) {
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, key);
            break;
        }
    }

    let mut cjk_loaded: Vec<String> = Vec::new();
    for &family in CJK_FAMILIES {
        if let Some(key) = load_face(&db, &mut fonts, family) {
            cjk_loaded.push(key);
        }
    }
    if !cjk_loaded.is_empty() {
        let prop = fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default();
        for key in &cjk_loaded {
            prop.push(key.clone());
        }
        let mono = fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default();
        for key in &cjk_loaded {
            mono.push(key.clone());
        }
        log::debug!("egui CJK fallbacks loaded: {:?}", cjk_loaded);
    }

    ctx.set_fonts(fonts);

    use egui::{FontFamily, FontId, TextStyle};
    let ui = ui_pt();
    let mut style = (*ctx.global_style()).clone();
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(ui * 1.4, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(ui, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(ui, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(ui, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new((ui - 2.0).max(10.0), FontFamily::Proportional),
    );
    ctx.set_global_style(style);
}

fn load_face(
    db: &fontdb::Database,
    fonts: &mut egui::FontDefinitions,
    family: &str,
) -> Option<String> {
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(family)],
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };
    let id = db.query(&query)?;
    let face = db.face(id)?;
    let key = format!("font-{family}");
    if fonts.font_data.contains_key(&key) {
        return Some(key);
    }
    let data = match &face.source {
        fontdb::Source::File(path) => std::fs::read(path).ok()?,
        fontdb::Source::Binary(bytes) | fontdb::Source::SharedFile(_, bytes) => {
            bytes.as_ref().as_ref().to_vec()
        }
    };
    let mut font_data = egui::FontData::from_owned(data);
    font_data.index = face.index;
    fonts.font_data.insert(key.clone(), font_data.into());
    Some(key)
}
