use crate::vault::Vault;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Match the first positional string argument of a `#line-note("X")`
/// call. Tolerates whitespace around the parenthesis. Body / named
/// arguments after the first string are ignored — the link target is
/// the first quoted string regardless.
fn line_note_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"#line-note\s*\(\s*"([^"]+)""#).unwrap())
}

pub fn extract_links(source: &str) -> Vec<String> {
    line_note_re()
        .captures_iter(source)
        .map(|c| c[1].trim().to_string())
        .collect()
}

/// Map from a note's path to the paths of notes that link to it via
/// `#line-note(...)`. Targets are resolved at index time so that both
/// stem (`"foo"`) and path-qualified (`"ideas/foo"`) link forms land
/// on the same target note.
pub type BacklinkIndex = HashMap<PathBuf, Vec<PathBuf>>;

pub fn build_backlinks(vault: &Vault) -> BacklinkIndex {
    let mut index: BacklinkIndex = HashMap::new();
    for note in &vault.notes {
        let source = match vault.read_note(note) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("backlinks: skip {}: {}", note.display(), e);
                continue;
            }
        };
        for target in extract_links(&source) {
            let Some(target_path) = find_note_by_stem(vault, &target) else {
                continue;
            };
            index.entry(target_path).or_default().push(note.clone());
        }
    }
    // A source note linking to the same target multiple times should
    // appear once in the backlinks panel, not once per `#line-note`
    // occurrence.
    for refs in index.values_mut() {
        refs.sort();
        refs.dedup();
    }
    index
}

pub fn backlinks_for<'a>(index: &'a BacklinkIndex, note: &Path) -> Option<&'a Vec<PathBuf>> {
    index.get(note)
}

/// Rewrite `#line-note("X")` calls in `source` so that any X currently
/// resolving to `old` is updated to point to `new`. Returns `Some` only
/// if at least one substitution was made; `None` means the source is
/// unchanged and the caller should skip writing.
///
/// Stem-only targets stay stem-only (`"foo"` → `"bar"`); path-qualified
/// targets stay path-qualified (`"ideas/foo"` → `"projects/bar"`). Both
/// `old` and `new` are full filesystem paths; the resolution check uses
/// the *current* vault state, so call this *before* renaming the file
/// on disk.
pub fn rewrite_link_targets(
    source: &str,
    vault: &Vault,
    old: &Path,
    new: &Path,
) -> Option<String> {
    let re = line_note_re();
    let mut result = String::with_capacity(source.len());
    let mut last_end = 0;
    let mut any_change = false;
    for caps in re.captures_iter(source) {
        let target_m = caps.get(1).expect("regex always has capture group 1");
        let raw = target_m.as_str();
        let trimmed = raw.trim();

        let resolved = find_note_by_stem(vault, trimmed);
        let replacement = if resolved.as_deref() == Some(old) {
            new_target_for(vault, trimmed, new)
        } else {
            None
        };

        result.push_str(&source[last_end..target_m.start()]);
        match replacement {
            Some(nt) if nt != raw => {
                result.push_str(&nt);
                any_change = true;
            }
            _ => result.push_str(raw),
        }
        last_end = target_m.end();
    }
    if !any_change {
        return None;
    }
    result.push_str(&source[last_end..]);
    Some(result)
}

/// Pick the textual form to substitute into the link target slot,
/// preserving whichever form the user originally wrote.
fn new_target_for(vault: &Vault, original_target: &str, new_path: &Path) -> Option<String> {
    if original_target.contains('/') {
        // Path-qualified — produce the new vault-relative path without
        // the `.typ` extension.
        let notes_dir = vault.root.join("note");
        let rel = new_path.strip_prefix(&notes_dir).ok()?;
        Some(rel.with_extension("").to_string_lossy().into_owned())
    } else {
        // Stem-only — produce the new filename stem.
        new_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
    }
}

/// Resolve a `#line-note` link target to a vault note path.
///
/// If `name` contains `/` it is treated as a path relative to `note/`
/// (without the `.typ` extension), so `"ideas/foo"` matches
/// `note/ideas/foo.typ` unambiguously. Otherwise every note whose
/// filename stem equals `name` (case-insensitive) is a candidate and
/// the first match in sorted order wins.
pub fn find_note_by_stem(vault: &Vault, name: &str) -> Option<PathBuf> {
    let needle = name.to_ascii_lowercase();
    if needle.contains('/') {
        let notes_dir = vault.root.join("note");
        vault.notes.iter().find(|p| {
            p.strip_prefix(&notes_dir)
                .ok()
                .map(|rel| {
                    rel.with_extension("").to_string_lossy().to_ascii_lowercase() == needle
                })
                .unwrap_or(false)
        }).cloned()
    } else {
        vault
            .notes
            .iter()
            .find(|p| {
                Vault::note_stem(p)
                    .map(|s| s.to_ascii_lowercase() == needle)
                    .unwrap_or(false)
            })
            .cloned()
    }
}

pub struct ContentHit {
    pub path: PathBuf,
    pub line: usize,
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::extract_links;

    #[test]
    fn extracts_single_target() {
        assert_eq!(extract_links(r#"#line-note("foo")"#), vec!["foo"]);
    }

    #[test]
    fn extracts_path_qualified_target() {
        assert_eq!(
            extract_links(r#"#line-note("ideas/foo")"#),
            vec!["ideas/foo"]
        );
    }

    #[test]
    fn extracts_inline_target_in_prose() {
        assert_eq!(
            extract_links(r#"See #line-note("bar") for context."#),
            vec!["bar"]
        );
    }

    #[test]
    fn extracts_multiple_targets() {
        let src = r#"#line-note("first") and #line-note("second")"#;
        assert_eq!(extract_links(src), vec!["first", "second"]);
    }

    #[test]
    fn ignores_named_body_argument() {
        // Only the first positional string is the target; `body: [...]`
        // is for rendering and never a link target.
        assert_eq!(
            extract_links(r#"#line-note("foo", body: [Click])"#),
            vec!["foo"]
        );
    }

    #[test]
    fn ignores_wiki_link_syntax() {
        // `[[wiki-link]]` is not supported — only `#line-note(...)` is.
        assert!(extract_links("see [[foo]] for more").is_empty());
    }
}

pub fn content_search(vault: &Vault, query: &str, limit: usize) -> Vec<ContentHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_ascii_lowercase();
    let mut hits = Vec::new();
    for note in &vault.notes {
        let source = match vault.read_note(note) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("content search: skip {}: {}", note.display(), e);
                continue;
            }
        };
        for (i, line) in source.lines().enumerate() {
            if line.to_ascii_lowercase().contains(&needle) {
                hits.push(ContentHit {
                    path: note.clone(),
                    line: i + 1,
                    snippet: line.trim().to_string(),
                });
                if hits.len() >= limit {
                    return hits;
                }
            }
        }
    }
    hits
}
