use std::path::PathBuf;

use hopd::kde_adapter::request_hopd_search_over_socket;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut query_parts = Vec::new();
    let mut socket_path = default_socket_path();
    let mut limit: u32 = 8;

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
            _ => query_parts.push(arg),
        }
    }

    if query_parts.is_empty() {
        eprintln!("usage: kde-hopd-query [--socket <path>] [--limit <n>] <query text>");
        std::process::exit(2);
    }

    let query = query_parts.join(" ");
    match request_hopd_search_over_socket(&socket_path, &query, limit).await {
        Ok(Some(response)) => println!("{response}"),
        Ok(None) => {
            eprintln!("query has no utility intent; skipped hopd request");
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
