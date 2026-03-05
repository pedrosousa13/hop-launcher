use std::path::PathBuf;

use hopd::kde_adapter::{
    request_hopd_execute_over_socket, request_hopd_search_over_socket_with_mode,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut query_parts = Vec::new();
    let mut execute_result_id: Option<String> = None;
    let mut action = "enter".to_string();
    let mut socket_path = default_socket_path();
    let mut limit: u32 = 8;
    let mut mode: Option<String> = None;

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
            "usage: kde-hopd-query [--socket <path>] [--limit <n>] [--mode <all|apps|windows|files|recents|settings|weather|timezone|emoji|calculator|currency>] <query text>\n       kde-hopd-query [--socket <path>] --execute <result_id> [--action <action>]"
        );
        std::process::exit(2);
    }

    let query = query_parts.join(" ");
    match request_hopd_search_over_socket_with_mode(&socket_path, &query, limit, mode.as_deref())
        .await
    {
        Ok(Some(response)) => println!("{response}"),
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
