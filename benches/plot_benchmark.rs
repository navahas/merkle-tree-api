use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use reqwest::Client;
use serde_json::json;
use rand::{random, Rng};
use std::time::Duration;
use tokio::time::timeout;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const TEST_LEAVES: usize = 100;

fn setup_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

async fn setup_server() -> (String, Client) {
    let client = Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap();
    ("http://127.0.0.1:8080".to_string(), client)
}

async fn add_leaf_single(client: &Client, base_url: &str, leaf: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaf", base_url))
            .json(&json!({ "leaf": leaf }))
            .send()
    ).await??;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn add_leaves_batch(client: &Client, base_url: &str, leaves: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaves", base_url))
            .json(&json!({ "leaves": leaves }))
            .send()
    ).await??;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn get_root(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client.get(&format!("{}/get-root", base_url)).send()
    ).await??;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn get_proof(client: &Client, base_url: &str, index: usize) -> Result<(), Box<dyn std::error::Error>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/get-proof", base_url))
            .json(&json!({ "index": index }))
            .send()
    ).await??;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }
    Ok(())
}

fn bench_merkle_operations(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());
    
    // Pre-populate tree for root/proof operations
    runtime.block_on(async {
        let leaves: Vec<String> = (0..TEST_LEAVES)
            .map(|_| format!("{:064x}", random::<u64>()))
            .collect();
        let _ = add_leaves_batch(&client, &base_url, &leaves).await;
    });

    let mut group = c.benchmark_group("Merkle Tree Operations");
    
    // Single leaf addition
    group.bench_function("add single leaf", |b| {
        b.iter(|| async {
            let leaf = format!("{:064x}", black_box(random::<u64>()));
            let _ = add_leaf_single(&client, &base_url, &leaf).await;
        })
    });

    // Batch leaf addition (10 leaves)
    group.bench_function("add batch (10 leaves)", |b| {
        b.iter(|| async {
            let leaves: Vec<String> = (0..10)
                .map(|_| format!("{:064x}", random::<u64>()))
                .collect();
            let _ = add_leaves_batch(&client, &base_url, black_box(&leaves)).await;
        })
    });

    // Batch leaf addition (50 leaves)
    group.bench_function("add batch (50 leaves)", |b| {
        b.iter(|| async {
            let leaves: Vec<String> = (0..50)
                .map(|_| format!("{:064x}", random::<u64>()))
                .collect();
            let _ = add_leaves_batch(&client, &base_url, black_box(&leaves)).await;
        })
    });

    // Get root
    group.bench_function("get root", |b| {
        b.iter(|| async {
            let _ = get_root(&client, &base_url).await;
        })
    });

    // Get proof
    group.bench_function("get proof", |b| {
        b.iter(|| async {
            let mut rng = rand::rng();
            let index = black_box(rng.random_range(0..TEST_LEAVES));
            let _ = get_proof(&client, &base_url, index).await;
        })
    });

    group.finish();
}

fn bench_batch_sizes(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    let mut group = c.benchmark_group("Batch Size Comparison");
    
    for &size in &[1, 5, 10, 25, 50, 100] {
        group.bench_function(&format!("batch size {}", size), |b| {
            b.iter(|| async {
                let leaves: Vec<String> = (0..size)
                    .map(|_| format!("{:064x}", random::<u64>()))
                    .collect();
                let _ = add_leaves_batch(&client, &base_url, black_box(&leaves)).await;
            })
        });
    }

    group.finish();
}

fn bench_single_vs_batch(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    let mut group = c.benchmark_group("Single vs Batch");
    
    // 10 single operations
    group.bench_function("10 single operations", |b| {
        b.iter(|| async {
            for _ in 0..10 {
                let leaf = format!("{:064x}", random::<u64>());
                let _ = add_leaf_single(&client, &base_url, black_box(&leaf)).await;
            }
        })
    });

    // 1 batch of 10
    group.bench_function("1 batch of 10", |b| {
        b.iter(|| async {
            let leaves: Vec<String> = (0..10)
                .map(|_| format!("{:064x}", random::<u64>()))
                .collect();
            let _ = add_leaves_batch(&client, &base_url, black_box(&leaves)).await;
        })
    });

    group.finish();
}

criterion_group!(
    benches, 
    bench_merkle_operations,
    bench_batch_sizes,
    bench_single_vs_batch
);
criterion_main!(benches);
