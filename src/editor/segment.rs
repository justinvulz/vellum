//! Splits Typst source into segments using the Typst syntax tree.
//!
//! Block-level constructs each become their own segment, even when no
//! blank line separates them from neighbours:
//!  - headings (`= Title`)
//!  - block math (`$ ... $` — Typst's block form, where the body has
//!    whitespace immediately inside the dollar signs)
//!  - `#`-prefixed code that is alone on its source line (e.g.
//!    `#table(...)[]`, `#let x = 1`, `#show: ...`)
//!
//! Plain prose, list items, inline markup, inline math (`$x$`), and
//! inline function calls inside a sentence accumulate into text
//! segments split only by blank lines (top-level `Parbreak`).

use typst::syntax::{
    ast::{AstNode, Equation},
    parse, SyntaxKind, SyntaxNode,
};

pub fn parse_segments(source: &str) -> Vec<String> {
    let root = parse(source);
    let children: Vec<&SyntaxNode> = root.children().collect();

    // Cumulative byte offsets so each child knows where it sits in `source`.
    let mut starts = Vec::with_capacity(children.len() + 1);
    starts.push(0usize);
    let mut acc = 0usize;
    for child in &children {
        acc += child.len();
        starts.push(acc);
    }

    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < children.len() {
        let start = starts[i];
        let end = starts[i + 1];
        let text = &source[start..end];

        match children[i].kind() {
            SyntaxKind::Heading => {
                flush(&mut segments, &mut current);
                segments.push(text.trim().to_string());
                i += 1;
            }
            SyntaxKind::Equation if is_block_equation(children[i]) => {
                flush(&mut segments, &mut current);
                segments.push(text.trim().to_string());
                i += 1;
            }
            SyntaxKind::Hash if i + 1 < children.len() => {
                // Pair the Hash with the following code expression. Treat
                // the pair as a block segment only when it sits alone on
                // its source line — otherwise it's inline like `#strong[…]`
                // inside running prose.
                let expr_end = starts[i + 2];
                if alone_on_line(source, start, expr_end) {
                    flush(&mut segments, &mut current);
                    segments.push(source[start..expr_end].trim().to_string());
                    i += 2;
                } else {
                    current.push_str(text);
                    i += 1;
                }
            }
            SyntaxKind::Parbreak => {
                flush(&mut segments, &mut current);
                i += 1;
            }
            _ => {
                current.push_str(text);
                i += 1;
            }
        }
    }
    flush(&mut segments, &mut current);
    segments
}

fn flush(segs: &mut Vec<String>, cur: &mut String) {
    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        segs.push(trimmed.to_string());
    }
    cur.clear();
}

fn is_block_equation(node: &SyntaxNode) -> bool {
    Equation::from_untyped(node)
        .map(|e| e.block())
        .unwrap_or(false)
}

/// True when the byte span `[start..end)` is the only non-whitespace
/// content on its source line — i.e., walking back to the previous
/// newline (or source start) and forward to the next newline (or
/// source end) crosses only spaces and tabs.
fn alone_on_line(source: &str, start: usize, end: usize) -> bool {
    let bytes = source.as_bytes();
    let mut i = start;
    while i > 0 {
        match bytes[i - 1] {
            b' ' | b'\t' => i -= 1,
            b'\n' => break,
            _ => return false,
        }
    }
    let mut j = end;
    while j < bytes.len() {
        match bytes[j] {
            b' ' | b'\t' => j += 1,
            b'\n' => break,
            _ => return false,
        }
    }
    true
}

pub fn join(segments: &[String]) -> String {
    segments.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_paragraph() {
        assert_eq!(parse_segments("hello world"), vec!["hello world"]);
    }

    #[test]
    fn heading_splits_without_blank_line() {
        let segs = parse_segments("= Title\ntext below");
        assert_eq!(segs, vec!["= Title", "text below"]);
    }

    #[test]
    fn block_math_splits_without_blank_line() {
        let segs = parse_segments("text\n$ E = m c^2 $\nmore");
        assert_eq!(segs, vec!["text", "$ E = m c^2 $", "more"]);
    }

    #[test]
    fn function_call_splits_without_blank_line() {
        let segs = parse_segments("#table()[a]\nmore");
        assert_eq!(segs, vec!["#table()[a]", "more"]);
    }

    #[test]
    fn full_mix_without_blank_lines() {
        let src = "= Title\nsome text below\n$ E = m c^2 $\nmore text\n#table()[a]\nfinal";
        let segs = parse_segments(src);
        assert_eq!(
            segs,
            vec![
                "= Title",
                "some text below",
                "$ E = m c^2 $",
                "more text",
                "#table()[a]",
                "final",
            ]
        );
    }

    #[test]
    fn inline_function_stays_in_text_segment() {
        let segs = parse_segments("Hello #strong[bold] world");
        assert_eq!(segs, vec!["Hello #strong[bold] world"]);
    }

    #[test]
    fn inline_math_stays_in_text_segment() {
        let segs = parse_segments("Hello $x$ world");
        assert_eq!(segs, vec!["Hello $x$ world"]);
    }

    #[test]
    fn multiline_function_call_stays_one_segment() {
        let src = "#table(\n  columns: 2,\n)[\n  a\n\n  b\n]\n\ntext";
        let segs = parse_segments(src);
        assert_eq!(segs.len(), 2, "got: {:?}", segs);
        assert!(segs[0].starts_with("#table"));
    }

    #[test]
    fn consecutive_let_bindings_split() {
        let segs = parse_segments("#let x = 1\n#let y = 2\ntext");
        assert_eq!(segs, vec!["#let x = 1", "#let y = 2", "text"]);
    }

    #[test]
    fn round_trip_with_blank_lines() {
        let src = "hello\n\n= Heading\n\nmore text\n\n#table()";
        let segs = parse_segments(src);
        assert_eq!(join(&segs), src);
    }
}
