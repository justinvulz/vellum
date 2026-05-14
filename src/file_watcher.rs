//! Notifies the app of external `.typ` file changes inside the vault.

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

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
        while let Ok(Ok(event)) = self.rx.try_recv() {
            for path in event.paths {
                if path.extension().and_then(|s| s.to_str()) == Some("typ") {
                    changed.push(path);
                }
            }
        }
        changed
    }
}
