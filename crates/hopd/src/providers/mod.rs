use crate::SearchItem;

pub mod apps;
pub mod files;
pub mod recents;
pub mod settings;
pub mod utility;
pub mod windows;

pub fn collect_provider_items(query: &str, mode: &str) -> Vec<SearchItem> {
    match mode {
        "apps" => apps::results(query),
        "windows" => windows::results(query),
        "files" => files::results(query),
        "recents" => recents::results(query),
        "settings" => settings::results(query),
        _ => {
            let mut items = Vec::new();
            items.extend(apps::results(query));
            items.extend(windows::results(query));
            items.extend(files::results(query));
            items.extend(recents::results(query));
            items.extend(settings::results(query));
            items.extend(utility::results(query));
            items
        }
    }
}
