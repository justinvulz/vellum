//! Syntax highlighting for the source `TextEdit` in edit mode.
//!
//! Walks Typst's syntax tree, emits one coloured span per leaf node
//! whose kind has an entry in [`SyntaxColors`], and packs the result
//! into an [`egui::text::LayoutJob`] ready to hand to egui's
//! `TextEdit::layouter`. Unrecognised leaves are coloured with
//! [`SyntaxColors::default`].

use crate::style::SyntaxColors;
use typst::syntax::{parse, SyntaxKind, SyntaxNode};

pub fn highlight(
    source: &str,
    font_id: &egui::FontId,
    line_height: Option<f32>,
    colors: &SyntaxColors,
) -> egui::text::LayoutJob {
    let root = parse(source);
    let mut spans = Vec::new();
    walk(&root, 0, &mut spans, colors);

    let mut job = egui::text::LayoutJob::default();
    let mut cursor = 0;
    for (start, end, color) in spans {
        if start > cursor {
            push(&mut job, &source[cursor..start], font_id, colors.default, line_height);
        }
        push(&mut job, &source[start..end], font_id, color, line_height);
        cursor = end;
    }
    if cursor < source.len() {
        push(&mut job, &source[cursor..], font_id, colors.default, line_height);
    }
    job
}

fn push(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_id: &egui::FontId,
    color: egui::Color32,
    line_height: Option<f32>,
) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            line_height,
            // Centre glyphs vertically within the row so the manual
            // caret in `mixed::paint_caret` (also centred) sits over
            // the text rather than above it.
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
}

fn walk(
    node: &SyntaxNode,
    offset: usize,
    spans: &mut Vec<(usize, usize, egui::Color32)>,
    colors: &SyntaxColors,
) {
    if node.children().next().is_none() {
        if let Some(color) = color_for(node.kind(), colors) {
            spans.push((offset, offset + node.len(), color));
        }
        return;
    }
    let mut child_offset = offset;
    for child in node.children() {
        walk(child, child_offset, spans, colors);
        child_offset += child.len();
    }
}

fn color_for(kind: SyntaxKind, colors: &SyntaxColors) -> Option<egui::Color32> {
    match kind {
        SyntaxKind::Dollar => Some(colors.dollar),
        SyntaxKind::Hash => Some(colors.hash),
        SyntaxKind::HeadingMarker => Some(colors.heading_marker),
        SyntaxKind::LineComment | SyntaxKind::BlockComment => Some(colors.comment),
        SyntaxKind::Str => Some(colors.string),
        SyntaxKind::Int | SyntaxKind::Float | SyntaxKind::Numeric => {
            Some(colors.number)
        }
        SyntaxKind::Let
        | SyntaxKind::Set
        | SyntaxKind::Show
        | SyntaxKind::Context
        | SyntaxKind::If
        | SyntaxKind::Else
        | SyntaxKind::For
        | SyntaxKind::In
        | SyntaxKind::While
        | SyntaxKind::Break
        | SyntaxKind::Continue
        | SyntaxKind::Return
        | SyntaxKind::Import
        | SyntaxKind::Include
        | SyntaxKind::As
        | SyntaxKind::Not
        | SyntaxKind::And
        | SyntaxKind::Or
        | SyntaxKind::None
        | SyntaxKind::Auto
        | SyntaxKind::Bool => Some(colors.keyword),
        SyntaxKind::Star | SyntaxKind::Underscore => Some(colors.emphasis),
        SyntaxKind::ListMarker
        | SyntaxKind::EnumMarker
        | SyntaxKind::TermMarker => Some(colors.list_marker),
        SyntaxKind::LeftParen
        | SyntaxKind::RightParen
        | SyntaxKind::LeftBracket
        | SyntaxKind::RightBracket
        | SyntaxKind::LeftBrace
        | SyntaxKind::RightBrace
        | SyntaxKind::Comma
        | SyntaxKind::Semicolon
        | SyntaxKind::Colon => Some(colors.punct),
        SyntaxKind::Ident | SyntaxKind::MathIdent => Some(colors.ident),
        _ => None,
    }
}
