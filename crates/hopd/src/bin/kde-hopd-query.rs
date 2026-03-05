use std::path::PathBuf;

use hopd::kde_adapter::{
    request_hopd_execute_over_socket, request_hopd_search_over_socket_with_mode,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Runner,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut query_parts = Vec::new();
    let mut execute_result_id: Option<String> = None;
    let mut action = "enter".to_string();
    let mut socket_path = default_socket_path();
    let mut limit: u32 = 8;
    let mut mode: Option<String> = None;
    let mut format = OutputFormat::Json;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => {
                if let Some(value) = args.next() {
                    socket_path = PathBuf::from(value);
                }
            }
            "--limit" => {
                if let Some(value) = args.next() {
                    if let Ok(parsed) = value.parse::<u32>() {
                        limit = parsed.max(1);
                    }
                }
            }
            "--execute" => {
                if let Some(value) = args.next() {
                    execute_result_id = Some(value);
                }
            }
            "--mode" => {
                if let Some(value) = args.next() {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        mode = Some(trimmed.to_string());
                    }
                }
            }
            "--format" => {
                if let Some(value) = args.next() {
                    match parse_output_format(&value) {
                        Some(next) => format = next,
                        None => {
                            eprintln!("invalid --format value: {value} (expected json or runner)");
                            std::process::exit(2);
                        }
                    }
                }
            }
            "--action" => {
                if let Some(value) = args.next() {
                    action = value;
                }
            }
            _ => query_parts.push(arg),
        }
    }

    if let Some(result_id) = execute_result_id {
        match request_hopd_execute_over_socket(&socket_path, &result_id, &action).await {
            Ok(response) => {
                println!("{response}");
                return;
            }
            Err(error) => {
                eprintln!("hopd execute failed: {error}");
                std::process::exit(1);
            }
        }
    }

    if query_parts.is_empty() {
        eprintln!(
            "usage: kde-hopd-query [--socket <path>] [--limit <n>] [--mode <all|apps|windows|files|recents|settings|weather|timezone|emoji|calculator|currency>] [--format <json|runner>] <query text>\n       kde-hopd-query [--socket <path>] --execute <result_id> [--action <action>]"
        );
        std::process::exit(2);
    }

    let query = query_parts.join(" ");
    match request_hopd_search_over_socket_with_mode(&socket_path, &query, limit, mode.as_deref())
        .await
    {
        Ok(Some(response)) => match format {
            OutputFormat::Json => println!("{response}"),
            OutputFormat::Runner => print!("{}", format_runner_rows(&response)),
        },
        Ok(None) => {
            eprintln!("query has no utility intent; skipped hopd request (tip: pass --mode all)");
            std::process::exit(3);
        }
        Err(error) => {
            eprintln!("hopd request failed: {error}");
            std::process::exit(1);
        }
    }
}

fn default_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("hopd.sock")
}

fn parse_output_format(raw: &str) -> Option<OutputFormat> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "json" => Some(OutputFormat::Json),
        "runner" => Some(OutputFormat::Runner),
        _ => None,
    }
}

fn format_runner_rows(response: &Value) -> String {
    let Some(rows) = response
        .get("result")
        .and_then(|result| result.get("results"))
        .and_then(Value::as_array)
    else {
        return String::new();
    };

    let mut output = String::new();
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or_default();
        let title = row.get("title").and_then(Value::as_str).unwrap_or_default();
        let subtitle = row
            .get("subtitle")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let icon = row.get("icon").and_then(Value::as_str).unwrap_or_default();
        let kind = row.get("kind").and_then(Value::as_str).unwrap_or_default();
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            sanitize_field(id),
            sanitize_field(title),
            sanitize_field(subtitle),
            sanitize_field(icon),
            sanitize_field(kind)
        ));
    }
    output
}

fn sanitize_field(raw: &str) -> String {
    raw.replace('\t', " ").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_format_recognizes_runner() {
        assert_eq!(parse_output_format("runner"), Some(OutputFormat::Runner));
        assert_eq!(parse_output_format("json"), Some(OutputFormat::Json));
        assert_eq!(parse_output_format("invalid"), None);
    }

    #[test]
    fn format_runner_rows_renders_tsv_lines() {
        let response = serde_json::json!({
            "result": {
                "results": [
                    {
                        "id": "app:firefox.desktop",
                        "title": "Firefox",
                        "subtitle": "Installed application",
                        "icon": "firefox",
                        "kind": "app"
                    }
                ]
            }
        });
        let output = format_runner_rows(&response);
        assert_eq!(
            output,
            "app:firefox.desktop\tFirefox\tInstalled application\tfirefox\tapp\n"
        );
    }

    #[test]
    fn format_runner_rows_sanitizes_tabs_and_newlines() {
        let response = serde_json::json!({
            "result": {
                "results": [
                    {
                        "id": "utility:calculator:2%2B2",
                        "title": "Calc\t2+2",
                        "subtitle": "Line1\nLine2",
                        "icon": "accessories-calculator-symbolic",
                        "kind": "utility"
                    }
                ]
            }
        });
        let output = format_runner_rows(&response);
        assert_eq!(
            output,
            "utility:calculator:2%2B2\tCalc 2+2\tLine1 Line2\taccessories-calculator-symbolic\tutility\n"
        );
    }
}
