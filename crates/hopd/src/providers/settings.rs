use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();

    let mut catalog = vec![
        SearchItem::new(
            "setting:bluetooth",
            "setting",
            "Bluetooth Settings",
            "System settings",
            "preferences-system-symbolic",
            "settings bluetooth system preferences panel",
        ),
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
    ];

    if current_desktop().contains("KDE") {
        catalog.push(SearchItem::new(
            "setting:region",
            "setting",
            "Regional Settings",
            "KDE system settings",
            "preferences-desktop-locale-symbolic",
            "settings regional language locale kde",
        ));
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

fn current_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
}
