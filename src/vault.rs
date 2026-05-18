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
