use std::path::Path;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let desktop = current_desktop();
    let is_gnome = desktop.contains("GNOME");
    let is_kde = desktop.contains("KDE");
    let has_gnome_center = command_exists("gnome-control-center");
    let has_kde_settings = command_exists("systemsettings5") || command_exists("systemsettings");

    let mut catalog: Vec<SearchItem> = vec![
        SearchItem::new(
            "hop-launcher-settings",
            "setting",
            "Hop Launcher Settings",
            "Launcher preferences",
            "preferences-system-symbolic",
            "settings preferences launcher hotkey translucency results",
        ),
    ];

    if has_gnome_center && (is_gnome || !is_kde) {
        catalog.extend(gnome_catalog());
    }
    if has_kde_settings && (is_kde || !is_gnome) {
        catalog.extend(kde_catalog());
    }
    if !has_gnome_center && !has_kde_settings {
        catalog.extend(fallback_catalog());
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

fn gnome_catalog() -> Vec<SearchItem> {
    vec![
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
        SearchItem::new(
            "setting:sound",
            "setting",
            "Sound Settings",
            "GNOME settings",
            "audio-volume-high-symbolic",
            "settings sound volume audio panel gnome",
        ),
    ]
}

fn kde_catalog() -> Vec<SearchItem> {
    vec![
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
        SearchItem::new(
            "setting:region",
            "setting",
            "Regional Settings",
            "KDE system settings",
            "preferences-desktop-locale-symbolic",
            "settings regional language locale kde",
        ),
    ]
}

fn fallback_catalog() -> Vec<SearchItem> {
    vec![
        SearchItem::new(
            "setting:network",
            "setting",
            "Network Settings",
            "System settings",
            "network-wireless-symbolic",
            "settings network wifi ethernet internet panel",
        ),
        SearchItem::new(
            "setting:display",
            "setting",
            "Display Settings",
            "System settings",
            "video-display-symbolic",
            "settings display monitor resolution panel",
        ),
    ]
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
    fn command_exists_detects_binary_from_path() {
        let original_path = std::env::var_os("PATH").unwrap_or_default();
        let path_entries = std::env::split_paths(&original_path)
            .collect::<Vec<_>>();
        let temp_dir = std::env::temp_dir().join(format!(
            "hopd-settings-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).expect("temp dir");
        let binary = temp_dir.join("hopd-test-binary");
        std::fs::write(&binary, "#!/bin/sh\nexit 0\n").expect("write fake binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary).expect("meta").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary, perms).expect("chmod");
        }
        let mut merged = vec![temp_dir.clone()];
        merged.extend(path_entries);
        let joined = std::env::join_paths(merged).expect("join paths");
        std::env::set_var("PATH", joined);
        assert!(command_exists("hopd-test-binary"));
        std::env::set_var("PATH", original_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn fallback_catalog_contains_system_network_row() {
        let rows = fallback_catalog();
        assert!(rows.iter().any(|row| row.id == "setting:network"));
    }
}
