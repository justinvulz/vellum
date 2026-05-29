//! App-wide font and text-size setup. Loads a system sans-serif into egui
//! and exposes the same family name to the Typst theme template so plain
//! text and rendered blocks share a single font.
//!
//! Sizing accessors ([`ui_pt`], [`editor_pt`], [`content_width_pt`]) read
//! from the user config loaded by [`crate::config`], so overrides in
//! `~/.config/vellum/config.toml` flow through to every consumer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

/// Single accent colour reused across the chrome: edit outline, selection
/// highlights, hyperlink text, dirty-buffer marker, focus rings. Reads
/// from [`crate::config::current().ui_colors`] so user overrides flow
/// through to every consumer.
pub fn accent() -> egui::Color32 {
    crate::config::current().ui_colors.accent
}

/// Draw a soft accent outline around a widget — the visual cue that
/// the surrounded segment is in source-edit mode.
pub fn paint_edit_outline(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(
        rect.expand(3.0),
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.5, accent()),
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
///
/// Deliberately **not** `#[serde(default)]` — serde's struct-level
/// default would call [`SyntaxColors::default`] eagerly at the start of
/// every deserialization. Since `SyntaxColors::default()` parses the
/// bundled `default_config.toml` (via [`crate::config::defaults`]) and
/// that parse itself deserializes a `SyntaxColors`, the eager call
/// would re-enter the bundled-config `OnceLock` and deadlock. Partial
/// `[colors]` tables in a user config are handled by merging on
/// `toml::Value` *before* deserializing into [`Config`].
#[derive(Clone, Debug, Serialize, Deserialize)]
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
        crate::config::defaults().colors.clone()
    }
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_visuals(ctx);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // Cache of font-file bytes by path. Many CJK fallbacks (Noto Sans
    // CJK SC/TC/JP/KR) all live in the same .ttc collection; without
    // this each call would `std::fs::read` the whole 80 MB file again.
    // Entries are `Box::leak`'d so we can hand `&'static [u8]` to
    // `FontData::from_static`, which lets every face share the bytes
    // instead of egui copying them via `FontData::from_owned`.
    let mut file_bytes: HashMap<PathBuf, &'static [u8]> = HashMap::new();

    let cfg = crate::config::current();
    for family in &cfg.sans_families {
        if let Some(key) = load_face(&db, &mut fonts, &mut file_bytes, family) {
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, key);
            break;
        }
    }

    // Phosphor icon font as a fallback for Proportional. Glyphs live
    // in the Unicode private-use area, so it never shadows the sans
    // family above for regular text.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let mut cjk_loaded: Vec<String> = Vec::new();
    for family in &cfg.cjk_families {
        if let Some(key) = load_face(&db, &mut fonts, &mut file_bytes, family) {
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

/// Apply the app's custom palette, corner radii, shadows, and spacing.
/// Pulls the colour set from [`crate::config::current().ui_colors`] so
/// the whole UI reskins from a single config table.
fn install_visuals(ctx: &egui::Context) {
    use egui::epaint::Shadow;
    use egui::{Color32, CornerRadius, Margin, Stroke};

    let c = &crate::config::current().ui_colors;

    let cr = CornerRadius::same(6);

    let mut v = egui::Visuals::dark();
    v.panel_fill = c.panel;
    v.window_fill = c.elevated;
    v.window_stroke = Stroke::new(1.0, c.line);
    v.window_corner_radius = cr;
    v.menu_corner_radius = cr;
    v.extreme_bg_color = c.bg;
    v.faint_bg_color = c.elevated;
    v.code_bg_color = c.elevated;
    v.weak_text_color = Some(c.text_dim);
    v.hyperlink_color = c.accent;

    v.widgets.noninteractive.bg_fill = c.panel;
    v.widgets.noninteractive.weak_bg_fill = c.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.line);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
    v.widgets.noninteractive.corner_radius = cr;

    v.widgets.inactive.bg_fill = c.elevated;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, c.text);
    v.widgets.inactive.corner_radius = cr;

    v.widgets.hovered.bg_fill = c.hovered;
    v.widgets.hovered.weak_bg_fill = c.hovered;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.line_strong);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, c.text_strong);
    v.widgets.hovered.corner_radius = cr;
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = c.active;
    v.widgets.active.weak_bg_fill = c.active;
    v.widgets.active.bg_stroke = Stroke::new(1.0, c.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, c.text_strong);
    v.widgets.active.corner_radius = cr;
    v.widgets.active.expansion = 0.0;

    v.widgets.open.bg_fill = c.active;
    v.widgets.open.weak_bg_fill = c.active;
    v.widgets.open.bg_stroke = Stroke::new(1.0, c.line_strong);
    v.widgets.open.fg_stroke = Stroke::new(1.0, c.text_strong);
    v.widgets.open.corner_radius = cr;

    v.selection.bg_fill =
        Color32::from_rgba_unmultiplied(c.accent.r(), c.accent.g(), c.accent.b(), 64);
    v.selection.stroke = Stroke::new(1.0, c.accent);

    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(96),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(72),
    };

    ctx.global_style_mut(|style| {
        style.visuals = v.clone();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.window_margin = Margin::same(10);
        style.spacing.interact_size.y = 22.0;
    });
}

/// Chrome palette — the colour set driving [`install_visuals`]. Round-
/// trips through hex strings via the [`color_hex`] adaptor so the
/// on-disk config stays human-readable.
///
/// The default values are the ColorHunt palette `281c594e8d9c85c79aedf7bd`:
/// deep indigo panel, teal hover/active, soft mint accent, pale lemon
/// text.
///
/// See [`SyntaxColors`] for why `#[serde(default)]` is omitted here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiColors {
    /// Extreme background (text-edit fields, scroll-bar troughs,
    /// anything that needs to read as the "lowest" surface).
    #[serde(with = "color_hex")]
    pub bg: egui::Color32,
    /// Main panel surface — fills the sidebar, topbar, central area.
    #[serde(with = "color_hex")]
    pub panel: egui::Color32,
    /// Elevated surface for windows, popups, menus, and the rest state
    /// of buttons that need a visible fill.
    #[serde(with = "color_hex")]
    pub elevated: egui::Color32,
    /// Background of buttons / tree rows on hover.
    #[serde(with = "color_hex")]
    pub hovered: egui::Color32,
    /// Background of buttons while being clicked.
    #[serde(with = "color_hex")]
    pub active: egui::Color32,
    /// Subtle stroke colour — dividers, frame outlines at rest.
    #[serde(with = "color_hex")]
    pub line: egui::Color32,
    /// Stronger stroke colour for hovered widgets.
    #[serde(with = "color_hex")]
    pub line_strong: egui::Color32,
    /// Highlight colour reused for focus rings, selection strokes,
    /// hyperlinks, the dirty-buffer marker, and the edit outline.
    #[serde(with = "color_hex")]
    pub accent: egui::Color32,
    /// Primary body text colour.
    #[serde(with = "color_hex")]
    pub text: egui::Color32,
    /// Brighter text for hovered/active widgets.
    #[serde(with = "color_hex")]
    pub text_strong: egui::Color32,
    /// Dimmed text for secondary / weak labels.
    #[serde(with = "color_hex")]
    pub text_dim: egui::Color32,
}

impl Default for UiColors {
    fn default() -> Self {
        crate::config::defaults().ui_colors.clone()
    }
}

/// Soft horizontal divider — same colour as the (tuned) default
/// [`egui::Separator`] but with extra vertical breathing room so
/// adjacent sections feel like distinct blocks rather than cramped
/// rows.
pub fn soft_separator(ui: &mut egui::Ui) {
    ui.add(egui::Separator::default().spacing(12.0));
}

fn load_face(
    db: &fontdb::Database,
    fonts: &mut egui::FontDefinitions,
    file_bytes: &mut HashMap<PathBuf, &'static [u8]>,
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
    let bytes: &'static [u8] = match &face.source {
        fontdb::Source::File(path) => {
            if let Some(&b) = file_bytes.get(path) {
                b
            } else {
                let data = std::fs::read(path).ok()?;
                let leaked: &'static [u8] = Box::leak(data.into_boxed_slice());
                file_bytes.insert(path.clone(), leaked);
                leaked
            }
        }
        fontdb::Source::Binary(bytes) | fontdb::Source::SharedFile(_, bytes) => {
            let v: Vec<u8> = bytes.as_ref().as_ref().to_vec();
            Box::leak(v.into_boxed_slice())
        }
    };
    let mut font_data = egui::FontData::from_static(bytes);
    font_data.index = face.index;
    fonts.font_data.insert(key.clone(), font_data.into());
    Some(key)
}
