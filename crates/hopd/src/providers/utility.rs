use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let lower = query.trim().to_lowercase();
    let explicit_utility_intent = lower.contains("utility") || lower.contains("utilities");
    if !explicit_utility_intent {
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
