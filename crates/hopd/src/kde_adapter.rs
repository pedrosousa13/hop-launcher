use std::io;
use std::path::Path;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

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

pub fn build_hopd_execute_request(result_id: &str, action: &str) -> Value {
    json!({
        "id": "kde-proto-exec-1",
        "method": "actions.execute",
        "params": {
            "result_id": result_id,
            "action": action,
        }
    })
}

pub async fn request_hopd_search_over_socket<P: AsRef<Path>>(
    socket_path: P,
    query: &str,
    limit: u32,
) -> io::Result<Option<Value>> {
    let request = match build_hopd_search_request(query, limit) {
        Some(request) => request,
        None => return Ok(None),
    };

    send_json_request_over_socket(socket_path, &request).await.map(Some)
}

pub async fn request_hopd_execute_over_socket<P: AsRef<Path>>(
    socket_path: P,
    result_id: &str,
    action: &str,
) -> io::Result<Value> {
    let request = build_hopd_execute_request(result_id, action);
    send_json_request_over_socket(socket_path, &request).await
}

async fn send_json_request_over_socket<P: AsRef<Path>>(
    socket_path: P,
    request: &Value,
) -> io::Result<Value> {
    let mut stream = UnixStream::connect(socket_path).await?;
    let request_line = serde_json::to_string(request)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    stream.write_all(request_line.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    let mut response_line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut response_line).await?;
    if response_line.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "empty hopd response",
        ));
    }

    serde_json::from_str::<Value>(response_line.trim())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}
