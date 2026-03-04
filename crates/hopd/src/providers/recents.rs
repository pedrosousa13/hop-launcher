use std::fs;
use std::path::Path;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut rows = load_recent_file_paths()
        .into_iter()
        .filter(|path| path.to_lowercase().contains(&normalized))
        .take(12)
        .map(|path| {
            let title = Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Recent file")
                .to_string();
            SearchItem::new(
                &format!("recent:{path}"),
                "recent",
                &title,
                "Recent file",
                "document-open-recent-symbolic",
                &format!("recent {title} {path}"),
            )
        })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(SearchItem::new(
            "recent:/tmp/notes.txt",
            "recent",
            "Notes.txt",
            "Recent file",
            "document-open-recent-symbolic",
            "recent activity notes file",
        ));
    }

    rows
}

fn load_recent_file_paths() -> Vec<String> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let path = Path::new(&home).join(".local/share/recently-used.xbel");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_recent_file_uris(&raw)
}

fn parse_recent_file_uris(xbel: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in xbel.split("href=\"").skip(1) {
        let href = chunk.split('"').next().unwrap_or_default();
        if let Some(stripped) = href.strip_prefix("file://") {
            out.push(stripped.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recently_used_xbel_hrefs() {
        let xbel = r#"<?xml version="1.0"?><xbel><bookmark href="file:///home/pedro/Notes.txt"/><bookmark href="file:///tmp/demo.md"/></xbel>"#;
        let entries = parse_recent_file_uris(xbel);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("Notes.txt"));
        assert!(entries[1].contains("demo.md"));
    }
}
