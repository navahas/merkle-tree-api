use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use futures::future::join_all;
use rand::{Rng, random};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::time::timeout;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const BASE_URL_HEAP: &str = "http://127.0.0.1:8080";
const BASE_URL_LMDB: &str = "http://127.0.0.1:8080/lmdb";

fn setup_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(8) // More threads for concurrent testing
        .build()
        .unwrap()
}

async fn setup_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(50) // Higher connection pool for concurrency
        .pool_idle_timeout(Duration::from_secs(30))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap()
}

async fn add_leaf_single(
    client: &Client,
    base_url: &str,
    leaf: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaf", base_url))
            .json(&json!({ "leaf": leaf }))
            .send(),
    )
    .await??;

    if !response.status().is_success() {
        return Err(format!("Add leaf failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn add_leaves_batch(
    client: &Client,
    base_url: &str,
    leaves: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaves", base_url))
            .json(&json!({ "leaves": leaves }))
            .send(),
    )
    .await??;

    if !response.status().is_success() {
        return Err(format!("Add leaves failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn get_root(
    client: &Client,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client.get(&format!("{}/get-root", base_url)).send(),
    )
    .await??;

    if !response.status().is_success() && response.status() != 400 {
        // 400 is OK for empty tree
        return Err(format!("Get root failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn get_num_leaves(
    client: &Client,
    base_url: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client.get(&format!("{}/get-num-leaves", base_url)).send(),
    )
    .await??;

    if !response.status().is_success() {
        return Err(format!("Get num leaves failed with status: {}", response.status()).into());
    }

    let json_response: serde_json::Value = response.json().await?;
    Ok(json_response["num_leaves"].as_u64().unwrap_or(0) as usize)
}

async fn get_proof(
    client: &Client,
    base_url: &str,
    index: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/get-proof", base_url))
            .json(&json!({ "index": index }))
            .send(),
    )
    .await??;

    if !response.status().is_success() && response.status() != 400 {
        // 400 might occur during concurrent mutations
        return Err(format!("Get proof failed with status: {}", response.status()).into());
    }
    Ok(())
}

fn generate_test_leaves(count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            // Generate 32 random bytes (256 bits) for a proper hash
            let mut bytes = [0u8; 32];
            for b in &mut bytes {
                *b = random::<u8>();
            }
            hex::encode(bytes)
        })
        .collect()
}

async fn populate_tree(client: &Client, base_url: &str, leaf_count: usize) {
    let leaves = generate_test_leaves(leaf_count);
    let _ = add_leaves_batch(client, base_url, &leaves).await;
}

fn bench_concurrent_reads(c: &mut Criterion) {
    let runtime = setup_runtime();
    let client = runtime.block_on(setup_client());

    let mut group = c.benchmark_group("Concurrent Reads");
    group.sample_size(20);

    // Pre-populate both trees
    runtime.block_on(async {
        populate_tree(&client, BASE_URL_HEAP, 100).await;
        populate_tree(&client, BASE_URL_LMDB, 100).await;
    });

    for &concurrent_requests in &[5, 10, 25, 50] {
        // Concurrent get_root operations - Heap
        group.bench_with_input(
            BenchmarkId::new("heap_concurrent_get_root", concurrent_requests),
            &concurrent_requests,
            |b, &concurrent_requests| {
                b.iter(|| {
                    runtime.block_on(async {
                        let tasks: Vec<_> = (0..concurrent_requests)
                            .map(|_| get_root(&client, BASE_URL_HEAP))
                            .collect();

                        join_all(tasks).await
                    })
                });
            },
        );

        // Concurrent get_root operations - LMDB
        group.bench_with_input(
            BenchmarkId::new("lmdb_concurrent_get_root", concurrent_requests),
            &concurrent_requests,
            |b, &concurrent_requests| {
                b.iter(|| {
                    runtime.block_on(async {
                        let tasks: Vec<_> = (0..concurrent_requests)
                            .map(|_| get_root(&client, BASE_URL_LMDB))
                            .collect();

                        join_all(tasks).await
                    })
                });
            },
        );

        // Concurrent get_proof operations - Heap
        group.bench_with_input(
            BenchmarkId::new("heap_concurrent_get_proof", concurrent_requests),
            &concurrent_requests,
            |b, &concurrent_requests| {
                b.iter(|| {
                    runtime.block_on(async {
                        let mut rng = rand::rng();
                        let tasks: Vec<_> = (0..concurrent_requests)
                            .map(|_| {
                                let index = rng.random_range(0..100);
                                get_proof(&client, BASE_URL_HEAP, index)
                            })
                            .collect();

                        join_all(tasks).await
                    })
                });
            },
        );

        // Concurrent get_proof operations - LMDB
        group.bench_with_input(
            BenchmarkId::new("lmdb_concurrent_get_proof", concurrent_requests),
            &concurrent_requests,
            |b, &concurrent_requests| {
                b.iter(|| {
                    runtime.block_on(async {
                        let mut rng = rand::rng();
                        let tasks: Vec<_> = (0..concurrent_requests)
                            .map(|_| {
                                let index = rng.random_range(0..100);
                                get_proof(&client, BASE_URL_LMDB, index)
                            })
                            .collect();

                        join_all(tasks).await
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(api_benches, bench_concurrent_reads);
criterion_main!(api_benches);
