use std::fs;
use std::path::Path;

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    let is_empty_query = normalized.is_empty();

    let mut rows = load_recent_file_paths()
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
            let icon = icon_for_path(path_obj);
            SearchItem::new(
                &format!("recent:{path}"),
                "recent",
                &title,
                "Recent file",
                &icon,
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
        if let Some(path) = parse_file_uri(href) {
            out.push(path);
        }
    }
    out
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
}
