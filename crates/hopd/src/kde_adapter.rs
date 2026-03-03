use serde_json::{json, Value};

fn has_utility_intent(query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return false;
    }

    q.contains("weather")
        || q.contains("wx ")
        || q.starts_with("wx")
        || q.contains("timezone")
        || q.starts_with("tz ")
        || q.contains("time ")
        || q.contains("emoji")
        || q.starts_with(":")
}

pub fn build_hopd_search_request(query: &str, limit: u32) -> Option<Value> {
    let trimmed = query.trim();
    if !has_utility_intent(trimmed) {
        return None;
    }

    Some(json!({
        "id": "kde-proto-1",
        "method": "search.query",
        "params": {
            "query": trimmed,
            "limit": limit.max(1),
        }
    }))
}

