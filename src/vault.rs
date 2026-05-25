//! Vault directory layout, file I/O for notes, and theme bootstrap.
//!
//! A vault is a directory containing `note/` (the `.typ` files),
//! `asset/` (theme + images), and a `typst.toml` manifest that lets
//! tinymist resolve `/asset/…` imports.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const TYPST_TOML: &str = "[package]\nname = \"vellum-notes\"\nversion = \"0.1.0\"\n";

const DEFAULT_THEME: &str = include_str!("../assets/default_theme.typ");

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
}

impl Vault {
    pub fn open_or_init(root: PathBuf) -> Result<Self> {
        log::debug!("vault: initialising at {}", root.display());
        ensure_directories(&root)?;
        ensure_manifest(&root)?;
        ensure_theme(&root)?;
        let mut vault = Self {
            root,
            notes: Vec::new(),
            folders: Vec::new(),
        };
        vault.rescan();
        log::info!(
            "vault ready: {} ({} notes, {} folders)",
            vault.root.display(),
            vault.notes.len(),
            vault.folders.len()
        );
        Ok(vault)
    }

    pub fn rescan(&mut self) {
        let notes_dir = self.root.join("note");
        let mut notes = Vec::new();
        let mut folders = Vec::new();
        for entry in WalkDir::new(&notes_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path == notes_dir {
                continue;
            }
            if entry.file_type().is_dir() {
                folders.push(path.to_path_buf());
            } else if entry.file_type().is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("typ")
            {
                notes.push(path.to_path_buf());
            }
        }
        notes.sort();
        folders.sort();
        self.notes = notes;
        self.folders = folders;
        log::debug!(
            "vault rescan: {} notes, {} folder(s)",
            self.notes.len(),
            self.folders.len()
        );
    }

    pub fn read_note(&self, path: &Path) -> Result<String> {
        log::debug!("vault: read {}", path.display());
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
    }

    pub fn write_note(&self, path: &Path, contents: &str) -> Result<()> {
        log::debug!(
            "vault: write {} ({} bytes)",
            path.display(),
            contents.len()
        );
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
    }

    /// Create an empty `.typ` note. `name` may include `/` to land the
    /// note inside a subfolder of `note/` — missing parent directories
    /// are created. Fails if the file already exists.
    pub fn create_note(&mut self, name: &str) -> Result<PathBuf> {
        let rel = clean_relative(name)?;
        let mut path = self.root.join("note").join(rel);
        if path.extension().and_then(|s| s.to_str()) != Some("typ") {
            path.set_extension("typ");
        }
        if path.exists() {
            return Err(anyhow::anyhow!(
                "note already exists: {}",
                path.display()
            ));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating parent of {}", path.display()))?;
        }
        fs::write(&path, "")
            .with_context(|| format!("creating {}", path.display()))?;
        self.rescan();
        Ok(path)
    }

    pub fn delete_note(&mut self, path: &Path) -> Result<()> {
        log::debug!("vault: delete note {}", path.display());
        fs::remove_file(path)
            .with_context(|| format!("deleting {}", path.display()))?;
        self.rescan();
        Ok(())
    }

    /// Move a note into `to_folder` (an absolute path under `note/`),
    /// or back to the root `note/` directory when `None`. Fails if a
    /// file with the same name already exists at the destination.
    /// Rewrites `#line-note` references across the vault so links
    /// continue to resolve. Returns the note's new path.
    pub fn move_note(
        &mut self,
        from: &Path,
        to_folder: Option<&Path>,
    ) -> Result<PathBuf> {
        log::debug!(
            "vault: move {} → {}",
            from.display(),
            to_folder
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(root)".into())
        );
        let file_name = from
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("source has no filename: {}", from.display()))?;
        let dest_dir = to_folder
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.join("note"));
        let dest = dest_dir.join(file_name);
        if dest == from {
            return Ok(dest);
        }
        if dest.exists() {
            return Err(anyhow::anyhow!(
                "destination already exists: {}",
                dest.display()
            ));
        }
        self.relocate(from, &dest)
    }

    /// Rename a note within its current folder, keeping the same parent
    /// directory. `new_name` is the new file stem (no `/`, no `.typ`
    /// extension — both are tolerated and stripped). Rewrites
    /// `#line-note` references across the vault.
    pub fn rename_note(&mut self, from: &Path, new_name: &str) -> Result<PathBuf> {
        log::debug!("vault: rename {} -> {}", from.display(), new_name);
        let rel = clean_relative(new_name)?;
        if rel.components().count() != 1 {
            return Err(anyhow::anyhow!(
                "rename name must not contain '/': {}",
                new_name
            ));
        }
        let folder = from.parent().ok_or_else(|| {
            anyhow::anyhow!("source has no parent: {}", from.display())
        })?;
        let mut dest = folder.join(&rel);
        if dest.extension().and_then(|s| s.to_str()) != Some("typ") {
            dest.set_extension("typ");
        }
        if dest == from {
            return Ok(dest);
        }
        if dest.exists() {
            return Err(anyhow::anyhow!(
                "destination already exists: {}",
                dest.display()
            ));
        }
        self.relocate(from, &dest)
    }

    /// Shared rename / move primitive. The dance is:
    ///   1. **Before** touching the filesystem, scan every other note
    ///      for `#line-note` calls that currently resolve to `from`
    ///      and stage their rewritten sources. Resolution uses the
    ///      pre-rename vault state, so stem-only references still find
    ///      the file.
    ///   2. Rename the file on disk.
    ///   3. Write the staged rewrites. Failures here are logged but
    ///      don't abort — the rename has already succeeded.
    fn relocate(&mut self, from: &Path, to: &Path) -> Result<PathBuf> {
        let rewrites = self.collect_link_rewrites(from, to);

        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::rename(from, to)
            .with_context(|| format!("moving {} to {}", from.display(), to.display()))?;

        for (path, new_source) in rewrites {
            if let Err(e) = fs::write(&path, &new_source) {
                log::warn!("link rewrite: failed to update {}: {}", path.display(), e);
            } else {
                log::debug!("link rewrite: updated {}", path.display());
            }
        }

        self.rescan();
        Ok(to.to_path_buf())
    }

    fn collect_link_rewrites(&self, from: &Path, to: &Path) -> Vec<(PathBuf, String)> {
        let mut out = Vec::new();
        for note in &self.notes {
            if note == from {
                // The file being moved is content-irrelevant to its own
                // relocation — its `#line-note` calls point at *other*
                // notes, none of which are changing.
                continue;
            }
            let Ok(source) = fs::read_to_string(note) else {
                continue;
            };
            if let Some(new_source) =
                crate::search::rewrite_link_targets(&source, self, from, to)
            {
                out.push((note.clone(), new_source));
            }
        }
        out
    }

    /// Create an empty subfolder under `note/`. `name` may include `/`
    /// to nest folders. Fails if the path already exists.
    pub fn create_folder(&mut self, name: &str) -> Result<PathBuf> {
        let rel = clean_relative(name)?;
        let path = self.root.join("note").join(rel);
        if path.exists() {
            return Err(anyhow::anyhow!(
                "path already exists: {}",
                path.display()
            ));
        }
        fs::create_dir_all(&path)
            .with_context(|| format!("creating folder {}", path.display()))?;
        self.rescan();
        Ok(path)
    }

    /// Delete an *empty* subfolder under `note/`. Recursive delete is
    /// not exposed — callers (and users) clear the folder explicitly
    /// to avoid wiping notes by accident.
    pub fn delete_folder(&mut self, path: &Path) -> Result<()> {
        log::debug!("vault: delete folder {}", path.display());
        fs::remove_dir(path).with_context(|| {
            format!("deleting folder {} (must be empty)", path.display())
        })?;
        self.rescan();
        Ok(())
    }

    pub fn display_name(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    pub fn note_stem(path: &Path) -> Option<String> {
        path.file_stem().map(|s| s.to_string_lossy().into_owned())
    }
}

pub fn default_vault_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("vellum"))
        .unwrap_or_else(|| PathBuf::from("./vellum"))
}

fn ensure_directories(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("note"))
        .with_context(|| format!("creating vault/note at {}", root.display()))?;
    fs::create_dir_all(root.join("asset"))
        .with_context(|| format!("creating vault/asset at {}", root.display()))?;
    Ok(())
}

fn ensure_manifest(root: &Path) -> Result<()> {
    let manifest = root.join("typst.toml");
    if !manifest.exists() {
        fs::write(&manifest, TYPST_TOML).context("writing typst.toml")?;
    }
    Ok(())
}

/// Normalise a user-supplied name into a vault-relative `PathBuf`,
/// rejecting absolute paths and `..` traversal so creation can't
/// escape `note/`. Trims surrounding whitespace and slashes.
fn clean_relative(name: &str) -> Result<PathBuf> {
    let trimmed = name.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("name is empty"));
    }
    let rel = PathBuf::from(trimmed);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(anyhow::anyhow!("invalid name: {}", trimmed));
    }
    Ok(rel)
}

/// Always overwrite the theme — the template signature (parameters
/// and defaults) is owned by the app, and on-disk drift causes
/// confusing compile errors when the app passes new arguments to
/// `template.with(...)`.
fn ensure_theme(root: &Path) -> Result<()> {
    let theme_path = root.join("asset").join("theme.typ");
    fs::write(&theme_path, DEFAULT_THEME).context("writing default theme template")
}
