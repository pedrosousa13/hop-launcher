use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

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
}

impl HopdServer {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_json_line(&self, line: &str) -> Result<String, serde_json::Error> {
        let request: IpcRequest = serde_json::from_str(line)?;
        let response = match request.method.as_str() {
            "health.ping" => IpcResponse {
                id: request.id,
                result: json!({"ok": true}),
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

    let mut matches: Vec<(i32, Value)> = catalog_items()
        .into_iter()
        .filter_map(|item| {
            let title = item["title"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase();
            let keywords = item["keywords"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase();

            if query.is_empty() || title.contains(&query) || keywords.contains(&query) {
                let score = score_item(&query, &title, &keywords, item["kind"].as_str().unwrap_or_default());
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
        .map(|(_, item)| {
            json!({
                "id": item["id"],
                "kind": item["kind"],
                "title": item["title"],
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

fn score_item(query: &str, title: &str, keywords: &str, kind: &str) -> i32 {
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
    score + kind_priority(kind)
}

fn kind_priority(kind: &str) -> i32 {
    match kind {
        "weather" => 30,
        "timezone" => 20,
        "emoji" => 10,
        _ => 0,
    }
}

fn catalog_items() -> Vec<Value> {
    vec![
        json!({
            "id": "utility:weather",
            "kind": "weather",
            "title": "Weather",
            "keywords": "weather forecast temperature"
        }),
        json!({
            "id": "utility:timezone",
            "kind": "timezone",
            "title": "Timezone",
            "keywords": "timezone world clock time"
        }),
        json!({
            "id": "utility:emoji",
            "kind": "emoji",
            "title": "Emoji",
            "keywords": "emoji picker symbols"
        }),
    ]
}
