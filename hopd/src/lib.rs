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
                result: json!({
                    "results": [],
                    "telemetry": {
                        "elapsed_ms": 0,
                    }
                }),
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
