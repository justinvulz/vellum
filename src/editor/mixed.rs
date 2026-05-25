//! Mixed inline editor: every segment renders through the Typst engine
//! and flips to a source `TextEdit` on click. Headings, math blocks,
//! function calls, and plain prose all flow through the same pipeline.

use super::highlight;
use super::preamble;
use super::segment;
use super::typst_engine::{RenderedPage, TypstEngine, PIXEL_PER_PT};
use crate::style::{self, CONTENT_WIDTH_PT, EditorConfig};
use std::collections::HashMap;

/// Vertical gap between adjacent segments, in egui points.
const SEGMENT_GAP: f32 = 6.0;

/// Top padding above the first segment, in egui points. Keeps the
/// initial heading off the panel separator so the editor doesn't feel
/// crammed against the top toolbar.
const TOP_PADDING: f32 = 24.0;

/// Maximum wall-clock time (ms) spent compiling Typst segments per
/// frame. Segments beyond the budget show "⟳ rendering…" and compile
/// on the next repaint, keeping the UI responsive while a long note
/// loads progressively.
const FRAME_COMPILE_BUDGET_MS: u64 = 16;

/// Source-edit `TextEdit` font is scaled down from `EditorConfig::font_size`
/// so the monospace glyphs roughly match the visual weight of the
/// rendered proportional Typst output above and below them.
const EDIT_FONT_SCALE: f32 = 0.8;

pub struct MixedEditor {
    pub segments: Vec<String>,
    pub editing_index: Option<usize>,
    pub renders: HashMap<String, RenderedPage>,
    pub failed: HashMap<String, String>,
    pub dirty: bool,
    /// Tunables for the source `TextEdit` shown in edit mode —
    /// font, line height, syntax colours. Mutate to retheme.
    pub config: EditorConfig,
    /// Focus to apply on the next frame, once the matching `TextEdit`
    /// exists. Set when the user clicks a rendered segment.
    pending_focus: Option<egui::Id>,
    /// `ui.input().time` of the last keystroke in any segment. Used to
    /// hold the caret solid while the user is actively typing.
    last_typed: f64,
}

/// What a click on a rendered segment means.
enum SegmentClick {
    None,
    /// Click landed on body — flip the segment to source-edit mode.
    Edit,
    /// Click landed on a `vellum://` link rectangle — navigate to the
    /// named note.
    Link(String),
}

/// Scratch state collected during one frame's render pass. The
/// `show_*` helpers write into it; `show()` applies the result after
/// the inner egui closures unwind.
#[derive(Default)]
struct FrameState {
    new_editing: Option<usize>,
    any_changed: bool,
    any_lost_focus: bool,
    next_focus: Option<egui::Id>,
}

impl Default for MixedEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl MixedEditor {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            editing_index: None,
            renders: HashMap::new(),
            failed: HashMap::new(),
            dirty: false,
            config: EditorConfig::default(),
            pending_focus: None,
            last_typed: f64::NEG_INFINITY,
        }
    }

    pub fn load(&mut self, source: &str) {
        self.segments = segment::parse_segments(source);
        preamble::merge_leading(&mut self.segments);
        if self.segments.is_empty() {
            self.segments.push(String::new());
        }
        self.editing_index = None;
        self.dirty = false;
        log::debug!(
            "mixed: load {} bytes -> {} segments",
            source.len(),
            self.segments.len()
        );
        // Keep render/failed caches: keys are content-addressed, so
        // unchanged segments keep their textures across reloads.
    }

    pub fn source(&self) -> String {
        segment::join(&self.segments)
    }

    /// Returns `Some(name)` when the user clicked a `vellum://`
    /// link in a rendered segment — the caller is expected to open
    /// the matching note.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        engine: &TypstEngine,
    ) -> Option<String> {
        if self.segments.is_empty() {
            self.segments.push(String::new());
        }

        let effective = self.effective_sources();
        self.ensure_rendered(ctx, engine, &effective);
        let pending = self.pending_focus.take();

        let mut state = FrameState {
            new_editing: self.editing_index,
            ..Default::default()
        };

        let mut nav: Option<String> = None;
        show_content_column(ui, |ui| {
            ui.add_space(TOP_PADDING);
            for i in 0..self.segments.len() {
                if let Some(target) =
                    self.show_segment(ui, i, &effective[i], pending, &mut state)
                {
                    nav = Some(target);
                }
                ui.add_space(SEGMENT_GAP);
            }
        });

        self.apply_frame_state(ctx, state);
        nav
    }

    /// Wrapped Typst source per segment (template + preamble + body).
    /// Also serves as the render-cache key.
    fn effective_sources(&self) -> Vec<String> {
        let (preamble_text, preamble_count) = preamble::collect(&self.segments);
        self.segments
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let composed = if i < preamble_count || preamble_text.is_empty() {
                    body.clone()
                } else {
                    format!("{preamble_text}\n\n{body}")
                };
                preamble::wrap_for_render(&composed)
            })
            .collect()
    }

    fn ensure_rendered(
        &mut self,
        ctx: &egui::Context,
        engine: &TypstEngine,
        effective: &[String],
    ) {
        // Compile segments within a per-frame time budget so the UI stays
        // responsive while a long note is loading. Any segment not reached
        // this frame stays in Pending state ("⟳ rendering…") and is
        // compiled on the next repaint.
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(FRAME_COMPILE_BUDGET_MS);
        let mut any_pending = false;

        for key in effective {
            if self.renders.contains_key(key) || self.failed.contains_key(key) {
                continue;
            }
            if std::time::Instant::now() >= deadline {
                any_pending = true;
                continue;
            }
            match engine.render(ctx, key) {
                Ok(tex) => {
                    self.renders.insert(key.clone(), tex);
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    log::warn!(
                        "typst compile error: {}",
                        msg.lines().next().unwrap_or(&msg)
                    );
                    self.failed.insert(key.clone(), msg);
                }
            }
        }

        if any_pending {
            ctx.request_repaint();
        }
    }

    /// Re-split segments after an edit — a blank line typed inside a
    /// paragraph should split it; a `#` at the start of a line may
    /// promote a text segment into a function-call segment.
    fn re_parse(&mut self) {
        let prior_text = self
            .editing_index
            .and_then(|i| self.segments.get(i).cloned());
        let before = self.segments.len();

        self.segments = segment::parse_segments(&self.source());
        preamble::merge_leading(&mut self.segments);
        if self.segments.is_empty() {
            self.segments.push(String::new());
        }

        self.editing_index =
            prior_text.and_then(|t| self.segments.iter().position(|s| s == &t));
        log::debug!(
            "mixed: re-parsed {} -> {} segments (editing={:?})",
            before,
            self.segments.len(),
            self.editing_index
        );
    }

    fn apply_frame_state(&mut self, ctx: &egui::Context, state: FrameState) {
        self.editing_index = state.new_editing;
        if state.any_changed {
            self.dirty = true;
        }
        if state.any_lost_focus {
            self.re_parse();
        }
        if state.next_focus.is_some() {
            self.pending_focus = state.next_focus;
            ctx.request_repaint();
        }
    }

    fn show_segment(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        effective_key: &str,
        pending: Option<egui::Id>,
        state: &mut FrameState,
    ) -> Option<String> {
        let seg_id = egui::Id::new(("mixed-segment", i));
        let is_editing = state.new_editing == Some(i);

        if is_editing {
            let resp = show_editing(
                ui,
                &mut self.segments[i],
                seg_id,
                &self.config,
                self.last_typed,
            );
            if resp.changed() {
                state.any_changed = true;
                self.last_typed = ui.input(|i| i.time);
            }
            if resp.lost_focus() {
                state.new_editing = None;
                state.any_lost_focus = true;
            }
            if Some(seg_id) == pending {
                resp.request_focus();
            }
            style::paint_edit_outline(ui.painter(), resp.rect);
        } else if let Some(err) = self.failed.get(effective_key).cloned() {
            if show_compile_error(ui, &self.segments[i], &err) {
                state.new_editing = Some(i);
                state.next_focus = Some(seg_id);
            }
        } else if let Some(page) = self.renders.get(effective_key).cloned() {
            match show_rendered(ui, &page) {
                SegmentClick::Edit => {
                    state.new_editing = Some(i);
                    state.next_focus = Some(seg_id);
                }
                SegmentClick::Link(target) => return Some(target),
                SegmentClick::None => {}
            }
        } else {
            ui.weak("⟳ rendering…");
        }
        None
    }
}

/// How long after the last keystroke to hold the caret solid before
/// the blink cycle resumes.
const CARET_TYPING_HOLD: f64 = 0.5;

fn show_editing(
    ui: &mut egui::Ui,
    text: &mut String,
    seg_id: egui::Id,
    config: &EditorConfig,
    last_typed: f64,
) -> egui::Response {
    let reduce_font_size = EDIT_FONT_SCALE * config.font_size;

    let font_id = egui::FontId::new(reduce_font_size, config.font_family.clone());
    // `line_space` is the extra gap on top of `font_size`.
    let line_height = config.line_space.map(|space| reduce_font_size + space);
    // Capture-by-value into the layouter so the closure outlives
    // the borrow of `config`.
    let layouter_font = font_id.clone();
    let colors = config.colors.clone();
    let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = highlight::highlight(buf.as_str(), &layouter_font, line_height, &colors);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    };

    // egui paints its caret across the full row span — so any
    // `line_space > 0` would stretch the caret with it. To keep the
    // caret at `font_size`, we suppress egui's caret here and paint
    // a manual one below.
    let real_cursor_stroke = ui.visuals().text_cursor.stroke;
    ui.visuals_mut().text_cursor.stroke.color = egui::Color32::TRANSPARENT;

    let top_margin = (line_height.unwrap_or(reduce_font_size) - reduce_font_size)
        .round() as i8;
    let bottom_margin = (0.1 * reduce_font_size).round() as i8;
    let inner_margin = egui::Margin {
        left: 20,
        right: 6,
        top: top_margin,
        bottom: bottom_margin,
    };

    // egui 0.34's default `TextEdit` frame paints a stroke (the
    // widget's `bg_stroke` when blurred, `selection.stroke` when
    // focused). On top of that we paint our blue edit outline — two
    // strokes around the same widget. Supplying a custom `Frame::NONE`
    // makes `TextEdit::show` keep the frame untouched (no fill, no
    // stroke), so only `style::paint_edit_outline` draws around the
    // segment.
    let output = egui::TextEdit::multiline(text)
        .id(seg_id)
        .font(font_id.clone())
        .desired_width(CONTENT_WIDTH_PT)
        .desired_rows(1)
        .frame(egui::Frame::NONE.inner_margin(inner_margin))
        .margin(inner_margin)
        .layouter(&mut layouter)
        .show(ui);

    ui.visuals_mut().text_cursor.stroke = real_cursor_stroke;

    let response = output.response.response;
    if response.has_focus() {
        if let Some(range) = output.cursor_range {
            paint_caret(
                ui,
                &output.galley,
                output.galley_pos,
                range.primary,
                reduce_font_size,
                real_cursor_stroke,
                last_typed,
            );
        }
    }

    response
}

/// Seconds per caret blink phase (visible or hidden).
const CARET_BLINK_PERIOD: f64 = 0.53;

/// Paint a vertical caret `font_size` points tall at the cursor
/// position, centred vertically within the row. The highlighter
/// builds spans with `valign: Center`, so the caret sits over the
/// glyphs regardless of how much `line_space` the user adds.
///
/// Blinks at `1 / (2 * CARET_BLINK_PERIOD)` Hz; we request a repaint
/// at the next phase boundary so the toggle keeps ticking even when
/// the user is idle.
fn paint_caret(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    cursor: egui::epaint::text::cursor::CCursor,
    font_size: f32,
    stroke: egui::Stroke,
    last_typed: f64,
) {
    let time = ui.input(|i| i.time);
    let since_typed = time - last_typed;
    let (visible, until_next_phase) = if since_typed < CARET_TYPING_HOLD {
        // Hold solid while the user is actively typing, then resume
        // blinking from the start of a visible phase.
        (true, CARET_TYPING_HOLD - since_typed)
    } else {
        let phase_into = time.rem_euclid(CARET_BLINK_PERIOD * 2.0);
        let visible = phase_into < CARET_BLINK_PERIOD;
        let until_next = if visible {
            CARET_BLINK_PERIOD - phase_into
        } else {
            CARET_BLINK_PERIOD * 2.0 - phase_into
        };
        (visible, until_next)
    };
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(until_next_phase));

    if !visible {
        return;
    }

    let pos = galley.pos_from_cursor(cursor);
    let row_top = if pos.max.y > pos.min.y {
        pos.min.y + galley_pos.y
    } else {
        // Empty galley: `pos_from_cursor` returns a zero-sized rect,
        // so the cursor sits at the galley origin.
        galley_pos.y
    };
    let x = pos.min.x + galley_pos.x;
    ui.painter().line_segment(
        [
            egui::pos2(x, row_top),
            egui::pos2(x, row_top + font_size*1.1),
        ],
        stroke,
    );
}

/// Render the compile-error UI for a segment and return whether the
/// user clicked into the source — the cue to flip it to edit mode.
fn show_compile_error(ui: &mut egui::Ui, body: &str, err: &str) -> bool {
    ui.colored_label(
        egui::Color32::LIGHT_RED,
        "compile error (click to edit source)",
    );
    ui.add(
        egui::Label::new(egui::RichText::new(err).monospace().small()).wrap(),
    );
    ui.add(
        egui::Label::new(egui::RichText::new(body).monospace())
            .sense(egui::Sense::click()),
    )
    .clicked()
}

/// Render a compiled-Typst page at 1 egui pt ↔ 1 typst pt and classify
/// any click against the page's overlaid link rectangles. Clicks on a
/// link return `SegmentClick::Link(target)`; clicks elsewhere on the
/// image return `SegmentClick::Edit` (flip to source).
///
/// Hit-testing runs in the response's local coordinates rather than
/// laying down a second `Sense::click()` widget per link — that keeps
/// the image as a single interaction target and avoids egui z-order
/// surprises when a link's rectangle straddles a row boundary.
fn show_rendered(ui: &mut egui::Ui, page: &RenderedPage) -> SegmentClick {
    let [w_px, h_px] = page.texture.size();
    let size = egui::vec2(w_px as f32 / PIXEL_PER_PT, h_px as f32 / PIXEL_PER_PT);
    let resp = ui.add(
        egui::Image::new(&page.texture)
            .fit_to_exact_size(size)
            .sense(egui::Sense::click()),
    );

    // Show a pointing-hand cursor while the pointer is over any link
    // region. Other parts of the image keep the default cursor.
    if let Some(hover) = resp.hover_pos() {
        let local = (hover - resp.rect.min).to_pos2();
        if page.links.iter().any(|l| l.rect.contains(local)) {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
    }

    if !resp.clicked() {
        return SegmentClick::None;
    }
    let Some(click_pos) = resp.interact_pointer_pos() else {
        return SegmentClick::Edit;
    };
    let local = (click_pos - resp.rect.min).to_pos2();
    for link in &page.links {
        if link.rect.contains(local) {
            return SegmentClick::Link(link.target.clone());
        }
    }
    SegmentClick::Edit
}

/// Scrollable, centred, fixed-width content column. All segments lay
/// out inside this column so plain text and rendered Typst blocks
/// share one width; the outer `ScrollArea` handles overflow when the
/// viewport is narrower than `CONTENT_WIDTH_PT`.
fn show_content_column(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::both()
        .id_salt("mixed-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let padding = ((ui.available_width() - CONTENT_WIDTH_PT) / 2.0).max(0.0);
            ui.horizontal_top(|ui| {
                ui.add_space(padding);
                ui.vertical(|ui| {
                    ui.set_min_width(CONTENT_WIDTH_PT);
                    ui.set_max_width(CONTENT_WIDTH_PT);
                    content(ui);
                });
            });
        });
}
