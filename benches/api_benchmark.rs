use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use reqwest::Client;
use serde_json::json;
use rand::{random, Rng};
use std::time::Duration;
use tokio::time::timeout;

const LEAVES_FOR_ROOT: usize = 50;
const LEAVES_FOR_PROOF: usize = 100;
const BATCH_SIZE: &[usize] = &[10, 50, 100, 200];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// NOTE: Recreating a reqwest::Client for each request led to socket exhaustion,
// producing cryptic errors like "Can't assign requested address" on macOS.
// The official documentation recommends reusing a single client.
// See: https://docs.rs/reqwest/latest/reqwest/#creating-a-client
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

fn custom_criterion() -> Criterion {
    Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(2)) // Increased for stability
}

async fn add_leaf_request(client: &Client, base_url: &str, leaf: &str) -> Result<(), Box<dyn std::error::Error>> {
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

async fn add_leaves_request(client: &Client, base_url: &str, leaves: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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

async fn get_root_request(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = timeout(
        REQUEST_TIMEOUT,
        client
            .get(&format!("{}/get-root", base_url))
            .send()
    ).await??;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()).into());
    }
    Ok(())
}

async fn get_proof_request(client: &Client, base_url: &str, index: usize) -> Result<(), Box<dyn std::error::Error>> {
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

fn benchmark_add_leaf(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    c.bench_function("add_leaf", |b| {
        b.iter(|| async {
            let leaf = format!("{:064x}", black_box(random::<u64>()));
            if let Err(e) = add_leaf_request(&client, &base_url, &leaf).await {
                eprintln!("add_leaf request failed: {}", e);
            }
        });
    });
}

fn benchmark_add_leaves_batch(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    for &batch_size in BATCH_SIZE {
        c.bench_with_input(
            BenchmarkId::new("add_leaves_batch", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| async {
                    let leaves: Vec<String> = (0..batch_size)
                        .map(|_| format!("{:064x}", random::<u64>()))
                        .collect();
                    if let Err(e) = add_leaves_request(&client, &base_url, &leaves).await {
                        eprintln!("add_leaves request failed: {}", e);
                    }
                });
            },
        );
    }
}

fn benchmark_get_root(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    runtime.block_on(async {
        let leaves: Vec<String> = (0..LEAVES_FOR_ROOT)
            .map(|_| format!("{:064x}", random::<u64>()))
            .collect();
        if let Err(e) = add_leaves_request(&client, &base_url, &leaves).await {
            eprintln!("Setup failed for get_root benchmark: {}", e);
        }
    });

    c.bench_function("get_root", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ =get_root_request(&client, &base_url).await;
            })
        });
    });
}

fn benchmark_get_proof(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    runtime.block_on(async {
        let leaves: Vec<String> = (0..LEAVES_FOR_PROOF)
            .map(|_| format!("{:064x}", random::<u64>()))
            .collect();
        if let Err(e) = add_leaves_request(&client, &base_url, &leaves).await {
            eprintln!("Setup failed for get_proof benchmark: {}", e);
        }
    });

    for &tree_size in BATCH_SIZE {
        c.bench_with_input(
            BenchmarkId::new("get_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter(|| {
                    runtime.block_on(async {
                        let mut rng = rand::rng();
                        let index = black_box(rng.random_range(0..tree_size));
                        let _ = get_proof_request(&client, &base_url, index).await;
                    })
                });
            },
        );
    }
}

criterion_group!(
    name = benches;
    config = custom_criterion();
    targets = 
        benchmark_add_leaf,
        benchmark_add_leaves_batch,
        benchmark_get_root,
        benchmark_get_proof
);
criterion_main!(benches);
