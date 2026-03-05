use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use hopd::HopdServer;
use serde_json::{json, Value};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(250);

    let server = HopdServer::new();
    println!("hopd benchmark: {iterations} warm iterations per query");
    run_case(
        &server,
        "apps",
        json!({"id":"b1","method":"search.query","params":{"query":"terminal","mode":"apps","limit":8}}),
        iterations,
    )
    .await;
    run_case(
        &server,
        "utility",
        json!({"id":"b2","method":"search.query","params":{"query":"weather zurich","mode":"all","limit":8}}),
        iterations,
    )
    .await;

    for file_count in [200usize, 1200usize] {
        let bench_root = create_file_bench_root(file_count).expect("create file bench tree");
        set_indexed_folders(&server, &[bench_root.to_string_lossy().to_string()]).await;
        run_case(
            &server,
            &format!("files-{file_count}"),
            json!({"id":"bf","method":"search.query","params":{"query":"benchdoc","mode":"files","limit":8}}),
            iterations,
        )
        .await;
        let _ = fs::remove_dir_all(&bench_root);
    }
}

async fn run_case(server: &HopdServer, label: &str, request: Value, iterations: usize) {
    let request_line =
        serde_json::to_string(&request).expect("benchmark request should serialize to string");
    let cold_started = Instant::now();
    let cold_response = server
        .handle_json_line(&request_line)
        .await
        .expect("cold benchmark request should succeed");
    let cold_wall_us = cold_started.elapsed().as_micros() as u64;
    let cold_parsed: Value = serde_json::from_str(&cold_response).expect("cold response json");
    let cold_elapsed_ms = cold_parsed["result"]["telemetry"]["elapsed_ms"]
        .as_u64()
        .unwrap_or(0);

    let mut wall_samples_us = Vec::with_capacity(iterations);
    let mut daemon_elapsed_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let response = server
            .handle_json_line(&request_line)
            .await
            .expect("warm benchmark request should succeed");
        wall_samples_us.push(started.elapsed().as_micros() as u64);
        let parsed: Value = serde_json::from_str(&response).expect("warm response json");
        daemon_elapsed_ms.push(parsed["result"]["telemetry"]["elapsed_ms"].as_u64().unwrap_or(0));
    }

    let (wall_mean_ms, wall_p95_ms, wall_max_ms) = summarize_us(&mut wall_samples_us);
    let (daemon_mean_ms, daemon_p95_ms, daemon_max_ms) = summarize_ms(&mut daemon_elapsed_ms);

    println!(
        "{label}: cold wall={:.3}ms daemon={}ms | warm wall mean={:.3}ms p95={:.3}ms max={:.3}ms | daemon mean={:.3}ms p95={:.3}ms max={:.3}ms",
        cold_wall_us as f64 / 1000.0,
        cold_elapsed_ms,
        wall_mean_ms,
        wall_p95_ms,
        wall_max_ms,
        daemon_mean_ms,
        daemon_p95_ms,
        daemon_max_ms
    );
}

fn summarize_us(samples: &mut [u64]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    samples.sort_unstable();
    let total_us: u128 = samples.iter().map(|value| *value as u128).sum();
    let mean_us = total_us as f64 / samples.len() as f64;
    let p95_index = ((samples.len() as f64) * 0.95).ceil() as usize - 1;
    let p95_us = samples[p95_index];
    let max_us = *samples.last().unwrap_or(&0);
    (mean_us / 1000.0, p95_us as f64 / 1000.0, max_us as f64 / 1000.0)
}

fn summarize_ms(samples: &mut [u64]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    samples.sort_unstable();
    let total_ms: u128 = samples.iter().map(|value| *value as u128).sum();
    let mean_ms = total_ms as f64 / samples.len() as f64;
    let p95_index = ((samples.len() as f64) * 0.95).ceil() as usize - 1;
    let p95_ms = samples[p95_index] as f64;
    let max_ms = *samples.last().unwrap_or(&0) as f64;
    (mean_ms, p95_ms, max_ms)
}

async fn set_indexed_folders(server: &HopdServer, folders: &[String]) {
    let request = json!({
        "id": "cfg-indexed-folders",
        "method": "config.set",
        "params": {
            "key": "search.indexed_folders",
            "value": folders,
        }
    });
    let request_line =
        serde_json::to_string(&request).expect("config request should serialize to string");
    let _ = server
        .handle_json_line(&request_line)
        .await
        .expect("config.set should succeed");
}

fn create_file_bench_root(file_count: usize) -> std::io::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "hopd-bench-files-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&root)?;
    for index in 0..file_count {
        let group_dir = root.join(format!("group-{}", index / 200));
        fs::create_dir_all(&group_dir)?;
        let file_path = group_dir.join(format!("benchdoc-{index:04}.txt"));
        write_file(&file_path, index)?;
    }
    Ok(root)
}

fn write_file(path: &Path, index: usize) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "bench document {index}")?;
    Ok(())
}
