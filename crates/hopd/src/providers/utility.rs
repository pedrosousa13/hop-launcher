use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let lower = query.trim().to_lowercase();
    let explicit_utility_intent = lower.contains("utility") || lower.contains("utilities");
    if !explicit_utility_intent {
        return Vec::new();
    }
    let terms: Vec<&str> = lower
        .split_whitespace()
        .filter(|term| !matches!(*term, "utility" | "utilities"))
        .collect();

    utility_catalog()
        .into_iter()
        .filter(|item| {
            if terms.is_empty() {
                return true;
            }
            let haystack = format!("{} {}", item.title.to_lowercase(), item.keywords.to_lowercase());
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

fn utility_catalog() -> Vec<SearchItem> {
    vec![
        SearchItem::new(
            "utility:calculator",
            "utility",
            "Calculator",
            "Run quick calculations",
            "accessories-calculator-symbolic",
            "utility utilities calculator math expression evaluate",
        ),
        SearchItem::new(
            "utility:currency",
            "utility",
            "Currency Converter",
            "Convert money values",
            "preferences-system-time-symbolic",
            "utility utilities currency conversion exchange rates money",
        ),
        SearchItem::new(
            "utility:weather",
            "utility",
            "Weather",
            "Show current weather",
            "weather-overcast-symbolic",
            "utility utilities weather forecast temperature",
        ),
        SearchItem::new(
            "utility:timezone",
            "utility",
            "World Clock",
            "Lookup time in cities",
            "alarm-symbolic",
            "utility utilities timezone world clock city time",
        ),
        SearchItem::new(
            "utility:emoji",
            "utility",
            "Emoji Search",
            "Find emojis by name",
            "face-smile-symbolic",
            "utility utilities emoji symbols picker",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_full_catalog_for_generic_utility_query() {
        let rows = results("utilities");
        assert!(rows.len() >= 5);
        assert!(rows.iter().any(|row| row.id == "utility:calculator"));
        assert!(rows.iter().all(|row| row.kind == "utility"));
    }

    #[test]
    fn filters_utility_catalog_for_specific_term() {
        let rows = results("utility weather");
        assert!(rows.iter().any(|row| row.id == "utility:weather"));
        assert!(!rows.iter().any(|row| row.id == "utility:calculator"));
    }
}
