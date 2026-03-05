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
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut action_resolved = false;
    let mut launch_spawned = false;
    let mut execution_status = "unresolved".to_string();
    let mut error_message: Option<String> = Some("unsupported result id".to_string());
    if let Some((cmd, args)) = command_for_result_id_with_desktop(result_id, &desktop) {
        action_resolved = true;
        execution_status = "resolved".to_string();
        error_message = None;
        launch_spawned = spawn_with_fallback(&cmd, &args);
        if !launch_spawned {
            execution_status = "spawn_failed".to_string();
            error_message = Some(format!("failed to spawn {}", cmd));
        }
    }

    json!({
        "ok": true,
        "executed": action_resolved,
        "action_resolved": action_resolved,
        "launch_spawned": launch_spawned,
        "execution_status": execution_status,
        "error_message": error_message,
        "result_id": result_id,
        "action": action,
    })
}

fn command_for_result_id_with_desktop(
    result_id: &str,
    desktop: &str,
) -> Option<(String, Vec<String>)> {
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
            _ => return None,
        };
        return Some(("xdg-open".to_string(), vec![url.to_string()]));
    }

    None
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
        assert_eq!(response["execution_status"], "unresolved");
        assert_eq!(response["error_message"], "unsupported result id");
    }

    #[test]
    fn resolves_utility_weather_to_browser_command() {
        let resolved = command_for_result_id_with_desktop("utility:weather", "GNOME")
            .expect("utility command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(resolved.1, vec!["https://wttr.in".to_string()]);
    }

    #[test]
    fn resolves_utility_calculator_expression_to_browser_command() {
        let resolved = command_for_result_id_with_desktop("utility:calculator:2+2", "GNOME")
            .expect("calculator command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(
            resolved.1,
            vec!["https://www.google.com/search?q=2%2B2".to_string()]
        );
    }

    #[test]
    fn resolves_utility_currency_payload_to_browser_command() {
        let resolved =
            command_for_result_id_with_desktop("utility:currency:12:USD:CHF", "GNOME")
                .expect("currency command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(
            resolved.1,
            vec![
                "https://www.xe.com/currencyconverter/convert/?Amount=12&From=USD&To=CHF"
                    .to_string()
            ]
        );
    }
}
