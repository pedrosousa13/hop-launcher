use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let is_empty_query = normalized.is_empty();

    desktop_entry_files()
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
        .collect()
}

fn matches_query(item: &SearchItem, query: &str) -> bool {
    item.title.to_lowercase().contains(query) || item.keywords.to_lowercase().contains(query)
}

fn desktop_entry_files() -> Vec<PathBuf> {
    let mut roots = xdg_application_roots();
    if let Ok(home) = std::env::var("HOME") {
        roots.push(Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }
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

fn xdg_application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        roots.push(Path::new(&data_home).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        roots.push(Path::new(&home).join(".local/share/applications"));
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for root in data_dirs.split(':').filter(|value| !value.trim().is_empty()) {
        roots.push(Path::new(root.trim()).join("applications"));
    }
    roots
}

fn parse_desktop_entry(content: &str, file_name: &str) -> Option<SearchItem> {
    let mut name = String::new();
    let mut localized_name = String::new();
    let mut exec = String::new();
    let mut keywords = String::new();
    let mut generic_name = String::new();
    let mut comment = String::new();
    let mut icon = String::new();
    let mut hidden = false;
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = line.strip_prefix("Name=") {
            if name.is_empty() {
                name = value.trim().to_string();
            }
        } else if line.starts_with("Name[") {
            if let Some((_, value)) = line.split_once('=') {
                if localized_name.is_empty() {
                    localized_name = value.trim().to_string();
                }
            }
        } else if let Some(value) = line.strip_prefix("Exec=") {
            if exec.is_empty() {
                exec = sanitize_exec(value);
            }
        } else if let Some(value) = line.strip_prefix("Keywords=") {
            if keywords.is_empty() {
                keywords = value.replace(';', " ");
            }
        } else if let Some(value) = line.strip_prefix("GenericName=") {
            if generic_name.is_empty() {
                generic_name = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Comment=") {
            if comment.is_empty() {
                comment = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Icon=") {
            if icon.is_empty() {
                icon = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Hidden=") {
            hidden = value.trim().eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("NoDisplay=") {
            no_display = value.trim().eq_ignore_ascii_case("true");
        }
    }

    if hidden || no_display {
        return None;
    }
    if name.is_empty() && !localized_name.is_empty() {
        name = localized_name;
    }
    if name.is_empty() {
        return None;
    }

    let merged_keywords = if keywords.is_empty() {
        format!("{name} {exec} {generic_name} {comment}")
    } else {
        format!("{name} {exec} {keywords} {generic_name} {comment}")
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

fn sanitize_exec(raw: &str) -> String {
    raw.split_whitespace()
        .filter(|token| !token.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
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

    #[test]
    fn skips_hidden_desktop_entries() {
        let item = parse_desktop_entry(
            "[Desktop Entry]\nName=Hidden App\nExec=hidden-app\nNoDisplay=true\n",
            "hidden.desktop",
        );
        assert!(item.is_none());
    }

    #[test]
    fn parses_localized_name_when_primary_name_missing() {
        let item = parse_desktop_entry(
            "[Desktop Entry]\nName[en_US]=Localized App\nExec=localized-app %U\nType=Application\n",
            "localized.desktop",
        )
        .expect("desktop entry parsed");
        assert_eq!(item.title, "Localized App");
        assert!(item.keywords.contains("localized-app"));
        assert!(!item.keywords.contains("%U"));
    }
}
