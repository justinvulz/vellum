//! Paragraph-level splitter that classifies each paragraph as
//! `Plain` (editable text) or `Typst` (rendered).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Plain(String),
    Typst(String),
}

pub fn parse(source: &str) -> Vec<Segment> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in source.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }

    paragraphs
        .into_iter()
        .map(|p| {
            if is_plain(&p) {
                Segment::Plain(p)
            } else {
                Segment::Typst(p)
            }
        })
        .collect()
}

pub fn join(segments: &[Segment]) -> String {
    let mut parts = Vec::with_capacity(segments.len());
    for s in segments {
        match s {
            Segment::Plain(t) | Segment::Typst(t) => parts.push(t.as_str()),
        }
    }
    parts.join("\n\n")
}

fn is_plain(p: &str) -> bool {
    if p.contains('$') || p.contains('#') {
        return false;
    }
    for line in p.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('=')
            || trimmed.starts_with("- ")
            || trimmed.starts_with("+ ")
            || trimmed.starts_with("/ ")
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paragraph() {
        let s = parse("hello world");
        assert_eq!(s, vec![Segment::Plain("hello world".into())]);
    }

    #[test]
    fn typst_with_hash() {
        let s = parse("#table(columns: 2)[a][b]");
        assert!(matches!(s[0], Segment::Typst(_)));
    }

    #[test]
    fn typst_with_math() {
        let s = parse("see $ E = mc^2 $ for that");
        assert!(matches!(s[0], Segment::Typst(_)));
    }

    #[test]
    fn typst_heading() {
        let s = parse("= Heading");
        assert!(matches!(s[0], Segment::Typst(_)));
    }

    #[test]
    fn round_trip() {
        let src = "hello\n\n= Heading\n\nmore text\n\n#table()";
        let segs = parse(src);
        assert_eq!(join(&segs), src);
    }
}
