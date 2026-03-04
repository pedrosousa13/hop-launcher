use std::process::Command;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut rows = wmctrl_windows()
        .into_iter()
        .filter(|title| title.to_lowercase().contains(&normalized))
        .map(|title| {
            SearchItem::new(
                &format!("window:{title}"),
                "window",
                &title,
                "Open window",
                "window-symbolic",
                &format!("window {title}"),
            )
        })
        .take(20)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(SearchItem::new(
            "window:terminal-main",
            "window",
            "Terminal - Workspace",
            "Open window",
            "window-symbolic",
            "terminal window workspace",
        ));
    }

    rows
}

fn wmctrl_windows() -> Vec<String> {
    let Ok(output) = Command::new("wmctrl").arg("-lx").output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _id = parts.next()?;
            let _desktop = parts.next()?;
            let _wm_class = parts.next()?;
            let title = parts.collect::<Vec<_>>().join(" ");
            if title.trim().is_empty() {
                None
            } else {
                Some(title)
            }
        })
        .collect()
}
