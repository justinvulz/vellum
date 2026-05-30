//! Cross-note LRU cache for compiled Typst segments.
//!
//! Lives on `App` so cached textures survive note switches: a heading,
//! preamble, or block shared between notes hits the cache instead of
//! recompiling. Bounded by total RGBA byte footprint of held textures;
//! once over budget the least-recently-used entries are dropped until
//! the cache fits again.

use super::typst_engine::RenderedPage;
use std::collections::HashMap;

/// Default byte budget for cached `RenderedPage` textures
/// (width × height × 4). Caps steady-state heap + GPU footprint while
/// leaving plenty of room for long notes and cross-note reuse.
pub const DEFAULT_BYTE_BUDGET: usize = 256 * 1024 * 1024;

/// Hard cap on the failed-compile cache. Entries are small (an error
/// string), so the cap is large enough not to matter in practice while
/// still bounding pathological growth.
const FAILED_CAP: usize = 1024;

struct Entry {
    page: RenderedPage,
    bytes: usize,
    last_used: u64,
}

pub struct RenderCache {
    entries: HashMap<String, Entry>,
    failed: HashMap<String, (String, u64)>,
    byte_budget: usize,
    bytes_in_use: usize,
    counter: u64,
}

impl RenderCache {
    pub fn new(byte_budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            failed: HashMap::new(),
            byte_budget,
            bytes_in_use: 0,
            counter: 0,
        }
    }

    fn tick(&mut self) -> u64 {
        self.counter = self.counter.wrapping_add(1);
        self.counter
    }

    pub fn get(&mut self, key: &str) -> Option<RenderedPage> {
        let stamp = self.tick();
        let entry = self.entries.get_mut(key)?;
        entry.last_used = stamp;
        Some(entry.page.clone())
    }

    pub fn get_failed(&mut self, key: &str) -> Option<String> {
        let stamp = self.tick();
        let entry = self.failed.get_mut(key)?;
        entry.1 = stamp;
        Some(entry.0.clone())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key) || self.failed.contains_key(key)
    }

    pub fn insert(&mut self, key: String, page: RenderedPage) {
        let [w, h] = page.texture.size();
        let bytes = w * h * 4;
        let stamp = self.tick();
        if let Some(prev) = self.entries.insert(
            key,
            Entry {
                page,
                bytes,
                last_used: stamp,
            },
        ) {
            self.bytes_in_use = self.bytes_in_use.saturating_sub(prev.bytes);
        }
        self.bytes_in_use += bytes;
        self.shrink_to_budget();
    }

    pub fn insert_failed(&mut self, key: String, msg: String) {
        let stamp = self.tick();
        self.failed.insert(key, (msg, stamp));
        while self.failed.len() > FAILED_CAP {
            let oldest = self
                .failed
                .iter()
                .min_by_key(|(_, (_, ts))| *ts)
                .map(|(k, _)| k.clone());
            if let Some(k) = oldest {
                self.failed.remove(&k);
            } else {
                break;
            }
        }
    }

    /// Drop least-recently-used entries until the cache fits in budget.
    fn shrink_to_budget(&mut self) {
        let mut evicted = 0usize;
        while self.bytes_in_use > self.byte_budget && !self.entries.is_empty() {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone());
            let Some(k) = oldest else { break };
            if let Some(removed) = self.entries.remove(&k) {
                self.bytes_in_use = self.bytes_in_use.saturating_sub(removed.bytes);
                evicted += 1;
            }
        }
        if evicted > 0 {
            log::debug!(
                "render cache: evicted {} LRU entries, now {} entries, {:.1} MB",
                evicted,
                self.entries.len(),
                self.bytes_in_use as f64 / (1024.0 * 1024.0),
            );
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn bytes_in_use(&self) -> usize {
        self.bytes_in_use
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new(DEFAULT_BYTE_BUDGET)
    }
}
