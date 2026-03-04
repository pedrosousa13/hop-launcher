use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "window:terminal-main",
        "window",
        "Terminal - Workspace",
        "Open window",
        "window-symbolic",
        "terminal window workspace",
    )]
}
