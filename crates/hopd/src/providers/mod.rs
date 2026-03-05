use crate::SearchItem;

pub mod apps;
pub mod files;
pub mod recents;
pub mod settings;
pub mod utility;
pub mod windows;

pub fn collect_provider_items(query: &str, mode: &str, indexed_folders: &[String]) -> Vec<SearchItem> {
    let is_empty_query = query.trim().is_empty();
    match mode {
        "apps" => apps::results(query),
        "windows" => windows::results(query),
        "files" => files::results_with_roots(query, indexed_folders),
        "recents" => recents::results(query),
        "settings" => settings::results(query),
        _ => {
            let mut items = Vec::new();
            items.extend(apps::results(query));
            items.extend(windows::results(query));
            if !is_empty_query {
                items.extend(files::results_with_roots(query, indexed_folders));
            }
            items.extend(recents::results(query));
            items.extend(settings::results(query));
            items.extend(utility::results(query));
            items
        }
    }
}
