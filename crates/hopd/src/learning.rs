use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Constants ---

const MAX_QUERIES: usize = 500;
const MAX_ITEMS_PER_QUERY: usize = 20;
const MAX_GLOBAL_ENTRIES: usize = 1000;

const QUERY_BOOST_PER_COUNT: i32 = 15;
const QUERY_BOOST_CAP: i32 = 150;

const FREQ_BOOST_PER_COUNT: i32 = 3;
const FREQ_BOOST_CAP: i32 = 60;

/// 30 days in milliseconds — half-life for decay.
const DECAY_HALF_MS: u64 = 30 * 24 * 60 * 60 * 1000;
/// 90 days in milliseconds — quarter-life for decay.
const DECAY_QUARTER_MS: u64 = 90 * 24 * 60 * 60 * 1000;

// --- Data types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub count: u32,
    pub last_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningStore {
    pub version: u32,
    pub selections: HashMap<String, HashMap<String, LearningEntry>>,
    pub global_frequency: HashMap<String, LearningEntry>,
    #[serde(skip)]
    path: PathBuf,
}

// --- Helper functions ---

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Apply recency decay to a raw boost value.
/// Returns full value if within half-life, halved if within quarter-life, quartered beyond.
fn apply_decay(raw: i32, last_ms: u64, now: u64) -> i32 {
    if now <= last_ms {
        return raw;
    }
    let age = now - last_ms;
    if age <= DECAY_HALF_MS {
        raw
    } else if age <= DECAY_QUARTER_MS {
        raw / 2
    } else {
        raw / 4
    }
}

/// Evict least-recently-used entries from an inner map (result_id -> LearningEntry)
/// until the map size is at most `max`.
fn evict_lru_map(map: &mut HashMap<String, LearningEntry>, max: usize) {
    while map.len() > max {
        if let Some(oldest_key) = map
            .iter()
            .min_by_key(|(_, entry)| entry.last_ms)
            .map(|(key, _)| key.clone())
        {
            map.remove(&oldest_key);
        } else {
            break;
        }
    }
}

/// Evict least-recently-used outer keys from the selections map
/// until the map size is at most `max`. The "age" of an outer key is
/// the maximum last_ms across its inner entries.
fn evict_lru_outer(map: &mut HashMap<String, HashMap<String, LearningEntry>>, max: usize) {
    while map.len() > max {
        if let Some(oldest_key) = map
            .iter()
            .map(|(key, inner)| {
                let max_ms = inner.values().map(|e| e.last_ms).max().unwrap_or(0);
                (key.clone(), max_ms)
            })
            .min_by_key(|(_, ms)| *ms)
            .map(|(key, _)| key)
        {
            map.remove(&oldest_key);
        } else {
            break;
        }
    }
}

// --- LearningStore implementation ---

impl LearningStore {
    /// Create an empty store bound to the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            version: 1,
            selections: HashMap::new(),
            global_frequency: HashMap::new(),
            path,
        }
    }

    /// Load from disk, falling back to a fresh store on any error.
    pub fn load_or_new(path: PathBuf) -> Self {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(mut store) = serde_json::from_str::<LearningStore>(&data) {
                store.path = path;
                return store;
            }
        }
        Self::new(path)
    }

    /// Record a selection: the user chose `result_id` while typing `query`.
    pub fn record(&mut self, query: &str, result_id: &str) {
        let ts = now_ms();
        let normalized = query.trim().to_lowercase();

        // Update per-query selections
        let inner = self
            .selections
            .entry(normalized)
            .or_insert_with(HashMap::new);
        let entry = inner.entry(result_id.to_string()).or_insert(LearningEntry {
            count: 0,
            last_ms: 0,
        });
        entry.count = entry.count.saturating_add(1);
        entry.last_ms = ts;

        // Evict inner map if too large
        evict_lru_map(inner, MAX_ITEMS_PER_QUERY);

        // Evict outer map if too large
        evict_lru_outer(&mut self.selections, MAX_QUERIES);

        // Update global frequency
        let global = self
            .global_frequency
            .entry(result_id.to_string())
            .or_insert(LearningEntry {
                count: 0,
                last_ms: 0,
            });
        global.count = global.count.saturating_add(1);
        global.last_ms = ts;

        // Evict global map if too large
        evict_lru_map(&mut self.global_frequency, MAX_GLOBAL_ENTRIES);
    }

    /// Persist the store to disk.
    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.path, serde_json::to_string_pretty(self).unwrap_or_default());
    }

    /// Clear all learned data and persist.
    pub fn reset(&mut self) {
        self.selections.clear();
        self.global_frequency.clear();
        self.save();
    }

    /// Compute a query-specific boost for `result_id`.
    ///
    /// Prefix matching works both ways:
    /// - A shorter stored key that is a prefix of `query` contributes.
    /// - A longer stored key that starts with `query` also contributes.
    ///
    /// The boost is count * QUERY_BOOST_PER_COUNT, with recency decay, capped at QUERY_BOOST_CAP.
    pub fn query_boost(&self, query: &str, result_id: &str) -> i32 {
        let normalized = query.trim().to_lowercase();
        if normalized.is_empty() {
            return 0;
        }
        let now = now_ms();
        let mut total: i32 = 0;

        for (stored_query, inner) in &self.selections {
            // Prefix match: either stored_query is a prefix of the current query
            // or the current query is a prefix of the stored_query.
            let is_prefix_match =
                normalized.starts_with(stored_query.as_str()) || stored_query.starts_with(&normalized);
            if !is_prefix_match {
                continue;
            }
            if let Some(entry) = inner.get(result_id) {
                let raw = (entry.count as i32).saturating_mul(QUERY_BOOST_PER_COUNT);
                total = total.saturating_add(apply_decay(raw, entry.last_ms, now));
            }
        }

        total.min(QUERY_BOOST_CAP)
    }

    /// Compute a global frequency boost for `result_id`, with recency decay, capped at FREQ_BOOST_CAP.
    pub fn frequency_boost(&self, result_id: &str) -> i32 {
        let now = now_ms();
        if let Some(entry) = self.global_frequency.get(result_id) {
            let raw = (entry.count as i32).saturating_mul(FREQ_BOOST_PER_COUNT);
            apply_decay(raw, entry.last_ms, now).min(FREQ_BOOST_CAP)
        } else {
            0
        }
    }

    /// Return the most recently launched result IDs, sorted by last_ms descending.
    pub fn recent_launches(&self, limit: usize) -> Vec<(String, u64)> {
        let mut entries: Vec<(String, u64)> = self
            .global_frequency
            .iter()
            .map(|(id, entry)| (id.clone(), entry.last_ms))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries
    }

    /// Return the most frequently launched result IDs, sorted by count descending,
    /// excluding the given IDs.
    pub fn frequent_launches(&self, limit: usize, exclude: &[String]) -> Vec<(String, u32)> {
        let mut entries: Vec<(String, u32)> = self
            .global_frequency
            .iter()
            .filter(|(id, _)| !exclude.contains(id))
            .map(|(id, entry)| (id.clone(), entry.count))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries
    }

    /// Returns true if there are no selections and no global frequency entries.
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty() && self.global_frequency.is_empty()
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn record_and_query_selection() {
        let path = PathBuf::from("/tmp/learning_test_nonexistent.json");
        let mut store = LearningStore::new(path);

        store.record("fire", "app:firefox");
        store.record("fire", "app:firefox");
        store.record("fire", "app:firewall");
        store.record("code", "app:vscode");

        // firefox was selected twice for "fire"
        let inner = store.selections.get("fire").unwrap();
        assert_eq!(inner.get("app:firefox").unwrap().count, 2);
        assert_eq!(inner.get("app:firewall").unwrap().count, 1);

        // global frequency
        assert_eq!(store.global_frequency.get("app:firefox").unwrap().count, 2);
        assert_eq!(
            store.global_frequency.get("app:firewall").unwrap().count,
            1
        );
        assert_eq!(store.global_frequency.get("app:vscode").unwrap().count, 1);

        // query_boost should be positive for a matching query/result pair
        let boost = store.query_boost("fire", "app:firefox");
        assert!(boost > 0, "expected positive boost, got {boost}");

        // frequency_boost should be positive
        let freq = store.frequency_boost("app:firefox");
        assert!(freq > 0, "expected positive freq boost, got {freq}");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");

        let mut store = LearningStore::new(path.clone());
        store.record("fire", "app:firefox");
        store.record("fire", "app:firefox");
        store.save();

        let loaded = LearningStore::load_or_new(path);
        assert_eq!(
            loaded
                .global_frequency
                .get("app:firefox")
                .unwrap()
                .count,
            2
        );
        let inner = loaded.selections.get("fire").unwrap();
        assert_eq!(inner.get("app:firefox").unwrap().count, 2);
    }

    #[test]
    fn empty_store_returns_no_boosts() {
        let path = PathBuf::from("/tmp/learning_test_empty_nonexistent.json");
        let store = LearningStore::new(path);

        assert!(store.is_empty());
        assert_eq!(store.query_boost("anything", "app:foo"), 0);
        assert_eq!(store.frequency_boost("app:foo"), 0);
        assert!(store.recent_launches(10).is_empty());
        assert!(store.frequent_launches(10, &[]).is_empty());
    }
}
