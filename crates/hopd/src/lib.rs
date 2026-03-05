use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

mod actions;
pub mod kde_adapter;
mod providers;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default = "default_params")]
    pub params: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcResponse {
    pub id: String,
    pub result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

#[derive(Debug, Default)]
pub struct HopdServer {
    config: RwLock<HashMap<String, Value>>,
    total_requests: AtomicU64,
}

impl HopdServer {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(HashMap::new()),
            total_requests: AtomicU64::new(0),
        }
    }

    pub async fn handle_json_line(&self, line: &str) -> Result<String, serde_json::Error> {
        let request: IpcRequest = serde_json::from_str(line)?;
        if request.method != "metrics.snapshot" {
            self.total_requests.fetch_add(1, Ordering::Relaxed);
        }
        let response = match request.method.as_str() {
            "health.ping" => IpcResponse {
                id: request.id,
                result: json!({
                    "ok": true,
                    "service": "hopd",
                    "protocol_version": 1
                }),
                error: None,
            },
            "search.query" => {
                let config = self.config.read().await.clone();
                IpcResponse {
                    id: request.id,
                    result: json!(build_search_result(&request.params, &config)),
                    error: None,
                }
            }
            "actions.execute" => IpcResponse {
                id: request.id,
                result: actions::execute(&request.params),
                error: None,
            },
            "config.set" => {
                let key = request
                    .params
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let value = request.params.get("value").cloned().unwrap_or(Value::Null);
                self.config.write().await.insert(key, value);

                IpcResponse {
                    id: request.id,
                    result: json!({"ok": true}),
                    error: None,
                }
            }
            "config.get" => {
                let key = request
                    .params
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let value = self
                    .config
                    .read()
                    .await
                    .get(&key)
                    .cloned()
                    .unwrap_or(Value::Null);

                IpcResponse {
                    id: request.id,
                    result: json!({"value": value}),
                    error: None,
                }
            }
            "metrics.snapshot" => IpcResponse {
                id: request.id,
                result: json!({
                    "total_requests": self.total_requests.load(Ordering::Relaxed),
                }),
                error: None,
            },
            _ => IpcResponse {
                id: request.id,
                result: Value::Null,
                error: Some(IpcError {
                    code: -32601,
                    message: "method not found".to_string(),
                }),
            },
        };

        serde_json::to_string(&response)
    }
}

fn default_params() -> Value {
    json!({})
}

fn build_search_result(params: &Value, config: &HashMap<String, Value>) -> Value {
    let started_at = Instant::now();
    let query = params
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(8);
    let mode = params
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("all");

    let rank = RankSettings::from_config(config);
    let features = FeatureSettings::from_config(config);
    let indexed_folders = indexed_folders_from_config(config);
    let mut matches: Vec<(i32, SearchItem)> = aggregate_provider_items(&query, mode, &indexed_folders)
        .into_iter()
        .filter(|item| features.is_enabled(&item.kind))
        .filter_map(|item| {
            let score = score_item(&query, &item, &rank);
            let is_match = if query.is_empty() {
                score > 0
            } else {
                score >= rank.min_fuzzy_score
            };
            if is_match {
                Some((score, item))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|left, right| right.0.cmp(&left.0));
    let ordered = diversify_matches(matches, limit.max(1));
    let results: Vec<Value> = ordered
        .into_iter()
        .map(|(score, item)| {
            json!({
                "id": item.id,
                "kind": item.kind,
                "title": item.title,
                "subtitle": item.subtitle,
                "icon": item.icon,
                "primary_action": "enter",
                "score": score,
            })
        })
        .collect();
    let elapsed_ms = started_at.elapsed().as_millis() as u64;

    json!({
        "results": results,
        "telemetry": {
            "elapsed_ms": elapsed_ms,
        }
    })
}

#[derive(Debug, Clone, Copy)]
struct FeatureSettings {
    apps: bool,
    windows: bool,
    files: bool,
    recents: bool,
    settings: bool,
    utility: bool,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            apps: true,
            windows: true,
            files: true,
            recents: true,
            settings: true,
            utility: true,
        }
    }
}

impl FeatureSettings {
    fn from_config(config: &HashMap<String, Value>) -> Self {
        let default = Self::default();
        Self {
            apps: config_bool(config, "features.apps", default.apps),
            windows: config_bool(config, "features.windows", default.windows),
            files: config_bool(config, "features.files", default.files),
            recents: config_bool(config, "features.recents", default.recents),
            settings: config_bool(config, "features.settings", default.settings),
            utility: config_bool(config, "features.utility", default.utility),
        }
    }

    fn is_enabled(self, kind: &str) -> bool {
        match kind {
            "app" => self.apps,
            "window" => self.windows,
            "file" => self.files,
            "recent" => self.recents,
            "setting" => self.settings,
            "utility" | "emoji" | "calculator" | "currency" | "weather" | "timezone" => {
                self.utility
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RankSettings {
    weight_windows: i32,
    weight_apps: i32,
    weight_recents: i32,
    weight_files: i32,
    weight_emoji: i32,
    weight_utility: i32,
    min_fuzzy_score: i32,
}

impl Default for RankSettings {
    fn default() -> Self {
        Self {
            weight_windows: 30,
            weight_apps: 20,
            weight_recents: 10,
            weight_files: 12,
            weight_emoji: 8,
            weight_utility: 6,
            min_fuzzy_score: 30,
        }
    }
}

impl RankSettings {
    fn from_config(config: &HashMap<String, Value>) -> Self {
        let default = Self::default();
        Self {
            weight_windows: config_int(config, "ranking.weight_windows", default.weight_windows, -200, 200),
            weight_apps: config_int(config, "ranking.weight_apps", default.weight_apps, -200, 200),
            weight_recents: config_int(config, "ranking.weight_recents", default.weight_recents, -200, 200),
            weight_files: config_int(config, "ranking.weight_files", default.weight_files, -200, 200),
            weight_emoji: config_int(config, "ranking.weight_emoji", default.weight_emoji, -200, 200),
            weight_utility: config_int(config, "ranking.weight_utility", default.weight_utility, -200, 200),
            min_fuzzy_score: config_int(
                config,
                "ranking.min_fuzzy_score",
                default.min_fuzzy_score,
                0,
                400,
            ),
        }
    }
}

fn config_int(
    config: &HashMap<String, Value>,
    key: &str,
    fallback: i32,
    min: i32,
    max: i32,
) -> i32 {
    config
        .get(key)
        .and_then(Value::as_i64)
        .map(|value| value.clamp(min as i64, max as i64) as i32)
        .unwrap_or(fallback)
}

fn config_bool(config: &HashMap<String, Value>, key: &str, fallback: bool) -> bool {
    config
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn diversify_matches(matches: Vec<(i32, SearchItem)>, limit: usize) -> Vec<(i32, SearchItem)> {
    if matches.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut selected: Vec<(i32, SearchItem)> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_kinds: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (score, item) in &matches {
        if selected.len() >= limit {
            break;
        }
        if seen_kinds.insert(item.kind.clone()) {
            seen_ids.insert(item.id.clone());
            selected.push((*score, item.clone()));
        }
    }

    for (score, item) in matches {
        if selected.len() >= limit {
            break;
        }
        if seen_ids.insert(item.id.clone()) {
            selected.push((score, item));
        }
    }

    selected
}

#[derive(Debug, Clone)]
struct SearchItem {
    id: String,
    kind: String,
    title: String,
    subtitle: String,
    icon: String,
    keywords: String,
}

impl SearchItem {
    fn new(id: &str, kind: &str, title: &str, subtitle: &str, icon: &str, keywords: &str) -> Self {
        Self {
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            icon: icon.to_string(),
            keywords: keywords.to_string(),
        }
    }
}

fn aggregate_provider_items(query: &str, mode: &str, indexed_folders: &[String]) -> Vec<SearchItem> {
    let mut items = providers::collect_provider_items(query, mode, indexed_folders);
    match mode {
        "calculator" => items.extend(calculator_provider(query)),
        "currency" => items.extend(currency_provider(query)),
        "weather" => items.extend(weather_provider(query)),
        "timezone" => items.extend(timezone_provider(query)),
        "emoji" => items.extend(emoji_provider(query)),
        "apps" | "windows" | "files" | "recents" | "settings" => {}
        _ => {
            items.extend(calculator_provider(query));
            items.extend(currency_provider(query));
            items.extend(weather_provider(query));
            items.extend(timezone_provider(query));
            items.extend(emoji_provider(query));
        }
    }
    items
}

fn indexed_folders_from_config(config: &HashMap<String, Value>) -> Vec<String> {
    config
        .get("search.indexed_folders")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn score_item(query: &str, item: &SearchItem, rank: &RankSettings) -> i32 {
    let title = item.title.to_lowercase();
    let keywords = item.keywords.to_lowercase();
    let mut score = 0;
    if !query.is_empty() && title == query {
        score += 300;
    }
    if title.contains(query) {
        score += 100;
    }
    if keywords.contains(query) {
        score += 70;
    }
    if query.is_empty() {
        score += 10;
    }
    for token in query.split_whitespace() {
        if title.contains(token) {
            score += 60;
        } else if keywords.contains(token) {
            score += 40;
        }
    }

    if !query.is_empty() && score <= 0 {
        return 0;
    }

    score + kind_priority(item.kind.as_str(), rank)
}

fn kind_priority(kind: &str, rank: &RankSettings) -> i32 {
    match kind {
        "window" => rank.weight_windows,
        "app" => rank.weight_apps,
        "recent" => rank.weight_recents,
        "file" => rank.weight_files,
        "emoji" => rank.weight_emoji,
        "calculator" | "currency" | "weather" | "timezone" | "utility" => rank.weight_utility,
        _ => 0,
    }
}

fn looks_like_math(query: &str) -> bool {
    let trimmed = query.trim();
    !trimmed.is_empty()
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || "+-*/(). %".contains(ch))
}

fn parse_currency_query(query: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.len() != 4 || parts[2].to_lowercase() != "to" {
        return None;
    }
    let amount = parts[0];
    let src = parts[1];
    let dst = parts[3];
    let amount_ok = amount
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch == '.')
        && amount.chars().any(|ch| ch.is_ascii_digit());
    let src_ok = src.chars().all(|ch| ch.is_ascii_alphabetic()) && src.len() == 3;
    let dst_ok = dst.chars().all(|ch| ch.is_ascii_alphabetic()) && dst.len() == 3;
    if !amount_ok || !src_ok || !dst_ok {
        return None;
    }
    Some((
        amount.to_string(),
        src.to_uppercase(),
        dst.to_uppercase(),
    ))
}

fn calculator_provider(query: &str) -> Vec<SearchItem> {
    if !looks_like_math(query) {
        return Vec::new();
    }
    let expression = query.trim();
    vec![SearchItem {
        id: format!("utility:calculator:{expression}"),
        kind: "calculator".to_string(),
        title: format!("Calculate {expression}"),
        subtitle: "Utility".to_string(),
        icon: "accessories-calculator-symbolic".to_string(),
        keywords: "calculator math arithmetic expression".to_string(),
    }]
}

fn currency_provider(query: &str) -> Vec<SearchItem> {
    let Some((amount, src, dst)) = parse_currency_query(query) else {
        return Vec::new();
    };
    vec![SearchItem {
        id: format!("utility:currency:{amount}:{src}:{dst}"),
        kind: "currency".to_string(),
        title: format!("{amount} {src} -> {dst}"),
        subtitle: "Currency conversion".to_string(),
        icon: "accessories-calculator-symbolic".to_string(),
        keywords: "currency exchange convert forex".to_string(),
    }]
}

fn weather_provider(query: &str) -> Vec<SearchItem> {
    if query.is_empty() {
        return Vec::new();
    }

    let lower = query.to_lowercase();
    let location = extract_weather_location(&lower);
    let has_weather_intent = lower.contains("weather") || lower.starts_with("wx ");
    if !has_weather_intent && location.is_none() {
        return Vec::new();
    }
    let weather_id = location
        .as_ref()
        .map(|value| format!("utility:weather:{}", encode_component(value)))
        .unwrap_or_else(|| "utility:weather".to_string());
    let weather_title = location
        .as_ref()
        .map(|value| format!("Weather in {value}"))
        .unwrap_or_else(|| "Weather".to_string());
    let weather_keywords = location
        .as_ref()
        .map(|value| format!("weather forecast temperature {}", value.to_lowercase()))
        .unwrap_or_else(|| "weather forecast temperature".to_string());

    vec![SearchItem {
        id: weather_id,
        kind: "weather".to_string(),
        title: weather_title,
        subtitle: "Utility".to_string(),
        icon: "weather-clear-symbolic".to_string(),
        keywords: weather_keywords,
    }]
}

fn timezone_provider(query: &str) -> Vec<SearchItem> {
    if query.is_empty() {
        return Vec::new();
    }

    let lower = query.to_lowercase();
    let city = extract_time_location(&lower);

    let has_time_intent = lower.contains("time ") || lower.starts_with("time")
        || lower.contains("tz ")
        || lower.starts_with("tz")
        || lower.contains("timezone")
        || lower.ends_with(" time");
    if !has_time_intent && city.is_none() {
        return Vec::new();
    }
    if city.is_some() && lower.contains("weather") && !has_time_intent {
        return Vec::new();
    }

    let title = city
        .map(|name| format!("Time in {}", name))
        .unwrap_or_else(|| "Timezone".to_string());
    vec![SearchItem {
        id: "utility:timezone".to_string(),
        kind: "timezone".to_string(),
        title,
        subtitle: "Utility".to_string(),
        icon: "preferences-system-time-symbolic".to_string(),
        keywords: "timezone world clock time".to_string(),
    }]
}

fn extract_weather_location(query: &str) -> Option<String> {
    if let Some(rest) = query.strip_prefix("weather ") {
        return to_title_case(rest);
    }
    if let Some(rest) = query.strip_prefix("wx ") {
        return to_title_case(rest);
    }
    if let Some(rest) = query.strip_suffix(" weather") {
        return to_title_case(rest);
    }
    None
}

fn extract_time_location(query: &str) -> Option<String> {
    for prefix in ["time in ", "time ", "tz ", "timezone "] {
        if let Some(rest) = query.strip_prefix(prefix) {
            return to_title_case(rest);
        }
    }
    if let Some(rest) = query.strip_suffix(" time") {
        return to_title_case(rest);
    }
    None
}

fn to_title_case(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (index, token) in trimmed.split_whitespace().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let mut chars = token.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn encode_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else if ch == ' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn emoji_provider(query: &str) -> Vec<SearchItem> {
    if query.is_empty() {
        return Vec::new();
    }

    let lower = query.to_lowercase();
    let has_emoji_intent = lower.contains("emoji")
        || lower.starts_with(":")
        || lower.contains("smile")
        || lower.contains("grin");
    if !has_emoji_intent {
        return Vec::new();
    }

    vec![SearchItem {
        id: "utility:emoji".to_string(),
        kind: "emoji".to_string(),
        title: "Emoji".to_string(),
        subtitle: "Utility".to_string(),
        icon: "face-smile-symbolic".to_string(),
        keywords: "emoji picker symbols smile grin".to_string(),
    }]
}
