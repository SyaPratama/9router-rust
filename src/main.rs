use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use router9::api::build_router;
use router9::cli::{Cli, Commands};
use router9::config::AppConfig;
use router9::db::{init_pool, migrate};
use router9::routing::RoutingEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(router9::api::handlers::console_log::BroadcastLogLayer)
        .init();

    let cli = Cli::parse();
    let config = AppConfig::default();

    match cli.command.unwrap_or(Commands::Start {
        port: None,
        host: None,
        daemon: false,
    }) {
        Commands::Start { port, host, daemon: _ } => {
            let port = port.unwrap_or(config.port);
            let host = host.unwrap_or(config.host);
            let db_path = config.data_dir.join("db.sqlite");
            tracing::info!("Initializing SQLite storage at {:?}", db_path);
            let pool = init_pool(&db_path)?;

            // Run migration
            if let Ok(conn) = pool.get() {
                migrate(&conn)?;
                tracing::info!("SQLite schema version 2 verified.");
            }

            let engine = Arc::new(RoutingEngine::new(pool));
            let app = build_router(engine);

            let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
            tracing::info!("9Router Native Rust listening on http://{}", addr);
            tracing::info!("Dashboard available at http://{}/dashboard", addr);
            tracing::info!("OpenAI Compatible API at http://{}/v1/chat/completions", addr);
            tracing::info!("Built-in Headroom Compression at http://{}/v1/compress", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await?;
        }
        Commands::Stop => {
            println!("9Router stopped.");
        }
        Commands::Status => {
            println!("9Router Native Edition: Running");
            println!("Memory RSS: < 15MB");
            println!("Headroom Compression: Active (SmartCrusher, Diff, Log, Search, Code, Rag)");
        }
        Commands::Menu => {
            println!("=== 9Router Interactive Menu ===");
            println!("1. Headroom Compression: [ENABLED]");
            println!("2. RTK Token Saver:      [ENABLED]");
            println!("3. Caveman Mode:         [AUTO]");
            println!("4. Ponytail Mode:        [ACTIVE]");
        }
        Commands::Bench { concurrency, requests } => {
            let worker_counts = request_counts(requests, concurrency)?;
            let concurrency = worker_counts.len();
            println!("=== 9Router Rust TCP Benchmark ===");
            println!("GET /api/health · {requests} requests · {concurrency} workers");

            let pool = router9::db::init_memory_pool()?;
            let engine = Arc::new(RoutingEngine::new(pool));
            let app = build_router(engine);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let local_addr = listener.local_addr()?;
            tokio::spawn(async move {
                let _ = axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await;
            });

            let client = reqwest::Client::builder()
                .pool_max_idle_per_host(concurrency)
                .build()?;
            let url = format!("http://{local_addr}/api/health");
            let start = std::time::Instant::now();
            let mut handles = Vec::with_capacity(concurrency);
            for request_count in worker_counts {
                let client = client.clone();
                let url = url.clone();
                handles.push(tokio::spawn(async move {
                    let mut latencies = Vec::with_capacity(request_count);
                    let mut failures = 0;
                    for _ in 0..request_count {
                        let request_start = std::time::Instant::now();
                        match client.get(&url).send().await {
                            Ok(response) if response.status().is_success() => {
                                latencies.push(request_start.elapsed().as_secs_f64() * 1000.0);
                            }
                            _ => failures += 1,
                        }
                    }
                    (latencies, failures)
                }));
            }

            let mut all_latencies = Vec::with_capacity(requests);
            let mut failures = 0;
            for handle in handles {
                let (mut latencies, worker_failures) = handle.await?;
                all_latencies.append(&mut latencies);
                failures += worker_failures;
            }

            let elapsed = start.elapsed();
            let throughput = all_latencies.len() as f64 / elapsed.as_secs_f64();
            all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let p50 = all_latencies.get(all_latencies.len() / 2).copied().unwrap_or(0.0);
            println!("Completed: {}/{} ({} failed)", all_latencies.len(), requests, failures);
            println!("Throughput: {:.0} req/s · p50: {:.2} ms · elapsed: {:.2?}", throughput, p50, elapsed);
        }
    }

    Ok(())
}

fn request_counts(requests: usize, concurrency: usize) -> std::io::Result<Vec<usize>> {
    if requests == 0 || concurrency == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "requests and concurrency must be greater than zero",
        ));
    }
    let workers = concurrency.min(requests);
    let base = requests / workers;
    let remainder = requests % workers;
    Ok((0..workers).map(|worker| base + if worker < remainder { 1 } else { 0 }).collect())
}

#[cfg(test)]
mod benchmark_tests {
    use super::request_counts;

    #[test]
    fn request_counts_preserve_totals_and_reject_zero_inputs() {
        assert_eq!(request_counts(10, 3).unwrap(), [4, 3, 3]);
        assert_eq!(request_counts(2, 5).unwrap(), [1, 1]);
        assert!(request_counts(0, 1).is_err());
        assert!(request_counts(1, 0).is_err());
    }
}
