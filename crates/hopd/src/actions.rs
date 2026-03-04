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
        launch_spawned = Command::new(&cmd)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();
        if !launch_spawned {
            execution_status = "spawn_failed".to_string();
            error_message = Some(format!("failed to spawn {}", cmd));
        }
    }

    json!({
        "ok": true,
        "executed": !result_id.is_empty(),
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
        return Some((
            "wmctrl".to_string(),
            vec!["-ia".to_string(), window_id.to_string()],
        ));
    }

    None
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
    fn resolves_settings_result_id_for_kde_desktop() {
        let resolved = command_for_result_id_with_desktop("setting:network", "KDE")
            .expect("kde settings command");
        assert_eq!(resolved.0, "systemsettings5");
        assert_eq!(resolved.1, vec!["network".to_string()]);
    }

    #[test]
    fn execute_reports_unresolved_result_id_reason() {
        let payload = serde_json::json!({
            "result_id": "unknown-id",
            "action": "enter",
        });
        let response = execute(&payload);
        assert_eq!(response["execution_status"], "unresolved");
        assert_eq!(response["error_message"], "unsupported result id");
    }
}
