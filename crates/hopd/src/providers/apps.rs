use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let is_empty_query = normalized.is_empty();

    let mut items: Vec<SearchItem> = desktop_entry_files()
        .into_iter()
        .filter_map(|path| {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown.desktop")
                .to_string();
            let content = fs::read_to_string(path).ok()?;
            parse_desktop_entry(&content, &file_name)
        })
        .filter(|item| is_empty_query || matches_query(item, &normalized))
        .take(if is_empty_query { 12 } else { 24 })
        .collect();

    if items.is_empty() {
        items.push(SearchItem::new(
            "app:org.gnome.Terminal.desktop",
            "app",
            "Terminal",
            "System application",
            "utilities-terminal-symbolic",
            "terminal app shell console",
        ));
    }

    items
}

fn matches_query(item: &SearchItem, query: &str) -> bool {
    item.title.to_lowercase().contains(query) || item.keywords.to_lowercase().contains(query)
}

fn desktop_entry_files() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        roots.push(Path::new(&home).join(".local/share/applications"));
        roots.push(Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }
    roots.push(PathBuf::from("/usr/share/applications"));
    roots.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            if seen.insert(path.clone()) {
                out.push(path);
            }
        }
    }
    out
}

fn parse_desktop_entry(content: &str, file_name: &str) -> Option<SearchItem> {
    let mut name = String::new();
    let mut exec = String::new();
    let mut keywords = String::new();
    let mut icon = String::new();

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("Name=") {
            if name.is_empty() {
                name = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Exec=") {
            if exec.is_empty() {
                exec = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Keywords=") {
            if keywords.is_empty() {
                keywords = value.replace(';', " ");
            }
        } else if let Some(value) = line.strip_prefix("Icon=") {
            if icon.is_empty() {
                icon = value.trim().to_string();
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    let merged_keywords = if keywords.is_empty() {
        format!("{name} {exec}")
    } else {
        format!("{name} {exec} {keywords}")
    };
    Some(SearchItem::new(
        &format!("app:{file_name}"),
        "app",
        &name,
        "Installed application",
        if icon.is_empty() {
            "application-x-executable-symbolic"
        } else {
            &icon
        },
        &merged_keywords,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_entry_into_search_item() {
        let item = parse_desktop_entry(
            "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nIcon=firefox\nKeywords=browser;web;\n",
            "firefox.desktop",
        )
        .expect("desktop entry parsed");
        assert_eq!(item.id, "app:firefox.desktop");
        assert_eq!(item.kind, "app");
        assert_eq!(item.title, "Firefox");
        assert_eq!(item.icon, "firefox");
        assert!(item.keywords.contains("browser"));
    }
}
