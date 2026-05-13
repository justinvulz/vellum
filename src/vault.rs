use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const TYPST_TOML: &str = "[package]\nname = \"vellum-notes\"\nversion = \"0.1.0\"\n";

const DEFAULT_THEME: &str = include_str!("../assets/default_theme.typ");

const NOTE_BOILERPLATE: &str = "#import \"/asset/theme.typ\": template\n#show: template\n\n";

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<PathBuf>,
}

impl Vault {
    pub fn open_or_init(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("note"))
            .with_context(|| format!("creating vault/note at {}", root.display()))?;
        fs::create_dir_all(root.join("asset"))
            .with_context(|| format!("creating vault/asset at {}", root.display()))?;
        let manifest = root.join("typst.toml");
        if !manifest.exists() {
            fs::write(&manifest, TYPST_TOML)
                .with_context(|| "writing typst.toml")?;
        }
        let theme_path = root.join("asset").join("theme.typ");
        if !theme_path.exists() {
            fs::write(&theme_path, DEFAULT_THEME)
                .with_context(|| "writing default theme template")?;
        }
        let mut vault = Self {
            root,
            notes: Vec::new(),
        };
        vault.rescan();
        Ok(vault)
    }

    pub fn rescan(&mut self) {
        let notes_dir = self.root.join("note");
        self.notes = WalkDir::new(&notes_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("typ"))
            .collect();
        self.notes.sort();
    }

    pub fn read_note(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))
    }

    pub fn write_note(&self, path: &Path, contents: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, contents)
            .with_context(|| format!("writing {}", path.display()))
    }

    pub fn create_note(&mut self, name: &str) -> Result<PathBuf> {
        let mut path = self.root.join("note").join(name);
        if path.extension().and_then(|s| s.to_str()) != Some("typ") {
            path.set_extension("typ");
        }
        if !path.exists() {
            self.write_note(&path, NOTE_BOILERPLATE)?;
        }
        self.rescan();
        Ok(path)
    }

    pub fn delete_note(&mut self, path: &Path) -> Result<()> {
        fs::remove_file(path)
            .with_context(|| format!("deleting {}", path.display()))?;
        self.rescan();
        Ok(())
    }

    pub fn rename_note(&mut self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to)
            .with_context(|| format!("renaming {} -> {}", from.display(), to.display()))?;
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
