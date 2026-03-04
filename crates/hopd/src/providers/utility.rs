use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.chars().count() > 2 {
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
