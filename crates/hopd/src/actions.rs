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
    let mut action_resolved = false;
    let mut launch_spawned = false;
    if let Some((cmd, args)) = command_for_result_id(result_id) {
        action_resolved = true;
        launch_spawned = Command::new(&cmd)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();
    }

    json!({
        "ok": true,
        "executed": !result_id.is_empty(),
        "action_resolved": action_resolved,
        "launch_spawned": launch_spawned,
        "result_id": result_id,
        "action": action,
    })
}

fn command_for_result_id(result_id: &str) -> Option<(String, Vec<String>)> {
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
        return Some((
            "gnome-control-center".to_string(),
            vec![setting_key.to_string()],
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_app_result_id_to_launch_command() {
        let resolved = command_for_result_id("app:org.gnome.Nautilus.desktop")
            .expect("app command");
        assert_eq!(resolved.0, "gtk-launch");
        assert_eq!(resolved.1[0], "org.gnome.Nautilus.desktop");
    }

    #[test]
    fn resolves_file_result_id_to_xdg_open_command() {
        let resolved = command_for_result_id("file:/tmp/demo.txt")
            .expect("file command");
        assert_eq!(resolved.0, "xdg-open");
        assert_eq!(resolved.1[0], "/tmp/demo.txt");
    }
}
