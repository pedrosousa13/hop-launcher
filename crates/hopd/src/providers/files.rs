use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "file:readme",
        "file",
        "README.md",
        "Project file",
        "text-x-generic-symbolic",
        "readme markdown docs file",
    )]
}
