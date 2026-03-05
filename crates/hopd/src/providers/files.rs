use std::fs;
use std::path::{Path, PathBuf};

use crate::SearchItem;

pub fn results_with_roots(query: &str, indexed_roots: &[String]) -> Vec<SearchItem> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut roots = candidate_roots();
    roots.extend(
        indexed_roots
            .iter()
            .map(|root| root.trim())
            .filter(|root| !root.is_empty())
            .map(PathBuf::from),
    );

    let mut rows = collect_candidate_files(&roots, 3)
        .into_iter()
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

fn collect_candidate_files(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = roots
        .iter()
        .cloned()
        .map(|path| (path, 0))
        .collect();
    let mut seen_dirs = std::collections::HashSet::new();

    while let Some((path, depth)) = stack.pop() {
        let key = path.to_string_lossy().to_string();
        if !seen_dirs.insert(key) {
            continue;
        }

        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if depth < max_depth {
                    stack.push((entry_path, depth + 1));
                }
                continue;
            }
            out.push(entry_path);
            if out.len() >= 5000 {
                return out;
            }
        }
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
        _ => "text-x-generic-symbolic",
    };
    icon.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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

    #[test]
    fn results_with_roots_finds_nested_file_in_indexed_folder() {
        let temp_dir = std::env::temp_dir().join(format!(
            "hopd-files-provider-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        let nested_dir = temp_dir.join("projects").join("nested");
        std::fs::create_dir_all(&nested_dir).expect("create nested dirs");
        let file_path = nested_dir.join("roadmap-notes.txt");
        let mut file = std::fs::File::create(&file_path).expect("create file");
        writeln!(file, "notes").expect("write");

        let rows = results_with_roots(
            "roadmap-notes",
            &[temp_dir.to_string_lossy().to_string()],
        );

        assert!(
            rows.iter().any(|row| row.id.contains("roadmap-notes.txt")),
            "expected nested indexed-folder file in results"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
