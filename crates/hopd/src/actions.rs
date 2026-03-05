use std::process::{Command, Stdio};

use serde_json::{json, Value};

pub fn execute(params: &Value) -> Value {
    let result_id = params
        .get("result_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let action = params
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("enter");
    let effective_action = if action == "enter" && result_id.starts_with("utility:") {
        "copy"
    } else {
        action
    };
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut action_resolved = false;
    let mut launch_spawned = false;
    let mut execution_status = "unresolved".to_string();
    let mut error_message: Option<String> = Some("unsupported result id".to_string());
    let mut resolved_command: Option<String> = None;
    let mut resolved_args: Option<Vec<String>> = None;
    let mut copied_text: Option<String> = None;

    if effective_action == "copy" {
        if let Some(text) = copy_text_for_result_id(result_id) {
            action_resolved = true;
            execution_status = "copied".to_string();
            error_message = None;
            copied_text = Some(text);
        }
    } else if let Some((cmd, args)) = command_for_result_id_with_desktop(result_id, &desktop) {
        action_resolved = true;
        execution_status = "resolved".to_string();
        error_message = None;
        resolved_command = Some(cmd.clone());
        resolved_args = Some(args.clone());
        launch_spawned = spawn_with_fallback(&cmd, &args);
        if !launch_spawned {
            execution_status = "spawn_failed".to_string();
            error_message = Some(format!("failed to spawn {}", cmd));
        }
    }

    let success = if effective_action == "copy" {
        action_resolved
    } else {
        action_resolved && launch_spawned
    };

    json!({
        "ok": action_resolved,
        "success": success,
        "executed": action_resolved,
        "action_resolved": action_resolved,
        "launch_spawned": launch_spawned,
        "execution_status": execution_status,
        "error_message": error_message,
        "resolved_command": resolved_command,
        "resolved_args": resolved_args,
        "copied_text": copied_text,
        "result_id": result_id,
        "action": action,
        "action_effective": effective_action,
    })
}

fn command_for_result_id_with_desktop(
    result_id: &str,
    desktop: &str,
) -> Option<(String, Vec<String>)> {
    if let Some(payload) = result_id.strip_prefix("settingcmd:") {
        return parse_setting_command_payload(payload);
    }

    if let Some(desktop_id) = result_id.strip_prefix("app:") {
        if desktop_id.is_empty() {
            return None;
        }
        return Some(("gtk-launch".to_string(), vec![desktop_id.to_string()]));
    }

    if let Some(path) = result_id.strip_prefix("file:") {
        if path.is_empty() {
            return None;
        }
        return Some(("xdg-open".to_string(), vec![path.to_string()]));
    }

    if let Some(path) = result_id.strip_prefix("recent:") {
        if path.is_empty() {
            return None;
        }
        return Some(("xdg-open".to_string(), vec![path.to_string()]));
    }

    if let Some(setting_key) = result_id.strip_prefix("setting:") {
        if setting_key.is_empty() {
            return None;
        }
        let is_kde = desktop.to_uppercase().contains("KDE");
        if is_kde {
            return Some(("systemsettings5".to_string(), vec![setting_key.to_string()]));
        }
        return Some((
            "gnome-control-center".to_string(),
            vec![setting_key.to_string()],
        ));
    }

    if let Some(window_id) = result_id.strip_prefix("window:") {
        if window_id.is_empty() {
            return None;
        }
        if let Some(hypr_id) = window_id.strip_prefix("hypr:") {
            if hypr_id.is_empty() {
                return None;
            }
            return Some((
                "hyprctl".to_string(),
                vec![
                    "dispatch".to_string(),
                    "focuswindow".to_string(),
                    format!("address:{hypr_id}"),
                ],
            ));
        }
        if let Some(sway_id) = window_id.strip_prefix("sway:") {
            if sway_id.is_empty() {
                return None;
            }
            return Some((
                "swaymsg".to_string(),
                vec![format!("[con_id={sway_id}]"), "focus".to_string()],
            ));
        }
        return Some((
            "wmctrl".to_string(),
            vec!["-ia".to_string(), window_id.to_string()],
        ));
    }

    if let Some(utility_key) = result_id.strip_prefix("utility:") {
        if utility_key.is_empty() {
            return None;
        }
        if let Some(expression) = utility_key.strip_prefix("calculator:") {
            if expression.is_empty() {
                return None;
            }
            let query = format!("https://www.google.com/search?q={}", encode_component(expression));
            return Some(("xdg-open".to_string(), vec![query]));
        }
        if let Some(payload) = utility_key.strip_prefix("currency:") {
            let parts: Vec<&str> = payload.split(':').collect();
            if parts.len() != 3 {
                return None;
            }
            let url = format!(
                "https://www.xe.com/currencyconverter/convert/?Amount={}&From={}&To={}",
                encode_component(parts[0]),
                encode_component(parts[1]),
                encode_component(parts[2])
            );
            return Some(("xdg-open".to_string(), vec![url]));
        }
        let url = match utility_key {
            "weather" => "https://wttr.in",
            "timezone" => "https://time.is",
            "emoji" => "https://emojipedia.org",
            "calculator" => "https://www.google.com/search?q=calculator",
            "currency" => "https://www.xe.com/currencyconverter/",
            _ if utility_key.starts_with("weather:") => {
                let encoded_location = utility_key.strip_prefix("weather:").unwrap_or_default();
                if encoded_location.is_empty() {
                    return None;
                }
                let location = decode_component(encoded_location)?;
                let path = location.trim();
                if path.is_empty() {
                    return None;
                }
                return Some((
                    "xdg-open".to_string(),
                    vec![format!("https://wttr.in/{}", encode_component(path))],
                ));
            }
            _ => return None,
        };
        return Some(("xdg-open".to_string(), vec![url.to_string()]));
    }

    if let Some(payload) = result_id.strip_prefix("web-search:") {
        let mut parts = payload.splitn(3, ':');
        let _service_id = parts.next()?;
        let encoded_url = parts.next()?;
        let url = decode_component(encoded_url)?;
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return None;
        }
        return Some(("xdg-open".to_string(), vec![url]));
    }

    None
}

fn parse_setting_command_payload(payload: &str) -> Option<(String, Vec<String>)> {
    if payload.trim().is_empty() {
        return None;
    }

    let mut parts = payload
        .split('|')
        .map(decode_component)
        .collect::<Option<Vec<String>>>()?;
    if parts.is_empty() {
        return None;
    }
    let command = parts.remove(0);
    if command.trim().is_empty() {
        return None;
    }
    Some((command, parts))
}

fn copy_text_for_result_id(result_id: &str) -> Option<String> {
    if let Some(expression) = result_id.strip_prefix("utility:calculator:") {
        if expression.trim().is_empty() {
            return None;
        }
        let decoded = decode_component(expression)?;
        if let Ok(value) = fasteval::ez_eval(&decoded, &mut fasteval::EmptyNamespace) {
            if value.is_finite() {
                return Some(format_calculated_value(value));
            }
        }
        return Some(decoded);
    }
    if let Some(payload) = result_id.strip_prefix("utility:currency:") {
        let parts: Vec<&str> = payload.split(':').collect();
        if parts.len() != 3 {
            return None;
        }
        return Some(format!("{} {} to {}", parts[0], parts[1], parts[2]));
    }
    if let Some(location) = result_id.strip_prefix("utility:weather:") {
        let decoded = decode_component(location)?;
        if decoded.trim().is_empty() {
            return None;
        }
        return Some(decoded);
    }
    if result_id == "utility:weather" {
        return Some("weather".to_string());
    }
    if result_id == "utility:timezone" {
        return Some("timezone".to_string());
    }
    if result_id == "utility:emoji" {
        return Some("emoji".to_string());
    }
    None
}

fn format_calculated_value(value: f64) -> String {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    let mut text = format!("{rounded:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
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

fn decode_component(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let value = u8::from_str_radix(hex, 16).ok()?;
            out.push(value as char);
            i += 3;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    Some(out)
}

fn spawn_with_fallback(cmd: &str, args: &[String]) -> bool {
    for (candidate, candidate_args) in spawn_candidates(cmd, args) {
        let spawned = Command::new(&candidate)
            .args(&candidate_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();
        if spawned {
            return true;
        }
    }
    false
}

fn spawn_candidates(cmd: &str, args: &[String]) -> Vec<(String, Vec<String>)> {
    let mut candidates = vec![(cmd.to_string(), args.to_vec())];
    if cmd == "systemsettings5" {
        candidates.push(("systemsettings".to_string(), args.to_vec()));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_app_result_id_to_launch_command() {
        let resolved = command_for_result_id_with_desktop("app:org.gnome.Nautilus.desktop", "GNOME")
            .expect("app command");
        assert_eq!(resolved.0, "gtk-launch");
        assert_eq!(resolved.1[0], "org.gnome.Nautilus.desktop");
    }

    #[test]
    fn resolves_file_result_id_to_xdg_open_command() {
        let resolved = command_for_result_id_with_desktop("file:/tmp/demo.txt", "GNOME")
            .expect("file command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(resolved.1[0], "/tmp/demo.txt");
    }

    #[test]
    fn resolves_window_result_id_to_wmctrl_focus_command() {
        let resolved = command_for_result_id_with_desktop("window:0x04200004", "GNOME")
            .expect("window command");
        assert_eq!(resolved.0, "wmctrl");
        assert_eq!(resolved.1, vec!["-ia".to_string(), "0x04200004".to_string()]);
    }

    #[test]
    fn resolves_hypr_window_result_id_to_hyprctl_focus_command() {
        let resolved = command_for_result_id_with_desktop("window:hypr:0x04200004", "GNOME")
            .expect("hypr window command");
        assert_eq!(resolved.0, "hyprctl");
        assert_eq!(
            resolved.1,
            vec![
                "dispatch".to_string(),
                "focuswindow".to_string(),
                "address:0x04200004".to_string(),
            ]
        );
    }

    #[test]
    fn resolves_sway_window_result_id_to_swaymsg_focus_command() {
        let resolved = command_for_result_id_with_desktop("window:sway:42", "GNOME")
            .expect("sway window command");
        assert_eq!(resolved.0, "swaymsg");
        assert_eq!(resolved.1, vec!["[con_id=42]".to_string(), "focus".to_string()]);
    }

    #[test]
    fn resolves_settings_result_id_for_kde_desktop() {
        let resolved = command_for_result_id_with_desktop("setting:network", "KDE")
            .expect("kde settings command");
        assert_eq!(resolved.0, "systemsettings5");
        assert_eq!(resolved.1, vec!["network".to_string()]);
    }

    #[test]
    fn resolves_settingcmd_result_id_to_direct_command() {
        let resolved = command_for_result_id_with_desktop(
            "settingcmd:gnome-control-center|privacy",
            "GNOME",
        )
        .expect("settingcmd command");
        assert_eq!(resolved.0, "gnome-control-center");
        assert_eq!(resolved.1, vec!["privacy".to_string()]);
    }

    #[test]
    fn spawn_candidates_include_systemsettings_fallback_for_kde() {
        let args = vec!["network".to_string()];
        let candidates = spawn_candidates("systemsettings5", &args);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].0, "systemsettings5");
        assert_eq!(candidates[1].0, "systemsettings");
    }

    #[test]
    fn execute_reports_unresolved_result_id_reason() {
        let payload = serde_json::json!({
            "result_id": "unknown-id",
            "action": "enter",
        });
        let response = execute(&payload);
        assert_eq!(response["executed"], false);
        assert_eq!(response["success"], false);
        assert_eq!(response["execution_status"], "unresolved");
        assert_eq!(response["error_message"], "unsupported result id");
    }

    #[test]
    fn utility_enter_defaults_to_copy_semantics() {
        let payload = serde_json::json!({
            "result_id": "utility:weather:San+Francisco",
            "action": "enter",
        });
        let response = execute(&payload);
        assert_eq!(response["ok"], true);
        assert_eq!(response["success"], true);
        assert_eq!(response["execution_status"], "copied");
        assert_eq!(response["action_effective"], "copy");
        assert_eq!(response["resolved_command"], serde_json::Value::Null);
        assert_eq!(response["copied_text"], "San Francisco");
    }

    #[test]
    fn copy_action_returns_copied_text_for_calculator_utility() {
        let payload = serde_json::json!({
            "result_id": "utility:calculator:2%2B2",
            "action": "copy",
        });
        let response = execute(&payload);
        assert_eq!(response["ok"], true);
        assert_eq!(response["success"], true);
        assert_eq!(response["execution_status"], "copied");
        assert_eq!(response["copied_text"], "4");
        assert_eq!(response["launch_spawned"], false);
        assert_eq!(response["resolved_command"], serde_json::Value::Null);
    }

    #[test]
    fn resolves_web_search_result_to_xdg_open_command() {
        let encoded = encode_component("https://www.google.com/search?q=rust");
        let result_id = format!("web-search:google:{encoded}");
        let resolved = command_for_result_id_with_desktop(&result_id, "GNOME")
            .expect("web search command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(resolved.1, vec!["https://www.google.com/search?q=rust".to_string()]);
    }
}
