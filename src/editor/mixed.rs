//! Mixed inline editor: plain paragraphs become editable `TextEdit`s,
//! Typst paragraphs render as images and flip to source-editing on click.

use super::segment::{self, Segment};
use super::typst_engine::{TypstEngine, PIXEL_PER_PT};
use crate::style::{CONTENT_WIDTH_PT, EDITOR_PT};
use std::collections::HashMap;

pub struct MixedEditor {
    pub segments: Vec<Segment>,
    pub editing_index: Option<usize>,
    pub renders: HashMap<String, egui::TextureHandle>,
    pub failed: HashMap<String, String>,
    pub dirty: bool,
    /// Focus to apply on the next frame (when the TextEdit for the segment
    /// exists). Set when the user clicks a rendered Typst image.
    pending_focus: Option<egui::Id>,
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
        self.segments = segment::parse(source);
        if self.segments.is_empty() {
            self.segments.push(Segment::Plain(String::new()));
        }
        self.editing_index = None;
        self.dirty = false;
        // Keep renders/failed: keys match by content, so unchanged paragraphs
        // keep their cached textures across reloads.
    }

    pub fn source(&self) -> String {
        segment::join(&self.segments)
    }

    /// Initial Typst segments containing only declaration lines (`#let`,
    /// `#import`, `#set`, `#show`, comments, blanks). These are prepended to
    /// every following block so bindings/imports flow through.
    fn preamble(&self) -> (String, usize) {
        let mut parts: Vec<&str> = Vec::new();
        let mut count = 0;
        for seg in &self.segments {
            let text = match seg {
                Segment::Typst(t) => t,
                _ => break,
            };
            if !is_preamble_only(text) {
                break;
            }
            parts.push(text);
            count += 1;
        }
        (parts.join("\n\n"), count)
    }

    /// Build the fully-wrapped typst source for each Typst segment
    /// (template + preamble + body). Returns `None` for Plain segments.
    fn effective_sources(&self) -> Vec<Option<String>> {
        let (preamble, preamble_count) = self.preamble();
        self.segments
            .iter()
            .enumerate()
            .map(|(i, seg)| match seg {
                Segment::Typst(t) => {
                    let body = if i < preamble_count || preamble.is_empty() {
                        t.clone()
                    } else {
                        format!("{preamble}\n\n{t}")
                    };
                    Some(wrap_source(&body))
                }
                _ => None,
            })
            .collect()
    }

    fn ensure_rendered(
        &mut self,
        ctx: &egui::Context,
        engine: &TypstEngine,
        effective: &[Option<String>],
    ) {
        let needed: Vec<String> = effective
            .iter()
            .filter_map(|e| e.as_ref())
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

    /// Re-classify segments after edits (a Plain paragraph that gained `#` or
    /// `$` should flip to Typst; blank lines should split a paragraph).
    fn re_parse(&mut self) {
        let source = self.source();
        let prior_editing = self.editing_index;
        let prior_text = prior_editing
            .and_then(|i| self.segments.get(i))
            .map(|s| match s {
                Segment::Plain(t) | Segment::Typst(t) => t.clone(),
            });

        self.segments = segment::parse(&source);
        if self.segments.is_empty() {
            self.segments.push(Segment::Plain(String::new()));
        }

        // Try to keep the editing cursor on a segment with the same body.
        self.editing_index = prior_text.and_then(|t| {
            self.segments.iter().position(|s| match s {
                Segment::Plain(x) | Segment::Typst(x) => x == &t,
            })
        });
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        engine: &TypstEngine,
    ) {
        if self.segments.is_empty() {
            self.segments.push(Segment::Plain(String::new()));
        }

        let effective = self.effective_sources();
        self.ensure_rendered(ctx, engine, &effective);

        let pending = self.pending_focus.take();
        let mut new_editing = self.editing_index;
        let mut any_changed = false;
        let mut any_lost_focus = false;
        let mut next_focus: Option<egui::Id> = None;

        egui::ScrollArea::both()
            .id_source("mixed-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Centre the column when the viewport is wider than the
                // editor; flush-left (padding == 0) when narrower, letting
                // the outer ScrollArea handle horizontal overflow.
                let padding =
                    ((ui.available_width() - CONTENT_WIDTH_PT) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(padding);
                    ui.vertical(|ui| {
                        ui.set_min_width(CONTENT_WIDTH_PT);
                        ui.set_max_width(CONTENT_WIDTH_PT);
                        for i in 0..self.segments.len() {
                            let seg_id = egui::Id::new(("mixed-segment", i));

                            match &mut self.segments[i] {
                                Segment::Plain(text) => {
                                    let resp = ui.add(
                                        egui::TextEdit::multiline(text)
                                            .id(seg_id)
                                            .frame(false)
                                            .font(egui::FontId::new(
                                                EDITOR_PT,
                                                egui::FontFamily::Proportional,
                                            ))
                                            .desired_width(CONTENT_WIDTH_PT),
                                    );
                                    if resp.changed() {
                                        any_changed = true;
                                    }
                                    if resp.lost_focus() {
                                        any_lost_focus = true;
                                    }
                                    if Some(seg_id) == pending {
                                        resp.request_focus();
                                    }
                                    if resp.has_focus() {
                                        paint_edit_outline(ui.painter(), resp.rect);
                                    }
                                }
                                Segment::Typst(text) => {
                                    let is_editing = new_editing == Some(i);
                                    if is_editing {
                                        let resp = ui.add(
                                            egui::TextEdit::multiline(text)
                                                .id(seg_id)
                                                .font(egui::FontId::new(
                                                    EDITOR_PT,
                                                    egui::FontFamily::Monospace,
                                                ))
                                                .desired_width(CONTENT_WIDTH_PT),
                                        );
                                        if resp.changed() {
                                            any_changed = true;
                                        }
                                        if resp.lost_focus() {
                                            new_editing = None;
                                            any_lost_focus = true;
                                        }
                                        if Some(seg_id) == pending {
                                            resp.request_focus();
                                        }
                                        paint_edit_outline(ui.painter(), resp.rect);
                                    } else if let Some(err) = effective[i]
                                        .as_ref()
                                        .and_then(|k| self.failed.get(k))
                                        .cloned()
                                    {
                                        ui.colored_label(
                                            egui::Color32::LIGHT_RED,
                                            "compile error (click to edit source)",
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&err)
                                                    .monospace()
                                                    .small(),
                                            )
                                            .wrap(true),
                                        );
                                        let resp = ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(text.as_str())
                                                    .monospace(),
                                            )
                                            .sense(egui::Sense::click()),
                                        );
                                        if resp.clicked() {
                                            new_editing = Some(i);
                                            next_focus = Some(seg_id);
                                        }
                                    } else if let Some(tex) = effective[i]
                                        .as_ref()
                                        .and_then(|k| self.renders.get(k))
                                    {
                                        let [w_px, h_px] = tex.size();
                                        // 1 typst pt ↔ 1 egui logical pt; the
                                        // outer ScrollArea handles overflow if
                                        // the panel is narrower than
                                        // CONTENT_WIDTH_PT.
                                        let size = egui::vec2(
                                            w_px as f32 / PIXEL_PER_PT,
                                            h_px as f32 / PIXEL_PER_PT,
                                        );
                                        let resp = ui.add(
                                            egui::Image::new(tex)
                                                .fit_to_exact_size(size)
                                                .sense(egui::Sense::click()),
                                        );
                                        if resp.clicked() {
                                            new_editing = Some(i);
                                            next_focus = Some(seg_id);
                                        }
                                    } else {
                                        ui.weak("⟳ rendering…");
                                    }
                                }
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
            });

        self.editing_index = new_editing;
        if any_changed {
            self.dirty = true;
        }
        if any_lost_focus {
            self.re_parse();
        }
        if next_focus.is_some() {
            self.pending_focus = next_focus;
            ctx.request_repaint();
        }
    }
}

const EDIT_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(0x4a, 0x9e, 0xff);

fn paint_edit_outline(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(
        rect.expand(3.0),
        egui::Rounding::same(4.0),
        egui::Stroke::new(1.5, EDIT_OUTLINE_COLOR),
    );
}

/// Wrap a snippet body with the theme template, threading the app's
/// page width and editor body size through `template.with(...)` so the
/// rendered image stays in lock-step with the surrounding egui layout.
fn wrap_source(body: &str) -> String {
    format!(
        "#import \"/asset/theme.typ\": template\n\
         #show: template.with(width: {CONTENT_WIDTH_PT}pt, size: {EDITOR_PT}pt)\n\
         \n{body}\n"
    )
}

fn is_preamble_only(text: &str) -> bool {
    text.lines().all(|line| {
        let t = line.trim_start();
        t.is_empty()
            || t.starts_with("//")
            || t.starts_with("#let")
            || t.starts_with("#import")
            || t.starts_with("#set")
            || t.starts_with("#show")
    })
}
