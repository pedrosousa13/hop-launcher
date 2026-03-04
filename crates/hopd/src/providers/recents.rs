use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    vec![SearchItem::new(
        "recent:notes",
        "recent",
        "Notes.txt",
        "Recent file",
        "document-open-recent-symbolic",
        "recent activity notes file",
    )]
}
