use crate::SearchItem;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod apps;
pub mod files;
pub mod recents;
pub mod settings;
pub mod utility;
pub mod windows;

pub struct ProviderCollection {
    pub items: Vec<SearchItem>,
    pub timed_out_providers: Vec<String>,
    pub provider_errors: Vec<Value>,
}

pub fn collect_provider_items_with_budget(
    query: &str,
    mode: &str,
    indexed_folders: &[String],
    timeout_ms: u64,
    debug_provider_delays_ms: &HashMap<String, u64>,
    provider_item_budgets: &HashMap<String, usize>,
) -> ProviderCollection {
    let mut collected = ProviderCollection {
        items: Vec::new(),
        timed_out_providers: Vec::new(),
        provider_errors: Vec::new(),
    };
    let is_empty_query = query.trim().is_empty();
    match mode {
        "apps" => append_provider_with_budget(
            "apps",
            timeout_ms,
            debug_provider_delays_ms,
            provider_item_budgets,
            &mut collected,
            || apps::results(query),
        ),
        "windows" => append_provider_with_budget(
            "windows",
            timeout_ms,
            debug_provider_delays_ms,
            provider_item_budgets,
            &mut collected,
            || windows::results(query),
        ),
        "files" => append_provider_with_budget(
            "files",
            timeout_ms,
            debug_provider_delays_ms,
            provider_item_budgets,
            &mut collected,
            || files::results_with_roots(query, indexed_folders),
        ),
        "recents" => append_provider_with_budget(
            "recents",
            timeout_ms,
            debug_provider_delays_ms,
            provider_item_budgets,
            &mut collected,
            || recents::results(query),
        ),
        "settings" => append_provider_with_budget(
            "settings",
            timeout_ms,
            debug_provider_delays_ms,
            provider_item_budgets,
            &mut collected,
            || settings::results(query),
        ),
        _ => {
            append_provider_with_budget(
                "apps",
                timeout_ms,
                debug_provider_delays_ms,
                provider_item_budgets,
                &mut collected,
                || apps::results(query),
            );
            append_provider_with_budget(
                "windows",
                timeout_ms,
                debug_provider_delays_ms,
                provider_item_budgets,
                &mut collected,
                || windows::results(query),
            );
            if !is_empty_query {
                append_provider_with_budget(
                    "files",
                    timeout_ms,
                    debug_provider_delays_ms,
                    provider_item_budgets,
                    &mut collected,
                    || files::results_with_roots(query, indexed_folders),
                );
            }
            append_provider_with_budget(
                "recents",
                timeout_ms,
                debug_provider_delays_ms,
                provider_item_budgets,
                &mut collected,
                || recents::results(query),
            );
            append_provider_with_budget(
                "settings",
                timeout_ms,
                debug_provider_delays_ms,
                provider_item_budgets,
                &mut collected,
                || settings::results(query),
            );
            append_provider_with_budget(
                "utility",
                timeout_ms,
                debug_provider_delays_ms,
                provider_item_budgets,
                &mut collected,
                || utility::results(query),
            );
        }
    };
    collected
}

fn append_provider_with_budget<F>(
    provider_name: &str,
    timeout_ms: u64,
    debug_provider_delays_ms: &HashMap<String, u64>,
    provider_item_budgets: &HashMap<String, usize>,
    collected: &mut ProviderCollection,
    provider: F,
) where
    F: FnOnce() -> Vec<SearchItem>,
{
    let started_at = Instant::now();
    if let Some(delay_ms) = debug_provider_delays_ms.get(provider_name).copied() {
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    let mut items = provider();
    if let Some(max_items) = provider_item_budgets.get(provider_name).copied() {
        if items.len() > max_items {
            items.truncate(max_items);
        }
    }
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    if timeout_ms > 0 && elapsed_ms > timeout_ms {
        if !collected
            .timed_out_providers
            .iter()
            .any(|row| row == provider_name)
        {
            collected
                .timed_out_providers
                .push(provider_name.to_string());
        }
        return;
    }
    collected.items.extend(items);
}
