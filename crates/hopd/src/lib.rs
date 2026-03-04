use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

pub mod kde_adapter;

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
            "search.query" => IpcResponse {
                id: request.id,
                result: json!(build_search_result(&request.params)),
                error: None,
            },
            "actions.execute" => IpcResponse {
                id: request.id,
                result: json!({
                    "ok": true,
                    "executed": true,
                }),
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

fn build_search_result(params: &Value) -> Value {
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

    let mut matches: Vec<(i32, SearchItem)> = aggregate_provider_items(&query)
        .into_iter()
        .filter_map(|item| {
            let score = score_item(&query, &item);
            if score > 0 {
                Some((score, item))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|left, right| right.0.cmp(&left.0));
    let results: Vec<Value> = matches
        .into_iter()
        .take(limit.max(1))
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

    json!({
        "results": results,
        "telemetry": {
            "elapsed_ms": 0,
        }
    })
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

fn aggregate_provider_items(query: &str) -> Vec<SearchItem> {
    let mut items = Vec::new();
    items.extend(weather_provider(query));
    items.extend(timezone_provider(query));
    items.extend(emoji_provider(query));
    if items.is_empty() && query.chars().count() <= 2 {
        items.extend(default_catalog_items());
    }
    items
}

fn score_item(query: &str, item: &SearchItem) -> i32 {
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

    score + kind_priority(item.kind.as_str())
}

fn kind_priority(kind: &str) -> i32 {
    match kind {
        "weather" => 30,
        "timezone" => 20,
        "emoji" => 10,
        _ => 0,
    }
}

fn weather_provider(query: &str) -> Vec<SearchItem> {
    if query.is_empty() {
        return Vec::new();
    }

    let tokens: Vec<&str> = query.split_whitespace().collect();
    let has_weather_intent = tokens.iter().any(|token| *token == "weather" || *token == "wx");
    if !has_weather_intent {
        return Vec::new();
    }

    let blocking = tokens
        .iter()
        .any(|token| !["weather", "wx", "emoji", "time", "tz"].contains(token));
    if blocking {
        return Vec::new();
    }

    vec![SearchItem {
        id: "utility:weather".to_string(),
        kind: "weather".to_string(),
        title: "Weather".to_string(),
        subtitle: "Utility".to_string(),
        icon: "weather-clear-symbolic".to_string(),
        keywords: "weather forecast temperature".to_string(),
    }]
}

fn timezone_provider(query: &str) -> Vec<SearchItem> {
    if query.is_empty() {
        return Vec::new();
    }

    let lower = query.to_lowercase();
    let city = if lower.contains("tokyo") {
        Some("Tokyo")
    } else if lower.contains("zurich") {
        Some("Zurich")
    } else if lower.contains("berlin") {
        Some("Berlin")
    } else {
        None
    };

    let has_time_intent = lower.contains("time ") || lower.starts_with("time")
        || lower.contains("tz ") || lower.starts_with("tz");
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

fn default_catalog_items() -> Vec<SearchItem> {
    vec![
        SearchItem {
            id: "utility:weather".to_string(),
            kind: "weather".to_string(),
            title: "Weather".to_string(),
            subtitle: "Utility".to_string(),
            icon: "weather-clear-symbolic".to_string(),
            keywords: "weather forecast temperature".to_string(),
        },
        SearchItem {
            id: "utility:timezone".to_string(),
            kind: "timezone".to_string(),
            title: "Timezone".to_string(),
            subtitle: "Utility".to_string(),
            icon: "preferences-system-time-symbolic".to_string(),
            keywords: "timezone world clock time".to_string(),
        },
        SearchItem {
            id: "utility:emoji".to_string(),
            kind: "emoji".to_string(),
            title: "Emoji".to_string(),
            subtitle: "Utility".to_string(),
            icon: "face-smile-symbolic".to_string(),
            keywords: "emoji picker symbols".to_string(),
        },
    ]
}
