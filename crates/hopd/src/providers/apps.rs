use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "app:terminal",
        "app",
        "Terminal",
        "System application",
        "utilities-terminal-symbolic",
        "terminal app shell console",
    )]
}
