use std::fs;
use std::path::Path;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let is_empty_query = normalized.is_empty();

    load_recent_file_paths()
        .into_iter()
        .filter(|path| is_empty_query || path.to_lowercase().contains(&normalized))
        .take(12)
        .map(|path| {
            let path_obj = Path::new(&path);
            let title = path_obj
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Recent file")
                .to_string();
            let subtitle = path_obj
                .parent()
                .map(|parent| parent.to_string_lossy().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "Recent file".to_string());
            let icon = icon_for_path(path_obj);
            SearchItem::new(
                &format!("recent:{path}"),
                "recent",
                &title,
                &subtitle,
                &icon,
                &format!("recent {title} {path}"),
            )
        })
        .collect::<Vec<_>>()
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
    parse_recent_bookmarks(xbel)
        .into_iter()
        .map(|bookmark| bookmark.path)
        .collect()
}

#[derive(Debug)]
struct RecentBookmark {
    path: String,
    modified: Option<String>,
    seq: usize,
}

fn parse_recent_bookmarks(xbel: &str) -> Vec<RecentBookmark> {
    let mut out = Vec::new();
    for (seq, chunk) in xbel.split("<bookmark").skip(1).enumerate() {
        let href = extract_attr(chunk, "href").unwrap_or_default();
        let Some(path) = parse_file_uri(href) else {
            continue;
        };
        let modified = extract_attr(chunk, "modified").map(|value| value.to_string());
        out.push(RecentBookmark {
            path,
            modified,
            seq,
        });
    }
    out.sort_by(|left, right| match (&left.modified, &right.modified) {
        (Some(l), Some(r)) => r.cmp(l),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.seq.cmp(&right.seq),
    });
    out
}

fn extract_attr<'a>(chunk: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = chunk.find(&needle)? + needle.len();
    let rest = chunk.get(start..)?;
    Some(rest.split('"').next().unwrap_or_default())
}

fn parse_file_uri(uri: &str) -> Option<String> {
    let stripped = uri.strip_prefix("file://")?;
    let encoded_path = if stripped.starts_with('/') {
        stripped
    } else {
        let (host, rest) = stripped.split_once('/')?;
        if !host.is_empty() && host != "localhost" {
            return None;
        }
        &uri[(uri.len() - rest.len() - 1)..]
    };
    Some(percent_decode(encoded_path))
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%' && idx + 2 < bytes.len() {
            let hi = bytes[idx + 1] as char;
            let lo = bytes[idx + 2] as char;
            if let (Some(hi), Some(lo)) = (hi.to_digit(16), lo.to_digit(16)) {
                out.push((hi * 16 + lo) as u8 as char);
                idx += 3;
                continue;
            }
        }
        out.push(bytes[idx] as char);
        idx += 1;
    }
    out
}

fn icon_for_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let icon = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "heic" => "image-x-generic-symbolic",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => "audio-x-generic-symbolic",
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "video-x-generic-symbolic",
        "pdf" => "application-pdf-symbolic",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "package-x-generic-symbolic",
        "desktop" | "appimage" => "application-x-executable-symbolic",
        "rs" | "c" | "cpp" | "h" | "hpp" | "py" | "js" | "ts" | "tsx" | "java" | "go"
        | "sh" | "bash" | "zsh" | "toml" | "json" | "yaml" | "yml" | "xml" => {
            "text-x-script-symbolic"
        }
        _ => "document-open-recent-symbolic",
    };
    icon.to_string()
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

    #[test]
    fn parses_localhost_and_percent_encoded_file_uris() {
        let xbel = r#"<?xml version="1.0"?><xbel><bookmark href="file://localhost/home/pedro/My%20Notes.txt"/><bookmark href="file:///tmp/sprint%231.md"/></xbel>"#;
        let entries = parse_recent_file_uris(xbel);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "/home/pedro/My Notes.txt");
        assert_eq!(entries[1], "/tmp/sprint#1.md");
    }

    #[test]
    fn recent_icons_match_common_extensions() {
        assert_eq!(
            icon_for_path(Path::new("/tmp/photo.png")),
            "image-x-generic-symbolic"
        );
        assert_eq!(
            icon_for_path(Path::new("/tmp/video.mp4")),
            "video-x-generic-symbolic"
        );
        assert_eq!(
            icon_for_path(Path::new("/tmp/notes.txt")),
            "document-open-recent-symbolic"
        );
    }

    #[test]
    fn recent_entries_are_sorted_by_modified_time_descending() {
        let xbel = r#"<?xml version="1.0"?><xbel>
            <bookmark href="file:///tmp/older.txt" modified="2025-02-01T10:00:00Z"/>
            <bookmark href="file:///tmp/newer.txt" modified="2025-02-03T10:00:00Z"/>
        </xbel>"#;
        let entries = parse_recent_file_uris(xbel);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "/tmp/newer.txt");
        assert_eq!(entries[1], "/tmp/older.txt");
    }
}
