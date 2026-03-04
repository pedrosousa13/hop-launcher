use std::fs;
use std::path::{Path, PathBuf};

use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut rows = candidate_roots()
        .into_iter()
        .flat_map(read_dir_entries)
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let display = path.file_name()?.to_str()?.to_string();
            let full = path.to_string_lossy().to_string();
            let haystack = format!("{display} {full}").to_lowercase();
            if !haystack.contains(&normalized) {
                return None;
            }
            let icon = icon_for_path(&path);
            Some(SearchItem::new(
                &format!("file:{full}"),
                "file",
                &display,
                "Filesystem",
                &icon,
                &haystack,
            ))
        })
        .take(24)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(SearchItem::new(
            "file:README.md",
            "file",
            "README.md",
            "Project file",
            "text-x-generic-symbolic",
            "readme markdown docs file",
        ));
    }

    rows
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        for suffix in ["Desktop", "Documents", "Downloads"] {
            roots.push(Path::new(&home).join(suffix));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    roots
}

fn read_dir_entries(path: PathBuf) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    entries.flatten().map(|entry| entry.path()).collect()
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
        _ => "text-x-generic-symbolic",
    };
    icon.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_icons_match_common_extensions() {
        assert_eq!(
            icon_for_path(Path::new("/tmp/photo.png")),
            "image-x-generic-symbolic"
        );
        assert_eq!(
            icon_for_path(Path::new("/tmp/report.pdf")),
            "application-pdf-symbolic"
        );
        assert_eq!(
            icon_for_path(Path::new("/tmp/archive.zip")),
            "package-x-generic-symbolic"
        );
        assert_eq!(
            icon_for_path(Path::new("/tmp/script.rs")),
            "text-x-script-symbolic"
        );
    }
}
