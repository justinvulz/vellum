//! Launches Helix inside a terminal emulator for the current note.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn open_in_helix(path: &Path) -> Result<()> {
    // (terminal, args before the command). Order matters — first hit wins.
    let candidates: &[(&str, &[&str])] = &[
        ("alacritty", &["-e"]),
        ("kitty", &[]),
        ("foot", &[]),
        ("wezterm", &["start", "--"]),
        ("ghostty", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("konsole", &["-e"]),
        ("xterm", &["-e"]),
    ];

    let preferred = std::env::var("TERMINAL").ok();
    let ordered: Vec<(&str, &[&str])> = preferred
        .as_deref()
        .and_then(|p| candidates.iter().find(|(n, _)| *n == p).copied())
        .into_iter()
        .chain(candidates.iter().copied())
        .collect();

    let mut tried = Vec::new();
    for (term, pre_args) in ordered {
        if which(term).is_none() {
            continue;
        }
        let mut cmd = Command::new(term);
        cmd.args(pre_args).arg("hx").arg(path);
        match cmd.spawn() {
            Ok(_) => return Ok(()),
            Err(e) => tried.push(format!("{term}: {e}")),
        }
    }
    Err(anyhow::anyhow!(
        "no working terminal found. tried: {}",
        if tried.is_empty() {
            "none on PATH".into()
        } else {
            tried.join("; ")
        }
    ))
    .with_context(|| format!("opening {} in helix", path.display()))
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
