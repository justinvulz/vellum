//! Preamble detection and Typst source wrapping for the mixed editor.
//!
//! The first run of "preamble-only" segments at the top of a note —
//! lines containing only `#let`, `#import`, `#set`, `#show`, comments,
//! or blanks — is prepended to every later segment before compilation
//! so bindings and imports are in scope across all blocks.
//!
//! Every render body is also wrapped in the theme template so plain
//! prose and rendered blocks share styling.

use crate::style::{CONTENT_WIDTH_PT, EDITOR_PT};

/// True when every line is a preamble line: `#let`, `#import`,
/// `#set`, `#show`, a `//` line comment, or blank.
pub fn is_preamble_only(text: &str) -> bool {
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

/// Walks the leading run of preamble-only segments. Returns the joined
/// preamble text and the number of segments it spans.
pub fn collect(segments: &[String]) -> (String, usize) {
    let count = segments
        .iter()
        .take_while(|s| is_preamble_only(s))
        .count();
    (segments[..count].join("\n\n"), count)
}

/// Collapse the leading run of preamble-only segments into a single
/// segment so the user clicks once to edit the whole preamble instead
/// of jumping between every `#let` / `#import` / `#set` / `#show`.
/// Only runs when there are 2+ leading preamble segments — a single
/// preamble segment is already one click.
pub fn merge_leading(segments: &mut Vec<String>) {
    let count = segments
        .iter()
        .take_while(|s| is_preamble_only(s))
        .count();
    if count >= 2 {
        let merged = segments[..count].join("\n\n");
        segments.splice(0..count, std::iter::once(merged));
    }
}

/// Wrap a snippet body in the theme template, threading the editor's
/// content width and body size through `template.with(...)`.
///
/// `line-note` is co-imported so user code can write `#line-note("X")`
/// without an explicit `#import`. Clicks on the resulting link are
/// captured by the app (the URL uses the `vellum://` scheme).
pub fn wrap_for_render(body: &str) -> String {
    format!(
        "#import \"/asset/theme.typ\": template, line-note\n\
         #show: template.with(width: {CONTENT_WIDTH_PT}pt, size: {EDITOR_PT}pt)\n\
         \n{body}\n"
    )
}
