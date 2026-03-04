use std::process::Command;

use crate::SearchItem;
use serde_json::Value;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let is_empty_query = normalized.is_empty();

    let mut rows = collect_windows()
        .into_iter()
        .filter(|window| is_empty_query || window.title.to_lowercase().contains(&normalized))
        .map(|window| {
            SearchItem::new(
                &format!("window:{}", window.id),
                "window",
                &window.title,
                "Open window",
                "window-symbolic",
                &format!("window {} {}", window.id, window.title),
            )
        })
        .take(if is_empty_query { 8 } else { 20 })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(SearchItem::new(
            "window:0x00000000",
            "window",
            "Terminal - Workspace",
            "Open window",
            "window-symbolic",
            "terminal window workspace",
        ));
    }

    rows
}

#[derive(Debug, Clone)]
struct WindowEntry {
    id: String,
    title: String,
}

fn collect_windows() -> Vec<WindowEntry> {
    let mut out = Vec::new();
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        out.extend(hyprland_windows());
    }
    if std::env::var("SWAYSOCK").is_ok() {
        out.extend(sway_windows());
    }
    out.extend(wmctrl_windows());

    if out.is_empty() {
        return out;
    }

    let mut seen = std::collections::HashSet::new();
    out.into_iter()
        .filter(|entry| seen.insert(format!("{}:{}", entry.id, entry.title)))
        .collect()
}

fn wmctrl_windows() -> Vec<WindowEntry> {
    let Ok(output) = Command::new("wmctrl").arg("-lx").output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_wmctrl_line)
        .collect()
}

fn parse_wmctrl_line(line: &str) -> Option<WindowEntry> {
    let mut parts = line.split_whitespace();
    let id = parts.next()?.to_string();
    let _desktop = parts.next()?;
    let _host = parts.next()?;
    let _wm_class = parts.next()?;
    let title = parts.collect::<Vec<_>>().join(" ");
    if title.trim().is_empty() {
        return None;
    }
    Some(WindowEntry { id, title })
}

fn hyprland_windows() -> Vec<WindowEntry> {
    let Ok(output) = Command::new("hyprctl").args(["clients", "-j"]).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_hyprctl_clients(&String::from_utf8_lossy(&output.stdout))
}

fn parse_hyprctl_clients(raw: &str) -> Vec<WindowEntry> {
    let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(rows) = parsed.as_array() else {
        return Vec::new();
    };

    rows.iter()
        .filter_map(|row| {
            let address = row.get("address")?.as_str()?.trim();
            let title = row.get("title")?.as_str()?.trim();
            if address.is_empty() || title.is_empty() {
                return None;
            }
            Some(WindowEntry {
                id: format!("hypr:{address}"),
                title: title.to_string(),
            })
        })
        .collect()
}

fn sway_windows() -> Vec<WindowEntry> {
    let Ok(output) = Command::new("swaymsg").args(["-t", "get_tree", "-r"]).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_sway_tree(&String::from_utf8_lossy(&output.stdout))
}

fn parse_sway_tree(raw: &str) -> Vec<WindowEntry> {
    let Ok(root) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_sway_nodes(&root, &mut out);
    out
}

fn collect_sway_nodes(node: &Value, out: &mut Vec<WindowEntry>) {
    let name = node.get("name").and_then(Value::as_str).unwrap_or("").trim();
    let id = node.get("id").and_then(Value::as_i64);
    let has_window_props = node
        .get("window_properties")
        .and_then(Value::as_object)
        .is_some();
    let has_app = node.get("app_id").and_then(Value::as_str).is_some();
    if !name.is_empty() && (has_window_props || has_app) {
        if let Some(con_id) = id {
            out.push(WindowEntry {
                id: format!("sway:{con_id}"),
                title: name.to_string(),
            });
        }
    }

    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(Value::as_array) {
            for child in children {
                collect_sway_nodes(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wmctrl_line_with_window_id_and_title() {
        let line = "0x04a00007  0 host Terminal.Gnome-terminal  Terminal - Workspace";
        let entry = parse_wmctrl_line(line).expect("parsed window");
        assert_eq!(entry.id, "0x04a00007");
        assert_eq!(entry.title, "Terminal - Workspace");
    }

    #[test]
    fn parses_hyprctl_clients_json() {
        let raw = r#"[{"address":"0x12345","title":"Alacritty"},{"address":"0x67890","title":"Firefox"}]"#;
        let entries = parse_hyprctl_clients(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "hypr:0x12345");
        assert_eq!(entries[1].title, "Firefox");
    }

    #[test]
    fn parses_sway_tree_into_window_entries() {
        let raw = r#"{
            "id":1,
            "name":"root",
            "nodes":[
                {
                    "id":12,
                    "name":"Terminal - Workspace",
                    "app_id":"org.gnome.Terminal",
                    "nodes":[],
                    "floating_nodes":[]
                },
                {
                    "id":13,
                    "name":"Firefox",
                    "window_properties":{"class":"firefox"},
                    "nodes":[],
                    "floating_nodes":[]
                }
            ],
            "floating_nodes":[]
        }"#;
        let entries = parse_sway_tree(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "sway:12");
        assert_eq!(entries[1].title, "Firefox");
    }
}
