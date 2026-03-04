use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "utility:catalog",
        "utility",
        "Utilities",
        "Launcher utility results",
        "system-search-symbolic",
        "utility calculator conversion",
    )]
}
