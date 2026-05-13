use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};

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

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn new(root: &Path) -> Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn drain_changes(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(event) = res {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) != Some("typ") {
                        continue;
                    }
                    // Ignore writes inside our snippet cache.
                    if path
                        .components()
                        .any(|c| c.as_os_str() == ".vellum-snippets")
                    {
                        continue;
                    }
                    changed.push(path);
                }
            }
        }
        changed
    }
}
