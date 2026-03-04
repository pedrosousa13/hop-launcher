use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherResult {
    pub id: String,
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub id: String,
}

pub fn default_hopd_socket_path() -> String {
    if let Ok(path) = env::var("HOPD_SOCKET") {
        return path;
    }
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return format!("{runtime_dir}/hopd.sock");
    }
    "/tmp/hopd.sock".to_string()
}

pub fn default_control_socket_path() -> String {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return format!("{runtime_dir}/hop-launcher-control.sock");
    }
    "/tmp/hop-launcher-control.sock".to_string()
}

pub fn toggle_accelerator() -> &'static str {
    "<Primary><Shift>ampersand"
}

pub fn start_visible_on_launch() -> bool {
    true
}

pub fn parse_control_request(payload: &Value) -> Result<ControlRequest, String> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing string id".to_string())?;
    let method = payload
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing string method".to_string())?;
    if method != "ui.toggle" {
        return Err(format!("unsupported method: {}", method));
    }
    Ok(ControlRequest { id: id.to_string() })
}

pub fn build_control_ok_response(id: &str) -> Value {
    json!({
        "id": id,
        "result": {
            "ok": true
        }
    })
}

pub fn build_control_error_response(id: &str, code: i32, message: &str) -> Value {
    json!({
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

pub fn build_search_payload(query: &str, limit: u32, request_id: &str) -> Value {
    json!({
        "id": request_id,
        "method": "search.query",
        "params": {
            "query": query,
            "limit": limit.max(1),
        }
    })
}

pub fn build_execute_payload(result_id: &str, request_id: &str) -> Value {
    json!({
        "id": request_id,
        "method": "actions.execute",
        "params": {
            "result_id": result_id,
            "action": "enter",
        }
    })
}

pub fn parse_search_results(response: &Value) -> Vec<LauncherResult> {
    response
        .get("result")
        .and_then(|result| result.get("results"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("id")?.as_str()?.to_string();
                    let kind = row.get("kind")?.as_str()?.to_string();
                    let title = row.get("title")?.as_str()?.to_string();
                    Some(LauncherResult { id, kind, title })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn send_ipc(socket_path: &str, payload: &Value) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("connect {} failed: {}", socket_path, error))?;

    let encoded = serde_json::to_string(payload)
        .map_err(|error| format!("encode payload failed: {}", error))?;
    stream
        .write_all(encoded.as_bytes())
        .map_err(|error| format!("write payload failed: {}", error))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("write newline failed: {}", error))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("read response failed: {}", error))?;

    serde_json::from_str(line.trim())
        .map_err(|error| format!("decode response failed: {}", error))
}

pub fn search(socket_path: &str, query: &str, limit: u32) -> Result<Vec<LauncherResult>, String> {
    let payload = build_search_payload(query, limit, "gtk-search");
    let response = send_ipc(socket_path, &payload)?;
    Ok(parse_search_results(&response))
}

pub fn execute(socket_path: &str, result_id: &str) -> Result<(), String> {
    let payload = build_execute_payload(result_id, "gtk-execute");
    let _ = send_ipc(socket_path, &payload)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_search_query_payload() {
        let payload = build_search_payload("weather zurich", 5, "gtk-1");
        assert_eq!(payload["id"], "gtk-1");
        assert_eq!(payload["method"], "search.query");
        assert_eq!(payload["params"]["query"], "weather zurich");
        assert_eq!(payload["params"]["limit"], 5);
    }

    #[test]
    fn parses_search_response_rows() {
        let raw = serde_json::json!({
            "id": "gtk-1",
            "result": {
                "results": [
                    {"id": "utility:weather", "kind": "weather", "title": "Weather"},
                    {"id": "utility:emoji", "kind": "emoji", "title": "Emoji"}
                ]
            }
        });

        let rows = parse_search_results(&raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "utility:weather");
        assert_eq!(rows[1].kind, "emoji");
    }

    #[test]
    fn builds_execute_payload() {
        let payload = build_execute_payload("utility:weather", "gtk-2");
        assert_eq!(payload["id"], "gtk-2");
        assert_eq!(payload["method"], "actions.execute");
        assert_eq!(payload["params"]["result_id"], "utility:weather");
        assert_eq!(payload["params"]["action"], "enter");
    }

    #[test]
    fn exposes_toggle_accelerator() {
        assert_eq!(toggle_accelerator(), "<Primary><Shift>ampersand");
    }

    #[test]
    fn starts_visible_for_phase1_local_testing() {
        assert!(start_visible_on_launch());
    }

    #[test]
    fn control_default_socket_path_uses_runtime_dir() {
        if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            assert_eq!(
                default_control_socket_path(),
                format!("{runtime_dir}/hop-launcher-control.sock")
            );
        }
    }

    #[test]
    fn control_parse_toggle_request_accepts_ui_toggle() {
        let payload = serde_json::json!({
            "id": "ctrl-1",
            "method": "ui.toggle"
        });

        let request = parse_control_request(&payload).expect("toggle request should parse");
        assert_eq!(request.id, "ctrl-1");
    }

    #[test]
    fn control_build_ok_response_uses_original_id() {
        let response = build_control_ok_response("ctrl-1");
        assert_eq!(response["id"], "ctrl-1");
        assert_eq!(response["result"]["ok"], true);
    }

    #[test]
    fn control_build_error_response_uses_json_rpc_shape() {
        let response = build_control_error_response("ctrl-1", -32601, "method not found");
        assert_eq!(response["id"], "ctrl-1");
        assert_eq!(response["error"]["code"], -32601);
        assert_eq!(response["error"]["message"], "method not found");
    }
}
