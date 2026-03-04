use std::process::Command;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut rows = wmctrl_windows()
        .into_iter()
        .filter(|window| window.title.to_lowercase().contains(&normalized))
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
        .take(20)
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
}
