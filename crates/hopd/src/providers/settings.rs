use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();

    let mut catalog: Vec<SearchItem> = vec![SearchItem::new(
        "hop-launcher-settings",
        "setting",
        "Hop Launcher Settings",
        "Launcher preferences",
        "preferences-system-symbolic",
        "settings preferences launcher hotkey translucency results",
    )];

    let discovered = desktop_settings_rows();
    if discovered.is_empty() {
        catalog.extend(fallback_settings_catalog());
    } else {
        catalog.extend(discovered);
    }

    catalog
        .into_iter()
        .filter(|item| {
            normalized.is_empty()
                || item.title.to_lowercase().contains(&normalized)
                || item.keywords.to_lowercase().contains(&normalized)
        })
        .collect()
}

fn desktop_settings_rows() -> Vec<SearchItem> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for path in desktop_entry_files() {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown.desktop");
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(row) = parse_settings_desktop_entry(&content, file_name) else {
            continue;
        };
        if seen.insert(row.id.clone()) {
            out.push(row);
        }
    }

    out
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

    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for root in data_dirs.split(':').filter(|value| !value.trim().is_empty()) {
        roots.push(Path::new(root.trim()).join("applications"));
    }
    roots
}

fn parse_settings_desktop_entry(content: &str, file_name: &str) -> Option<SearchItem> {
    let mut name = String::new();
    let mut localized_name = String::new();
    let mut exec = String::new();
    let mut keywords = String::new();
    let mut categories = String::new();
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
                exec = value.trim().to_string();
            }
        } else if let Some(value) = line.strip_prefix("Keywords=") {
            if keywords.is_empty() {
                keywords = value.replace(';', " ");
            }
        } else if let Some(value) = line.strip_prefix("Categories=") {
            if categories.is_empty() {
                categories = value.replace(';', " ");
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
    if name.is_empty() || exec.is_empty() {
        return None;
    }

    let exec_parts = parse_exec_tokens(&exec);
    if !looks_like_settings_entry(&exec_parts, &categories, file_name, &name) {
        return None;
    }

    let command = exec_parts.first()?.to_string();
    let args = exec_parts.iter().skip(1).cloned().collect::<Vec<_>>();
    let result_id = build_setting_command_id(&command, &args);

    let merged_keywords = format!(
        "settings {} {} {} {} {} {}",
        name,
        keywords,
        categories,
        comment,
        file_name,
        exec_parts.join(" ")
    );

    Some(SearchItem::new(
        &result_id,
        "setting",
        &name,
        "System settings",
        if icon.is_empty() {
            "preferences-system-symbolic"
        } else {
            &icon
        },
        &merged_keywords,
    ))
}

fn parse_exec_tokens(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .filter(|token| !token.starts_with('%'))
        .filter(|token| !token.contains("=$") && !token.ends_with("=$"))
        .map(trim_wrapping_quotes)
        .filter(|token| !token.is_empty())
        .collect()
}

fn trim_wrapping_quotes(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn looks_like_settings_entry(exec_parts: &[String], categories: &str, file_name: &str, name: &str) -> bool {
    let categories_lower = categories.to_lowercase();
    if categories_lower.contains("settings") {
        return true;
    }

    if let Some(command) = exec_parts.first() {
        let base = Path::new(command)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(command)
            .to_lowercase();
        if [
            "gnome-control-center",
            "systemsettings",
            "systemsettings5",
            "kcmshell",
            "kcmshell5",
            "kcmshell6",
        ]
        .contains(&base.as_str())
        {
            return true;
        }
    }

    let name_lower = name.to_lowercase();
    let file_lower = file_name.to_lowercase();
    name_lower.contains("settings")
        || name_lower.contains("preferences")
        || file_lower.contains("settings")
}

fn build_setting_command_id(command: &str, args: &[String]) -> String {
    let mut id = format!("settingcmd:{}", encode_id_part(command));
    for arg in args {
        id.push('|');
        id.push_str(&encode_id_part(arg));
    }
    id
}

fn encode_id_part(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | '~') {
            out.push(ch);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn fallback_settings_catalog() -> Vec<SearchItem> {
    let desktop = current_desktop();
    let is_gnome = desktop.contains("GNOME");
    let is_kde = desktop.contains("KDE");
    let has_gnome_center = command_exists("gnome-control-center");
    let has_kde_settings = command_exists("systemsettings5") || command_exists("systemsettings");

    let mut out = Vec::new();
    if has_gnome_center && (is_gnome || !is_kde) {
        out.extend(vec![
            SearchItem::new(
                "setting:bluetooth",
                "setting",
                "Bluetooth Settings",
                "GNOME settings",
                "bluetooth-active-symbolic",
                "settings bluetooth system preferences panel gnome",
            ),
            SearchItem::new(
                "setting:network",
                "setting",
                "Network Settings",
                "GNOME settings",
                "network-wireless-symbolic",
                "settings network wifi ethernet internet panel gnome",
            ),
            SearchItem::new(
                "setting:display",
                "setting",
                "Display Settings",
                "GNOME settings",
                "video-display-symbolic",
                "settings display monitor resolution panel gnome",
            ),
        ]);
    }
    if has_kde_settings && (is_kde || !is_gnome) {
        out.extend(vec![
            SearchItem::new(
                "setting:network",
                "setting",
                "Network Settings",
                "KDE system settings",
                "network-wireless-symbolic",
                "settings network wifi ethernet internet panel kde",
            ),
            SearchItem::new(
                "setting:display",
                "setting",
                "Display Settings",
                "KDE system settings",
                "video-display-symbolic",
                "settings display monitor resolution panel kde",
            ),
        ]);
    }
    if out.is_empty() {
        out.push(SearchItem::new(
            "setting:network",
            "setting",
            "Network Settings",
            "System settings",
            "network-wireless-symbolic",
            "settings network wifi ethernet internet panel",
        ));
    }
    out
}

fn current_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase()
}

fn command_exists(binary: &str) -> bool {
    let path = std::env::var_os("PATH");
    let Some(path) = path else {
        return false;
    };
    for root in std::env::split_paths(&path) {
        let candidate = root.join(binary);
        if candidate.is_file() {
            return true;
        }
        if cfg!(windows) {
            for ext in [".exe", ".bat", ".cmd"] {
                if Path::new(&format!("{}{}", candidate.to_string_lossy(), ext)).is_file() {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_hop_launcher_settings_row() {
        let rows = results("settings");
        assert!(
            rows.iter().any(|row| row.id == "hop-launcher-settings"),
            "expected launcher settings row"
        );
    }

    #[test]
    fn parses_settings_desktop_entry_into_command_row() {
        let row = parse_settings_desktop_entry(
            "[Desktop Entry]\nName=Privacy\nExec=gnome-control-center privacy %u\nCategories=Settings;DesktopSettings;\nIcon=org.gnome.Settings-symbolic\n",
            "gnome-privacy-panel.desktop",
        )
        .expect("settings row parsed");
        assert_eq!(row.kind, "setting");
        assert_eq!(row.title, "Privacy");
        assert!(row.id.starts_with("settingcmd:gnome-control-center|privacy"));
        assert_eq!(row.icon, "org.gnome.Settings-symbolic");
    }

    #[test]
    fn skips_non_settings_desktop_entry() {
        let row = parse_settings_desktop_entry(
            "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nCategories=Network;WebBrowser;\n",
            "firefox.desktop",
        );
        assert!(row.is_none());
    }

    #[test]
    fn fallback_catalog_contains_system_network_row() {
        let rows = fallback_settings_catalog();
        assert!(rows.iter().any(|row| row.id == "setting:network"));
    }
}
