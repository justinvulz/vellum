//! Mixed inline editor: every segment renders through the Typst engine
//! and flips to a source `TextEdit` on click. Headings, math blocks,
//! function calls, and plain prose all flow through the same pipeline.

use super::preamble;
use super::segment;
use super::typst_engine::{TypstEngine, PIXEL_PER_PT};
use crate::style::{self, CONTENT_WIDTH_PT, EDITOR_PT};
use std::collections::HashMap;

/// Vertical gap between adjacent segments, in egui points.
const SEGMENT_GAP: f32 = 6.0;

pub struct MixedEditor {
    pub segments: Vec<String>,
    pub editing_index: Option<usize>,
    pub renders: HashMap<String, egui::TextureHandle>,
    pub failed: HashMap<String, String>,
    pub dirty: bool,
    /// Focus to apply on the next frame, once the matching `TextEdit`
    /// exists. Set when the user clicks a rendered segment.
    pending_focus: Option<egui::Id>,
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

impl MixedEditor {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            editing_index: None,
            renders: HashMap::new(),
            failed: HashMap::new(),
            dirty: false,
            pending_focus: None,
        }
    }

    pub fn load(&mut self, source: &str) {
        self.segments = segment::parse_segments(source);
        if self.segments.is_empty() {
            self.segments.push(String::new());
        }
        self.editing_index = None;
        self.dirty = false;
        // Keep render/failed caches: keys are content-addressed, so
        // unchanged segments keep their textures across reloads.
    }

    pub fn source(&self) -> String {
        segment::join(&self.segments)
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        engine: &TypstEngine,
    ) {
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

        show_content_column(ui, |ui| {
            for i in 0..self.segments.len() {
                self.show_segment(ui, i, &effective[i], pending, &mut state);
                ui.add_space(SEGMENT_GAP);
            }
        });

        self.apply_frame_state(ctx, state);
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
        let needed: Vec<String> = effective
            .iter()
            .filter(|key| {
                !self.renders.contains_key(*key) && !self.failed.contains_key(*key)
            })
            .cloned()
            .collect();

        for key in needed {
            match engine.render(ctx, &key) {
                Ok(tex) => {
                    self.renders.insert(key, tex);
                }
                Err(e) => {
                    self.failed.insert(key, format!("{e:#}"));
                }
            }
        }
    }

    /// Re-split segments after an edit — a blank line typed inside a
    /// paragraph should split it; a `#` at the start of a line may
    /// promote a text segment into a function-call segment.
    fn re_parse(&mut self) {
        let prior_text = self
            .editing_index
            .and_then(|i| self.segments.get(i).cloned());

        self.segments = segment::parse_segments(&self.source());
        if self.segments.is_empty() {
            self.segments.push(String::new());
        }

        self.editing_index =
            prior_text.and_then(|t| self.segments.iter().position(|s| s == &t));
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
    ) {
        let seg_id = egui::Id::new(("mixed-segment", i));
        let is_editing = state.new_editing == Some(i);

        if is_editing {
            let resp = show_editing(ui, &mut self.segments[i], seg_id);
            if resp.changed() {
                state.any_changed = true;
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
        } else if let Some(tex) = self.renders.get(effective_key).cloned() {
            if show_rendered(ui, &tex) {
                state.new_editing = Some(i);
                state.next_focus = Some(seg_id);
            }
        } else {
            ui.weak("⟳ rendering…");
        }
    }
}

fn show_editing(
    ui: &mut egui::Ui,
    text: &mut String,
    seg_id: egui::Id,
) -> egui::Response {
    ui.add(
        egui::TextEdit::multiline(text)
            .id(seg_id)
            .font(egui::FontId::new(EDITOR_PT, egui::FontFamily::Monospace))
            .desired_width(CONTENT_WIDTH_PT),
    )
}

/// Render the compile-error UI for a segment and return whether the
/// user clicked into the source — the cue to flip it to edit mode.
fn show_compile_error(ui: &mut egui::Ui, body: &str, err: &str) -> bool {
    ui.colored_label(
        egui::Color32::LIGHT_RED,
        "compile error (click to edit source)",
    );
    ui.add(
        egui::Label::new(egui::RichText::new(err).monospace().small()).wrap(true),
    );
    ui.add(
        egui::Label::new(egui::RichText::new(body).monospace())
            .sense(egui::Sense::click()),
    )
    .clicked()
}

/// Render a compiled-Typst texture at 1 egui pt ↔ 1 typst pt. Returns
/// whether the user clicked, signalling a flip to edit mode.
fn show_rendered(ui: &mut egui::Ui, tex: &egui::TextureHandle) -> bool {
    let [w_px, h_px] = tex.size();
    let size = egui::vec2(w_px as f32 / PIXEL_PER_PT, h_px as f32 / PIXEL_PER_PT);
    ui.add(
        egui::Image::new(tex)
            .fit_to_exact_size(size)
            .sense(egui::Sense::click()),
    )
    .clicked()
}

/// Scrollable, centred, fixed-width content column. All segments lay
/// out inside this column so plain text and rendered Typst blocks
/// share one width; the outer `ScrollArea` handles overflow when the
/// viewport is narrower than `CONTENT_WIDTH_PT`.
fn show_content_column(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::both()
        .id_source("mixed-scroll")
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
