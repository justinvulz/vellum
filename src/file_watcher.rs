//! Notifies the app of external `.typ` file changes inside the vault.

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

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
        log::debug!("file watcher: watching {}", root.display());
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn drain_changes(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        if path.extension().and_then(|s| s.to_str()) == Some("typ") {
                            changed.push(path);
                        }
                    }
                }
                Ok(Err(e)) => log::warn!("file watcher: {e}"),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("file watcher: channel disconnected");
                    break;
                }
            }
        }
        if !changed.is_empty() {
            log::debug!("file watcher: {} .typ change(s)", changed.len());
        }
        changed
    }
}
