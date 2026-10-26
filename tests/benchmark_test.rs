use axum::body::Body;
use axum::http::{Request, StatusCode};
use router9::api::build_router;
use router9::db::{init_memory_pool, migrate};
use router9::routing::RoutingEngine;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceExt;

#[tokio::test]
async fn test_benchmark_in_process() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    let total_requests = 1000;
    let concurrency = 50;
    let requests_per_worker = total_requests / concurrency;

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..concurrency {
        let app_clone = app.clone();
        handles.push(tokio::spawn(async move {
            let mut latencies = Vec::with_capacity(requests_per_worker);
            for _ in 0..requests_per_worker {
                let req_start = Instant::now();
                let req = Request::builder()
                    .uri("/api/health")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let resp = app_clone.clone().oneshot(req).await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                latencies.push(req_start.elapsed().as_secs_f64() * 1000.0);
            }
            latencies
        }));
    }

    let mut all_latencies = Vec::with_capacity(total_requests);
    for h in handles {
        let l = h.await.unwrap();
        all_latencies.extend(l);
    }

    let elapsed = start.elapsed();
    let throughput = (total_requests as f64) / elapsed.as_secs_f64();
    all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = all_latencies[all_latencies.len() / 2];

    println!(
        "[In-Process Benchmark @ c={}] Throughput: {:.0} req/s, p50: {:.3}ms",
        concurrency, throughput, p50
    );

    assert!(throughput > 2000.0);
}

#[tokio::test]
async fn test_benchmark_real_tcp_loopback() {
    let pool = init_memory_pool().expect("DB pool");
    let conn = pool.get().expect("conn");
    migrate(&conn).expect("migrate");
    drop(conn);

    let engine = Arc::new(RoutingEngine::new(pool));
    let app = build_router(engine);

    // Bind real TCP listener on ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .build()
        .unwrap();

    let total_requests = 1000;
    let concurrency = 50;
    let requests_per_worker = total_requests / concurrency;
    let url = format!("http://{}/api/health", local_addr);

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..concurrency {
        let c = client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(async move {
            let mut latencies = Vec::with_capacity(requests_per_worker);
            for _ in 0..requests_per_worker {
                let req_start = Instant::now();
                let resp = c.get(&u).send().await.unwrap();
                assert!(resp.status().is_success());
                latencies.push(req_start.elapsed().as_secs_f64() * 1000.0);
            }
            latencies
        }));
    }

    let mut all_latencies = Vec::with_capacity(total_requests);
    for h in handles {
        let l = h.await.unwrap();
        all_latencies.extend(l);
    }

    let elapsed = start.elapsed();
    let throughput = (total_requests as f64) / elapsed.as_secs_f64();
    all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = all_latencies[all_latencies.len() / 2];
    let p95 = all_latencies[(all_latencies.len() as f64 * 0.95) as usize];
    let p99 = all_latencies[(all_latencies.len() as f64 * 0.99) as usize];

    println!(
        "[Real TCP Loopback Benchmark @ c={}] Total: {}, Elapsed: {:.2?}, Throughput: {:.0} req/s, p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms",
        concurrency, total_requests, elapsed, throughput, p50, p95, p99
    );

    assert!(throughput > 1000.0, "Throughput must exceed 1000 req/s, got {:.0}", throughput);
    assert!(p50 < 50.0, "p50 latency must be under 50ms, got {:.2}ms", p50);

    // Verify raw-results.json exists and contains benchmark comparisons
    let raw_results_str = std::fs::read_to_string("raw-results.json").expect("read raw-results.json");
    let val: serde_json::Value = serde_json::from_str(&raw_results_str).expect("parse JSON");
    let comparison = val["comparison"].as_array().expect("comparison array");
    assert_eq!(comparison.len(), 3);
    assert_eq!(comparison[0]["name"], "9Router-Rust");
    assert_eq!(comparison[1]["name"], "9Router (Next.js)");
    assert_eq!(comparison[2]["name"], "OmniRoute");
}
