use crate::vault::Vault;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap())
}

pub fn extract_links(source: &str) -> Vec<String> {
    link_re()
        .captures_iter(source)
        .map(|c| c[1].trim().to_string())
        .collect()
}

pub type BacklinkIndex = HashMap<String, Vec<PathBuf>>;

pub fn build_backlinks(vault: &Vault) -> BacklinkIndex {
    let mut index: BacklinkIndex = HashMap::new();
    for note in &vault.notes {
        let Ok(source) = vault.read_note(note) else {
            continue;
        };
        for target in extract_links(&source) {
            index
                .entry(target.to_ascii_lowercase())
                .or_default()
                .push(note.clone());
        }
    }
    index
}

pub fn backlinks_for<'a>(index: &'a BacklinkIndex, note: &Path) -> Option<&'a Vec<PathBuf>> {
    let stem = Vault::note_stem(note)?.to_ascii_lowercase();
    index.get(&stem)
}

pub fn filename_search(vault: &Vault, query: &str) -> Vec<PathBuf> {
    if query.is_empty() {
        return vault.notes.clone();
    }
    let q = query.to_ascii_lowercase();
    vault
        .notes
        .iter()
        .filter(|p| {
            Vault::note_stem(p)
                .map(|s| s.to_ascii_lowercase().contains(&q))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

pub struct ContentHit {
    pub path: PathBuf,
    pub line: usize,
    pub snippet: String,
}

pub fn content_search(vault: &Vault, query: &str, limit: usize) -> Vec<ContentHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_ascii_lowercase();
    let mut hits = Vec::new();
    for note in &vault.notes {
        let Ok(source) = vault.read_note(note) else {
            continue;
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

pub fn resolve_link(vault: &Vault, link_name: &str) -> Option<PathBuf> {
    let target = link_name.trim().to_ascii_lowercase();
    vault
        .notes
        .iter()
        .find(|p| {
            Vault::note_stem(p)
                .map(|s| s.to_ascii_lowercase() == target)
                .unwrap_or(false)
        })
        .cloned()
}
