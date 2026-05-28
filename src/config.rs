//! On-disk configuration loaded from `~/.config/vellum/config.toml`.
//!
//! A single `Config` is loaded once at startup via [`load`] and stored
//! in a process-global `OnceLock` exposed by [`current`]. Style and
//! sizing accessors in [`crate::style`] read from this global so the
//! constants used throughout the app reflect user overrides.
//!
//! Missing or malformed files fall back to defaults; the file is
//! never required. On first run we write a commented sample so users
//! can discover what is configurable without consulting the source.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::style::SyntaxColors;

static CONFIG: OnceLock<Config> = OnceLock::new();

/// User-overridable settings loaded from disk. All fields have a
/// reasonable default; the on-disk file may omit any of them.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
    /// Per-token-kind palette used by the syntax highlighter.
    pub colors: SyntaxColors,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault_path: None,
            terminal: None,
            ui_pt: 14.0,
            editor_pt: 20.0,
            content_width_pt: 800.0,
            colors: SyntaxColors::default(),
        }
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
/// returns a process-wide default — every accessor stays usable.
pub fn current() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

/// Locate the config file: `$XDG_CONFIG_HOME/vellum/config.toml` (or
/// `~/.config/vellum/config.toml` on most systems). `None` only when
/// no config dir can be resolved at all.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("vellum").join("config.toml"))
}

/// Read the config file, falling back to defaults on missing /
/// malformed input. Writes a commented sample file on first run so
/// the location and schema are discoverable.
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

    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<Config>(&text) {
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
        },
        Err(e) => {
            log::warn!(
                "config: read error on {} ({}); using defaults",
                path.display(),
                e
            );
            Config::default()
        }
    }
}

fn write_default_sample(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_SAMPLE)
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

const DEFAULT_SAMPLE: &str = r##"# Vellum configuration.
# Every field is optional; uncomment to override the default.

# Path to the vault directory. Default: ~/vellum
# vault_path = "~/notes"

# Preferred terminal for `Ctrl+E` (open in Helix). When unset, the app
# checks $TERMINAL and then probes alacritty/kitty/foot/wezterm/...
# terminal = "alacritty"

# Sizing knobs (all in typographic points).
ui_pt = 14.0
editor_pt = 20.0
content_width_pt = 800.0

# Editor syntax colors. Hex strings, with or without leading '#'.
[colors]
default        = "#d4d4d4"
dollar         = "#c586c0"
hash           = "#4ec9b0"
heading_marker = "#dcdcaa"
comment        = "#6a9955"
string         = "#ce9178"
number         = "#b5cea8"
keyword        = "#569cd6"
ident          = "#9cdcfe"
punct          = "#808080"
emphasis       = "#ffd700"
list_marker    = "#ff8c42"
"##;
