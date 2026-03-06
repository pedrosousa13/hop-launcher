use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherResult {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryState {
    Ready,
    Searching,
    Results { count: usize },
    Empty,
    Error(String),
    Executed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub id: String,
    pub method: ControlMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMethod {
    Toggle,
    Ping,
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

pub fn settings_accelerators() -> &'static [&'static str] {
    &["<Primary>comma", "<Super>comma", "<Meta>comma"]
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
    let method = match method {
        "ui.toggle" => ControlMethod::Toggle,
        "ui.ping" => ControlMethod::Ping,
        other => return Err(format!("unsupported method: {}", other)),
    };
    Ok(ControlRequest {
        id: id.to_string(),
        method,
    })
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
    let route = extract_query_route(query);
    json!({
        "id": request_id,
        "method": "search.query",
        "params": {
            "query": route.query,
            "mode": route.mode,
            "limit": limit.max(1),
        }
    })
}

pub fn search_query_mode(query: &str) -> String {
    extract_query_route(query).mode
}

pub fn build_execute_payload(result_id: &str, request_id: &str) -> Value {
    build_execute_payload_with_action(result_id, request_id, "enter")
}

pub fn build_execute_payload_with_action(result_id: &str, request_id: &str, action: &str) -> Value {
    json!({
        "id": request_id,
        "method": "actions.execute",
        "params": {
            "result_id": result_id,
            "action": action,
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
                    let subtitle = row
                        .get("subtitle")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let icon = row
                        .get("icon")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    Some(LauncherResult {
                        id,
                        kind,
                        title,
                        subtitle,
                        icon,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_execute_response(response: &Value) -> Result<(), String> {
    let result = response
        .get("result")
        .ok_or_else(|| "missing result".to_string())?;

    let resolved = result
        .get("action_resolved")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let spawned = result
        .get("launch_spawned")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let success = result.get("success").and_then(Value::as_bool);

    if success.unwrap_or(resolved && spawned) {
        return Ok(());
    }

    if let Some(message) = result.get("error_message").and_then(Value::as_str) {
        return Err(message.to_string());
    }

    let status = result
        .get("execution_status")
        .and_then(Value::as_str)
        .unwrap_or("execution failed");
    Err(status.to_string())
}

pub fn render_status_text(state: QueryState) -> String {
    match state {
        QueryState::Ready => String::new(),
        QueryState::Searching => "Searching...".to_string(),
        QueryState::Results { .. } => String::new(),
        QueryState::Empty => String::new(),
        QueryState::Error(message) => format!("Error: {message}"),
        QueryState::Executed => String::new(),
    }
}

pub fn selected_result_id(results: &[LauncherResult], index: usize) -> Option<&str> {
    results.get(index).map(|row| row.id.as_str())
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
    execute_with_action(socket_path, result_id, "enter")
}

pub fn execute_with_action(socket_path: &str, result_id: &str, action: &str) -> Result<(), String> {
    let payload = build_execute_payload_with_action(result_id, "gtk-execute", action);
    let response = send_ipc(socket_path, &payload)?;
    parse_execute_response(&response)
}

pub fn config_set(socket_path: &str, key: &str, value: Value) -> Result<(), String> {
    let payload = json!({
        "id": "gtk-config-set",
        "method": "config.set",
        "params": {
            "key": key,
            "value": value,
        }
    });
    let response = send_ipc(socket_path, &payload)?;
    let ok = response
        .get("result")
        .and_then(|result| result.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("config.set returned non-ok response".to_string())
    }
}

pub fn learning_record(socket_path: &str, query: &str, result_id: &str) -> Result<(), String> {
    let payload = json!({
        "id": "gtk-learning-record",
        "method": "learning.record",
        "params": {
            "query": query,
            "result_id": result_id,
        }
    });
    let response = send_ipc(socket_path, &payload)?;
    let ok = response
        .get("result")
        .and_then(|result| result.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("learning.record returned non-ok response".to_string())
    }
}

pub fn learning_reset(socket_path: &str) -> Result<(), String> {
    let payload = json!({
        "id": "gtk-learning-reset",
        "method": "learning.reset",
    });
    let response = send_ipc(socket_path, &payload)?;
    let ok = response
        .get("result")
        .and_then(|result| result.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("learning.reset returned non-ok response".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryRoute {
    mode: String,
    query: String,
}

fn looks_like_math(query: &str) -> bool {
    let trimmed = query.trim();
    !trimmed.is_empty()
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || "+-*/(). %".contains(ch))
}

fn looks_like_currency(query: &str) -> bool {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.len() != 4 || parts[2].to_lowercase() != "to" {
        return false;
    }
    let left_ok = parts[0].chars().all(|ch| ch.is_ascii_digit() || ch == '.');
    let src_ok = parts[1].chars().all(|ch| ch.is_ascii_alphabetic()) && parts[1].len() == 3;
    let dst_ok = parts[3].chars().all(|ch| ch.is_ascii_alphabetic()) && parts[3].len() == 3;
    left_ok && src_ok && dst_ok
}

fn extract_query_route(raw_query: &str) -> QueryRoute {
    let trimmed = raw_query.trim_start();
    let lowered = trimmed.to_lowercase();

    let route = if lowered.starts_with("w ") {
        ("weather", trimmed[2..].to_string())
    } else if lowered.starts_with("a ") {
        ("apps", trimmed[2..].to_string())
    } else if lowered.starts_with("win ") {
        ("windows", trimmed[4..].to_string())
    } else if lowered.starts_with("windows ") {
        ("windows", trimmed[8..].to_string())
    } else if lowered.starts_with("f ") {
        ("files", trimmed[2..].to_string())
    } else if lowered.starts_with("r ") {
        ("recents", trimmed[2..].to_string())
    } else if lowered.starts_with("settings ") {
        ("settings", trimmed[9..].to_string())
    } else if lowered.starts_with("prefs ") {
        ("settings", trimmed[6..].to_string())
    } else if lowered.starts_with(":emoji ") {
        ("emoji", trimmed[7..].to_string())
    } else if lowered.starts_with("e ") {
        ("emoji", trimmed[2..].to_string())
    } else if lowered.starts_with("emoji ") {
        ("emoji", trimmed[6..].to_string())
    } else if lowered.starts_with("t ") {
        ("timezone", trimmed[2..].to_string())
    } else if lowered.starts_with("tz ") {
        ("timezone", trimmed[3..].to_string())
    } else if lowered.starts_with("timezone ") {
        ("timezone", trimmed[9..].to_string())
    } else if lowered.starts_with("time in ") {
        ("timezone", trimmed[8..].to_string())
    } else if lowered.starts_with("time ") {
        ("timezone", trimmed[5..].to_string())
    } else if lowered.starts_with("weather ") {
        ("weather", trimmed[8..].to_string())
    } else if lowered.starts_with("wx ") {
        ("weather", trimmed[3..].to_string())
    } else if lowered.starts_with("c ") {
        ("calculator", trimmed[2..].to_string())
    } else if lowered.starts_with("calc ") {
        ("calculator", trimmed[5..].to_string())
    } else if lowered.starts_with("calculator ") {
        ("calculator", trimmed[11..].to_string())
    } else if lowered.starts_with("x ") {
        ("currency", trimmed[2..].to_string())
    } else if lowered.starts_with("currency ") {
        ("currency", trimmed[9..].to_string())
    } else if lowered.starts_with("fx ") {
        ("currency", trimmed[3..].to_string())
    } else if lowered.ends_with(" weather") && trimmed.len() > 8 {
        ("weather", trimmed[..trimmed.len() - 8].trim().to_string())
    } else if looks_like_math(trimmed) {
        ("all", trimmed.to_string())
    } else if looks_like_currency(trimmed) {
        ("all", trimmed.to_string())
    } else {
        ("all", trimmed.to_string())
    };

    QueryRoute {
        mode: route.0.to_string(),
        query: route.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_search_query_payload() {
        let payload = build_search_payload("weather zurich", 5, "gtk-1");
        assert_eq!(payload["id"], "gtk-1");
        assert_eq!(payload["method"], "search.query");
        assert_eq!(payload["params"]["query"], "zurich");
        assert_eq!(payload["params"]["mode"], "weather");
        assert_eq!(payload["params"]["limit"], 5);
    }

    #[test]
    fn route_prefix_w_maps_to_weather_mode() {
        let route = extract_query_route("w zurich");
        assert_eq!(route.mode, "weather");
        assert_eq!(route.query, "zurich");
    }

    #[test]
    fn route_prefix_win_maps_to_windows_mode() {
        let route = extract_query_route("win terminal");
        assert_eq!(route.mode, "windows");
        assert_eq!(route.query, "terminal");
    }

    #[test]
    fn route_prefix_a_maps_to_apps_mode() {
        let route = extract_query_route("a firefox");
        assert_eq!(route.mode, "apps");
        assert_eq!(route.query, "firefox");
    }

    #[test]
    fn route_settings_keyword_maps_to_settings_mode() {
        let route = extract_query_route("settings bluetooth");
        assert_eq!(route.mode, "settings");
        assert_eq!(route.query, "bluetooth");
    }

    #[test]
    fn route_default_keeps_all_mode() {
        let route = extract_query_route("firefox");
        assert_eq!(route.mode, "all");
        assert_eq!(route.query, "firefox");
    }

    #[test]
    fn search_query_mode_reports_files_prefix() {
        assert_eq!(search_query_mode("f report"), "files");
    }

    #[test]
    fn route_calc_prefix_maps_to_calculator_mode() {
        let route = extract_query_route("calc 2+2");
        assert_eq!(route.mode, "calculator");
        assert_eq!(route.query, "2+2");
    }

    #[test]
    fn route_currency_prefix_maps_to_currency_mode() {
        let route = extract_query_route("currency 12 usd to chf");
        assert_eq!(route.mode, "currency");
        assert_eq!(route.query, "12 usd to chf");
    }

    #[test]
    fn route_single_letter_utility_prefixes_map_to_utility_modes() {
        assert_eq!(extract_query_route("t zurich").mode, "timezone");
        assert_eq!(extract_query_route("e smile").mode, "emoji");
        assert_eq!(extract_query_route("c 2+2").mode, "calculator");
        assert_eq!(extract_query_route("x 12 usd to chf").mode, "currency");
    }

    #[test]
    fn parses_search_response_rows() {
        let raw = serde_json::json!({
            "id": "gtk-1",
            "result": {
                "results": [
                    {"id": "utility:weather", "kind": "weather", "title": "Weather", "subtitle":"Utility", "icon":"weather-clear-symbolic"},
                    {"id": "utility:emoji", "kind": "emoji", "title": "Emoji", "subtitle":"Utility", "icon":"face-smile-symbolic"}
                ]
            }
        });

        let rows = parse_search_results(&raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "utility:weather");
        assert_eq!(rows[1].kind, "emoji");
        assert_eq!(rows[0].subtitle, "Utility");
        assert_eq!(rows[1].icon, "face-smile-symbolic");
    }

    #[test]
    fn parses_normalized_row_metadata() {
        let raw = serde_json::json!({
            "id":"gtk-1",
            "result":{"results":[{
                "id":"app:terminal",
                "kind":"app",
                "title":"Terminal",
                "subtitle":"System app",
                "icon":"utilities-terminal-symbolic",
                "primary_action":"enter",
                "score":220
            }]}
        });

        let rows = parse_search_results(&raw);
        assert_eq!(rows[0].subtitle, "System app");
        assert_eq!(rows[0].icon, "utilities-terminal-symbolic");
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
    fn builds_execute_payload_with_copy_action() {
        let payload = build_execute_payload_with_action("utility:calculator:2%2B2", "gtk-3", "copy");
        assert_eq!(payload["id"], "gtk-3");
        assert_eq!(payload["method"], "actions.execute");
        assert_eq!(payload["params"]["result_id"], "utility:calculator:2%2B2");
        assert_eq!(payload["params"]["action"], "copy");
    }

    #[test]
    fn exposes_toggle_accelerator() {
        assert_eq!(toggle_accelerator(), "<Primary><Shift>ampersand");
    }

    #[test]
    fn exposes_settings_accelerators() {
        assert_eq!(
            settings_accelerators(),
            &["<Primary>comma", "<Super>comma", "<Meta>comma"]
        );
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
        assert_eq!(request.method, ControlMethod::Toggle);
    }

    #[test]
    fn control_parse_ping_request_accepts_ui_ping() {
        let payload = serde_json::json!({
            "id": "ctrl-2",
            "method": "ui.ping"
        });

        let request = parse_control_request(&payload).expect("ping request should parse");
        assert_eq!(request.id, "ctrl-2");
        assert_eq!(request.method, ControlMethod::Ping);
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

    #[test]
    fn status_for_non_empty_results_is_silent() {
        let text = render_status_text(QueryState::Results { count: 8 });
        assert_eq!(text, "");
    }

    #[test]
    fn status_for_empty_results_is_silent() {
        let text = render_status_text(QueryState::Empty);
        assert_eq!(text, "");
    }

    #[test]
    fn status_for_error_remains_visible() {
        let text = render_status_text(QueryState::Error("boom".to_string()));
        assert_eq!(text, "Error: boom");
    }

    #[test]
    fn enter_uses_selected_row_result_id() {
        let selected = vec![LauncherResult {
            id: "app:terminal".into(),
            kind: "app".into(),
            title: "Terminal".into(),
            subtitle: "".into(),
            icon: "".into(),
        }];
        let id = selected_result_id(&selected, 0);
        assert_eq!(id, Some("app:terminal"));
    }

    #[test]
    fn parse_execute_response_returns_reason_on_unresolved_id() {
        let raw = serde_json::json!({
            "id": "gtk-execute",
            "result": {
                "ok": false,
                "success": false,
                "executed": false,
                "action_resolved": false,
                "launch_spawned": false,
                "execution_status": "unresolved",
                "error_message": "unsupported result id"
            }
        });

        let parsed = parse_execute_response(&raw);
        assert!(parsed.is_err());
        assert_eq!(parsed.err().as_deref(), Some("unsupported result id"));
    }
}
