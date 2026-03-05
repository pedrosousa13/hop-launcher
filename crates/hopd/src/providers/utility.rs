use crate::SearchItem;

pub fn results(query: &str) -> Vec<SearchItem> {
    let lower = query.trim().to_lowercase();
    let Some(utility_query) = utility_query_body(&lower) else {
        return Vec::new();
    };
    if utility_query.is_empty() {
        return utility_catalog();
    }

    if looks_like_math(utility_query) {
        return vec![SearchItem::new(
            &format!("utility:calculator:{}", utility_query),
            "utility",
            &format!("Calculator: {}", utility_query),
            "Run quick calculations",
            "accessories-calculator-symbolic",
            "utility calculator math expression evaluate",
        )];
    }

    if let Some((amount, src, dst)) = parse_currency_query(utility_query) {
        return vec![SearchItem::new(
            &format!("utility:currency:{amount}:{src}:{dst}"),
            "utility",
            &format!("{amount} {src} -> {dst}"),
            "Currency conversion",
            "preferences-system-time-symbolic",
            "utility currency conversion exchange rates money",
        )];
    }

    if let Some(location) = extract_weather_location(utility_query) {
        return vec![SearchItem::new(
            &format!("utility:weather:{}", encode_component(&location)),
            "utility",
            &format!("Weather in {}", title_case(&location)),
            "Show current weather",
            "weather-overcast-symbolic",
            "utility weather forecast temperature",
        )];
    }
    if utility_query == "weather" || utility_query == "wx" {
        return vec![SearchItem::new(
            "utility:weather",
            "utility",
            "Weather",
            "Show current weather",
            "weather-overcast-symbolic",
            "utility weather forecast temperature",
        )];
    }

    if let Some(location) = extract_time_location(utility_query) {
        return vec![SearchItem::new(
            "utility:timezone",
            "utility",
            &format!("Time in {}", title_case(&location)),
            "Lookup time in cities",
            "alarm-symbolic",
            "utility timezone world clock city time",
        )];
    }

    if utility_query.contains("emoji") || utility_query.starts_with(':') {
        return vec![SearchItem::new(
            "utility:emoji",
            "utility",
            "Emoji Search",
            "Find emojis by name",
            "face-smile-symbolic",
            "utility emoji symbols picker",
        )];
    }

    let terms: Vec<&str> = utility_query.split_whitespace().collect();

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

fn utility_query_body(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    if trimmed == "utility" || trimmed == "utilities" {
        return Some("");
    }
    if let Some(rest) = trimmed.strip_prefix("utility ") {
        return Some(rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("utilities ") {
        return Some(rest.trim());
    }
    None
}

fn looks_like_math(query: &str) -> bool {
    !query.is_empty()
        && query.chars().any(|ch| ch.is_ascii_digit())
        && query
            .chars()
            .all(|ch| ch.is_ascii_digit() || "+-*/(). %".contains(ch))
}

fn parse_currency_query(query: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.len() != 4 || parts[2] != "to" {
        return None;
    }
    let amount = parts[0];
    let src = parts[1];
    let dst = parts[3];
    let amount_ok =
        amount.chars().all(|ch| ch.is_ascii_digit() || ch == '.') && amount.contains(|ch: char| ch.is_ascii_digit());
    let src_ok = src.chars().all(|ch| ch.is_ascii_alphabetic()) && src.len() == 3;
    let dst_ok = dst.chars().all(|ch| ch.is_ascii_alphabetic()) && dst.len() == 3;
    if !amount_ok || !src_ok || !dst_ok {
        return None;
    }
    Some((amount.to_string(), src.to_uppercase(), dst.to_uppercase()))
}

fn extract_weather_location(query: &str) -> Option<String> {
    if let Some(rest) = query.strip_prefix("weather ") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = query.strip_prefix("wx ") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = query.strip_suffix(" weather") {
        return Some(rest.trim().to_string());
    }
    None
}

fn extract_time_location(query: &str) -> Option<String> {
    for prefix in ["time in ", "time ", "tz ", "timezone "] {
        if let Some(rest) = query.strip_prefix(prefix) {
            return Some(rest.trim().to_string());
        }
    }
    if let Some(rest) = query.strip_suffix(" time") {
        return Some(rest.trim().to_string());
    }
    None
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            let mut chars = token.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn encode_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else if ch == ' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
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

    #[test]
    fn parses_utility_currency_query() {
        let rows = results("utility 12 usd to chf");
        assert!(rows.iter().any(|row| row.id == "utility:currency:12:USD:CHF"));
    }

    #[test]
    fn parses_utility_weather_city_query() {
        let rows = results("utility weather zurich");
        assert!(rows.iter().any(|row| row.id == "utility:weather:zurich"));
    }
}
