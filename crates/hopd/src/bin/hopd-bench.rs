use std::time::Instant;

use hopd::HopdServer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(250);

    let server = HopdServer::new();
    println!("hopd benchmark: {iterations} iterations per query");
    run_case(
        &server,
        "apps",
        r#"{"id":"b1","method":"search.query","params":{"query":"terminal","mode":"apps","limit":8}}"#,
        iterations,
    )
    .await;
    run_case(
        &server,
        "utility",
        r#"{"id":"b2","method":"search.query","params":{"query":"weather zurich","mode":"all","limit":8}}"#,
        iterations,
    )
    .await;
}

async fn run_case(server: &HopdServer, label: &str, request: &str, iterations: usize) {
    let mut samples_us = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let _ = server
            .handle_json_line(request)
            .await
            .expect("benchmark request should serialize");
        samples_us.push(started.elapsed().as_micros() as u64);
    }

    samples_us.sort_unstable();
    let total_us: u128 = samples_us.iter().map(|value| *value as u128).sum();
    let mean_us = total_us as f64 / iterations as f64;
    let p95_index = ((iterations as f64) * 0.95).ceil() as usize - 1;
    let p95_us = samples_us[p95_index];
    let max_us = *samples_us.last().unwrap_or(&0);

    println!(
        "{label}: mean={:.3}ms p95={:.3}ms max={:.3}ms",
        mean_us / 1000.0,
        p95_us as f64 / 1000.0,
        max_us as f64 / 1000.0
    );
}
