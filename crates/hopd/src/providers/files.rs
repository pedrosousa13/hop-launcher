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
            Some(SearchItem::new(
                &format!("file:{full}"),
                "file",
                &display,
                "Filesystem",
                "text-x-generic-symbolic",
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
