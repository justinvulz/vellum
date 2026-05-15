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
        Self {
            font_size: EDITOR_PT,
            font_family: egui::FontFamily::Monospace,
            // 0.65 × font_size mirrors Typst's default `par.leading`.
            // The caret stays at `font_size` regardless — `MixedEditor`
            // suppresses egui's full-row caret and paints its own.
            line_space: Some(EDITOR_PT * 0.65),
            colors: SyntaxColors::default(),
        }
    }
}

/// Foreground colours for individual Typst syntax-tree leaf kinds.
/// `editor::highlight` walks the parser's tree and looks each leaf up
/// here; anything not specifically matched falls back to `default`.
#[derive(Clone, Debug)]
pub struct SyntaxColors {
    pub default: egui::Color32,
    /// `$` math delimiters.
    pub dollar: egui::Color32,
    /// `#` markup-to-code transition.
    pub hash: egui::Color32,
    /// `=` / `==` / … heading markers.
    pub heading_marker: egui::Color32,
    /// `// …` and `/* … */`.
    pub comment: egui::Color32,
    /// `"…"` string literals (in code mode).
    pub string: egui::Color32,
    /// Numeric literals (int / float / numeric / bool).
    pub number: egui::Color32,
    /// Keywords: `let`, `set`, `show`, `if`, `for`, `import`, …
    pub keyword: egui::Color32,
    /// Identifiers in code / math mode.
    pub ident: egui::Color32,
    /// Brackets, commas, semicolons.
    pub punct: egui::Color32,
    /// `*` and `_` markers (markup emphasis).
    pub emphasis: egui::Color32,
    /// `-` / `+` / `/` list / enum / term markers.
    pub list_marker: egui::Color32,
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
