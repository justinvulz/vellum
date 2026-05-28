//! On-disk configuration loaded from `~/.config/vellum/config.toml`.
//!
//! A single `Config` is loaded once at startup via [`load`] and stored
//! in a process-global `OnceLock` exposed by [`current`]. Style and
//! sizing accessors in [`crate::style`] read from this global so the
//! constants used throughout the app reflect user overrides.
//!
//! Missing or malformed files fall back to defaults; the file is
//! never required. On first run we write a copy of the bundled default
//! to the user's config dir so the location and schema are discoverable.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::style::{SyntaxColors, UiColors};

/// Bundled baseline for every setting. `~/.config/vellum/config.toml`
/// is written from this string on first run and any field the user
/// leaves out is filled from it on every subsequent load.
const DEFAULT_CONFIG_TOML: &str = include_str!("../assets/default_config.toml");

static CONFIG: OnceLock<Config> = OnceLock::new();
static DEFAULTS: OnceLock<Config> = OnceLock::new();

/// User-overridable settings loaded from disk. Every field has a value
/// in the bundled defaults — partial user configs are merged on top of
/// the defaults before being deserialized here, so no `#[serde(default)]`
/// is needed (which is also what keeps the bundled-defaults parse from
/// re-entering `SyntaxColors::default` / `UiColors::default`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Path to the vault directory. Supports a leading `~/` which is
    /// expanded to the user's home. `None` means "use the default
    /// (`~/vellum`)".
    pub vault_path: Option<String>,
    /// Preferred terminal emulator for the Helix integration. When
    /// `None`, falls back to the `$TERMINAL` env var and then to
    /// auto-detection in `external_editor`.
    pub terminal: Option<String>,
    /// Chrome font size (topbar, sidebar, buttons, status line), in pt.
    pub ui_pt: f32,
    /// Editor body font size, in pt. Drives both the rendered Typst
    /// output and the source `TextEdit` (which scales down slightly).
    pub editor_pt: f32,
    /// Editor content column width, in pt.
    pub content_width_pt: f32,
    /// Sans-serif families tried in priority order, both in egui and in
    /// the Typst theme template, so plain prose and rendered blocks
    /// pick up the same face on any given host.
    pub sans_families: Vec<String>,
    /// CJK fallback families. Each match is appended to both
    /// `Proportional` and `Monospace` so egui resolves CJK glyphs the
    /// primary sans / monospace lack.
    pub cjk_families: Vec<String>,
    /// Per-token-kind palette used by the syntax highlighter.
    pub colors: SyntaxColors,
    /// Chrome palette consumed by `style::install_visuals`.
    pub ui_colors: UiColors,
}

impl Default for Config {
    fn default() -> Self {
        defaults().clone()
    }
}

impl Config {
    /// Resolve `vault_path` (expanding `~/`) or fall back to
    /// [`crate::vault::default_vault_dir`].
    pub fn vault_dir(&self) -> PathBuf {
        match self.vault_path.as_deref() {
            Some(p) => expand_tilde(p),
            None => crate::vault::default_vault_dir(),
        }
    }
}

/// Borrow the loaded config. If [`load`] has not been called yet,
/// returns the bundled defaults so every accessor stays usable.
pub fn current() -> &'static Config {
    CONFIG.get_or_init(|| defaults().clone())
}

/// Borrow the bundled-default `Config`. Parsed once from
/// `assets/default_config.toml` and reused by the field-level
/// `Default` impls in [`crate::style`].
pub fn defaults() -> &'static Config {
    DEFAULTS.get_or_init(|| {
        let value: toml::Value = toml::from_str(DEFAULT_CONFIG_TOML)
            .expect("bundled assets/default_config.toml is malformed TOML");
        value
            .try_into()
            .expect("bundled assets/default_config.toml does not fit the Config schema")
    })
}

/// Locate the config file: `$XDG_CONFIG_HOME/vellum/config.toml` (or
/// `~/.config/vellum/config.toml` on most systems). `None` only when
/// no config dir can be resolved at all.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("vellum").join("config.toml"))
}

/// Read the config file, falling back to defaults on missing /
/// malformed input. Writes the bundled `default_config.toml` to disk on
/// first run so the location and schema are discoverable.
///
/// Safe to call exactly once at startup; subsequent calls are a no-op
/// because the `OnceLock` only stores the first value.
pub fn load() -> &'static Config {
    let cfg = read_or_default();
    let _ = CONFIG.set(cfg);
    current()
}

fn read_or_default() -> Config {
    let Some(path) = config_path() else {
        log::warn!("config: no config dir available; using defaults");
        return Config::default();
    };

    if !path.exists() {
        log::info!("config: writing default to {}", path.display());
        if let Err(e) = write_default_sample(&path) {
            log::warn!("config: failed to write default: {}", e);
        }
        return Config::default();
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            log::warn!(
                "config: read error on {} ({}); using defaults",
                path.display(),
                e
            );
            return Config::default();
        }
    };

    match merged_config(&text) {
        Ok(cfg) => {
            log::info!("config: loaded {}", path.display());
            cfg
        }
        Err(e) => {
            log::warn!(
                "config: parse error in {} ({}); using defaults",
                path.display(),
                e
            );
            Config::default()
        }
    }
}

/// Merge the user's TOML text on top of the bundled-default TOML at
/// `toml::Value` level, then deserialize the result into [`Config`].
///
/// Going through `Value` lets the user omit any field (or any whole
/// table) — the bundled value fills it back in — *without* having to
/// put `#[serde(default)]` on `Config` / `UiColors` / `SyntaxColors`,
/// which would cause serde to call `<T as Default>::default()` eagerly
/// at the start of every deserialization (and so re-enter the
/// bundled-config `OnceLock` and deadlock).
fn merged_config(user_text: &str) -> Result<Config, toml::de::Error> {
    let bundled: toml::Value = toml::from_str(DEFAULT_CONFIG_TOML)
        .expect("bundled default_config.toml is malformed TOML");
    let user: toml::Value = toml::from_str(user_text)?;
    let merged = merge_values(bundled, user);
    merged.try_into()
}

/// Recursive table merge: keys present in `overlay` win; tables are
/// merged key-by-key, everything else is replaced wholesale.
fn merge_values(base: toml::Value, overlay: toml::Value) -> toml::Value {
    use toml::Value::Table;
    match (base, overlay) {
        (Table(mut b), Table(o)) => {
            for (k, v) in o {
                let merged = match b.remove(&k) {
                    Some(bv) => merge_values(bv, v),
                    None => v,
                };
                b.insert(k, merged);
            }
            Table(b)
        }
        (_, overlay) => overlay,
    }
}

fn write_default_sample(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_CONFIG_TOML)
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if s == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(s)
}
