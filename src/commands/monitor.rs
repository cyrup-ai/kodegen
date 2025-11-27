use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Deserialize)]
#[allow(dead_code)]
struct HealthResponse {
    timestamp: String,
    status: String,
    requests_processed: u64,
    memory_used: u64,
}

struct ServerSnapshot {
    memory: u64,
    requests: u64,
    timestamp: Instant,
}

pub async fn handle_monitor(interval_secs: u64) -> Result<()> {
    let servers = vec![
        ("filesystem", 30438),
        ("terminal", 30439),
        ("process", 30440),
        ("git", 30441),
        ("github", 30442),
        ("browser", 30443),
        ("citescrape", 30444),
        ("database", 30445),
        ("config", 30446),
        ("prompt", 30447),
        ("reasoner", 30448),
        ("sequential-thinking", 30449),
        ("introspection", 30450),
        ("claude-agent", 30451),
        ("candle-agent", 30452),
    ];

    let client = reqwest::Client::new();
    let mut snapshots: HashMap<String, ServerSnapshot> = HashMap::new();

    loop {
        for (i, (name, port)) in servers.iter().enumerate() {
            // Stagger by 2 seconds per server
            if i > 0 {
                sleep(Duration::from_secs(2)).await;
            }

            let url = format!("http://localhost:{}/mcp/health", port);
            let response = match client.get(&url).send().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let health: HealthResponse = match response.json().await {
                Ok(h) => h,
                Err(_) => continue,
            };

            let now = Instant::now();

            if let Some(prev) = snapshots.get(*name) {
                let memory_growth = health.memory_used.saturating_sub(prev.memory);
                let threshold = 100 * 1024 * 1024;

                if memory_growth >= threshold {
                    let elapsed = now.duration_since(prev.timestamp);
                    let requests_delta = health.requests_processed.saturating_sub(prev.requests);

                    println!(
                        "[{}] Memory growth: {} over {:?} ({} requests)",
                        name,
                        format_bytes(memory_growth),
                        elapsed,
                        requests_delta
                    );
                }
            }

            snapshots.insert(
                name.to_string(),
                ServerSnapshot {
                    memory: health.memory_used,
                    requests: health.requests_processed,
                    timestamp: now,
                },
            );
        }

        sleep(Duration::from_secs(interval_secs)).await;
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{} bytes", bytes)
    }
}
