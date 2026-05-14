//! Mixed inline editor: plain paragraphs become editable `TextEdit`s,
//! Typst paragraphs render as images and flip to source-editing on click.

use super::segment::{self, Segment};
use super::typst_engine::TypstEngine;
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

    /// Build the "effective" source for each Typst segment (preamble + body
    /// for segments after the preamble; body alone for the preamble blocks
    /// themselves). Returns `None` for Plain segments.
    fn effective_sources(&self) -> Vec<Option<String>> {
        let (preamble, preamble_count) = self.preamble();
        self.segments
            .iter()
            .enumerate()
            .map(|(i, seg)| match seg {
                Segment::Typst(t) => Some(if i < preamble_count || preamble.is_empty() {
                    t.clone()
                } else {
                    format!("{preamble}\n\n{t}")
                }),
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
            match engine.render_snippet(ctx, &key, 2.0) {
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

        egui::ScrollArea::vertical()
            .id_source("mixed-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for i in 0..self.segments.len() {
                    let seg_id = egui::Id::new(("mixed-segment", i));

                    match &mut self.segments[i] {
                        Segment::Plain(text) => {
                            let resp = ui.add(
                                egui::TextEdit::multiline(text)
                                    .id(seg_id)
                                    .frame(false)
                                    .desired_width(f32::INFINITY),
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
                        }
                        Segment::Typst(text) => {
                            let is_editing = new_editing == Some(i);
                            if is_editing {
                                let resp = ui.add(
                                    egui::TextEdit::multiline(text)
                                        .id(seg_id)
                                        .code_editor()
                                        .desired_width(f32::INFINITY),
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
                                        egui::RichText::new(&err).monospace().small(),
                                    )
                                    .wrap(true),
                                );
                                let resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(text.as_str()).monospace(),
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
                                let [w, h] = tex.size();
                                let avail = ui.available_width();
                                let scale = (avail / w as f32).min(1.0);
                                let size = egui::vec2(w as f32 * scale, h as f32 * scale);
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
