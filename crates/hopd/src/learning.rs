use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
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
/// Hard retention cutoff for persisted learning data.
const PERSIST_RETENTION_MS: u64 = 90 * 24 * 60 * 60 * 1000;

// --- Data types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub count: u32,
    pub last_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningStore {
    pub version: u32,
    #[serde(default, skip_serializing)]
    pub selections: HashMap<String, HashMap<String, LearningEntry>>,
    pub global_frequency: HashMap<String, LearningEntry>,
    #[serde(skip)]
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedLearningStore {
    version: u32,
    global_frequency: HashMap<String, LearningEntry>,
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
            if let Ok(persisted) = serde_json::from_str::<PersistedLearningStore>(&data) {
                let mut store = Self::new(path);
                store.version = persisted.version;
                store.global_frequency = persisted.global_frequency;
                store.purge_expired();
                return store;
            }
            if let Ok(mut store) = serde_json::from_str::<LearningStore>(&data) {
                store.path = path;
                store.selections.clear();
                store.purge_expired();
                return store;
            }
        }
        Self::new(path)
    }

    /// Record a selection: the user chose `result_id` while typing `query`.
    pub fn record(&mut self, query: &str, result_id: &str) {
        self.purge_expired();
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
    pub fn save(&mut self) {
        self.purge_expired();
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
            #[cfg(unix)]
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
        let Some(parent) = self.path.parent() else {
            return;
        };
        let Some(file_name) = self.path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        let temp_name = format!(
            ".{}.tmp-{}-{}",
            file_name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = parent.join(temp_name);
        let payload = serde_json::to_string_pretty(&PersistedLearningStore {
            version: self.version,
            global_frequency: canonicalized_global_frequency(&self.global_frequency),
        })
        .unwrap_or_default();

        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let Ok(mut file) = options.open(&temp_path) else {
            return;
        };
        if file.write_all(payload.as_bytes()).is_err() {
            let _ = fs::remove_file(&temp_path);
            return;
        }
        if file.write_all(b"\n").is_err() || file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);
            return;
        }
        if fs::rename(&temp_path, &self.path).is_err() {
            let _ = fs::remove_file(&temp_path);
            return;
        }
        #[cfg(unix)]
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600));
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

    fn purge_expired(&mut self) {
        let cutoff = now_ms().saturating_sub(PERSIST_RETENTION_MS);
        self.selections.retain(|_, inner| {
            inner.retain(|_, entry| entry.last_ms >= cutoff);
            !inner.is_empty()
        });
        self.global_frequency
            .retain(|_, entry| entry.last_ms >= cutoff);
    }
}

fn canonicalize_result_id(result_id: &str) -> String {
    if let Some(utility_tail) = result_id.strip_prefix("utility:") {
        let utility_kind = utility_tail.split(':').next().unwrap_or_default();
        if !utility_kind.is_empty() {
            return format!("utility:{utility_kind}");
        }
    }
    if let Some(web_tail) = result_id.strip_prefix("web-search:") {
        let service = web_tail.split(':').next().unwrap_or_default();
        if !service.is_empty() {
            return format!("web-search:{service}");
        }
    }
    result_id.to_string()
}

fn canonicalized_global_frequency(
    input: &HashMap<String, LearningEntry>,
) -> HashMap<String, LearningEntry> {
    let mut out: HashMap<String, LearningEntry> = HashMap::new();
    for (id, entry) in input {
        let key = canonicalize_result_id(id);
        let aggregate = out.entry(key).or_insert(LearningEntry {
            count: 0,
            last_ms: 0,
        });
        aggregate.count = aggregate.count.saturating_add(entry.count);
        aggregate.last_ms = aggregate.last_ms.max(entry.last_ms);
    }
    out
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
    fn save_and_load_round_trip_without_persisting_query_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");

        let mut store = LearningStore::new(path.clone());
        store.record("fire", "app:firefox");
        store.record("fire", "app:firefox");
        store.save();

        let saved = std::fs::read_to_string(&path).expect("saved learning file");
        assert!(
            !saved.contains("\"fire\""),
            "raw query keys should not be persisted"
        );

        let loaded = LearningStore::load_or_new(path);
        assert_eq!(
            loaded
                .global_frequency
                .get("app:firefox")
                .unwrap()
                .count,
            2
        );
        assert!(
            loaded.selections.is_empty(),
            "query selections should remain in-memory only after reload"
        );
    }

    #[test]
    fn canonicalizes_dynamic_result_ids_for_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");

        let mut store = LearningStore::new(path.clone());
        store.record(
            "rust docs",
            "web-search:duckduckgo:https%3A%2F%2Fduckduckgo.com%2F%3Fq%3Drust%2Bdocs",
        );
        store.record("2+2", "utility:calculator:2+2");
        store.save();

        let loaded = LearningStore::load_or_new(path);
        assert!(
            loaded.global_frequency.contains_key("web-search:duckduckgo"),
            "web-search ids should strip query payloads before persistence"
        );
        assert!(
            loaded.global_frequency.contains_key("utility:calculator"),
            "utility ids should strip dynamic suffixes before persistence"
        );
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

    #[test]
    fn lru_eviction_respects_max_queries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");
        let mut store = LearningStore::new(path);

        for i in 0..510 {
            store.record(&format!("query{i}"), "some.desktop");
        }
        assert!(store.selections.len() <= 500, "selections should be capped at MAX_QUERIES");
    }

    #[test]
    fn prefix_matching_boosts_across_query_lengths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");
        let mut store = LearningStore::new(path);

        store.record("firefox", "firefox.desktop");
        store.record("firefox", "firefox.desktop");
        store.record("firefox", "firefox.desktop");

        // "fi" should match "firefox" via prefix matching
        let boost = store.query_boost("fi", "firefox.desktop");
        assert!(boost > 0, "prefix 'fi' should match learning for 'firefox'");

        // "firefox browser" should match "firefox" too (starts_with)
        let boost2 = store.query_boost("firefox browser", "firefox.desktop");
        assert!(boost2 > 0, "longer query should match stored shorter key");
    }

    #[test]
    fn recent_launches_sorted_by_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");
        let mut store = LearningStore::new(path);

        store.record("a", "first.desktop");
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.record("b", "second.desktop");

        let recent = store.recent_launches(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].0, "second.desktop", "most recent should be first");
        assert_eq!(recent[1].0, "first.desktop");
    }

    #[test]
    fn frequent_launches_excludes_specified_ids() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");
        let mut store = LearningStore::new(path);

        for _ in 0..5 { store.record("a", "popular.desktop"); }
        for _ in 0..2 { store.record("b", "other.desktop"); }

        let frequent = store.frequent_launches(10, &["popular.desktop".to_string()]);
        assert!(frequent.iter().all(|(id, _)| id != "popular.desktop"), "excluded IDs should not appear");
        assert!(!frequent.is_empty(), "should still have other entries");
    }

    #[test]
    fn reset_clears_all_data_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learning.json");
        let mut store = LearningStore::new(path.clone());

        store.record("test", "app.desktop");
        assert!(!store.is_empty());

        store.reset();
        assert!(store.is_empty());

        // Verify reset persisted
        let loaded = LearningStore::load_or_new(path);
        assert!(loaded.is_empty());
    }
}
