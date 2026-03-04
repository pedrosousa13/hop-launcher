use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "setting:bluetooth",
        "setting",
        "Bluetooth Settings",
        "System settings",
        "preferences-system-symbolic",
        "settings bluetooth system preferences panel",
    )]
}
