use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::{Rng, random};
use reqwest::Client;
use serde_json::json;
use std::hint::black_box;
use std::time::Duration;
use tokio::time::timeout;

const LEAVES_FOR_ROOT: usize = 50;
const LEAVES_FOR_PROOF: usize = 100;
const BATCH_SIZE: &[usize] = &[10, 50, 100, 200];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// utility to generate random 64-byte hex leaves
fn generate_leaves(n: usize) -> Vec<String> {
    (0..n)
        .map(|_| format!("{:064x}", random::<u64>()))
        .collect()
}

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
        .measurement_time(Duration::from_secs(2))
        .configure_from_args()
}

async fn add_leaf(
    client: &Client,
    base_url: &str,
    leaf: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaf", base_url))
            .json(&json!({ "leaf": leaf }))
            .send(),
    )
    .await??;
    Ok(())
}

async fn add_leaves(
    client: &Client,
    base_url: &str,
    leaves: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/add-leaves", base_url))
            .json(&json!({ "leaves": leaves }))
            .send(),
    )
    .await??;
    Ok(())
}

async fn get_root(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    timeout(
        REQUEST_TIMEOUT,
        client.get(&format!("{}/get-root", base_url)).send(),
    )
    .await??;
    Ok(())
}

async fn get_proof(
    client: &Client,
    base_url: &str,
    index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    timeout(
        REQUEST_TIMEOUT,
        client
            .post(&format!("{}/get-proof", base_url))
            .json(&json!({ "index": index }))
            .send(),
    )
    .await??;
    Ok(())
}

/// Benchmark: POST /add-leaf
fn bench_add_leaf(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());
    let mut group = c.benchmark_group("API: POST /add-leaf");

    group.bench_function("add_leaf_single", |b| {
        b.iter(|| async {
            let leaf = format!("{:064x}", black_box(random::<u64>()));
            let _ = add_leaf(&client, &base_url, &leaf).await;
        });
    });

    group.finish();
}

/// Benchmark: POST /add-leaves with varying batch sizes
fn bench_add_leaves_batch(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());
    let mut group = c.benchmark_group("API: POST /add-leaves");

    for &batch_size in BATCH_SIZE {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("add_leaves_batch", batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| async {
                    let leaves = generate_leaves(size);
                    let _ = add_leaves(&client, &base_url, black_box(&leaves)).await;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: GET /get-root (after fixed-size tree setup)
fn bench_get_root(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    // Preload leaves once before the benchmark
    runtime.block_on(async {
        let _ = add_leaves(&client, &base_url, &generate_leaves(LEAVES_FOR_ROOT)).await;
    });

    let mut group = c.benchmark_group("API: GET /get-root");

    group.bench_function("get_root_fixed_tree", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ = get_root(&client, &base_url).await;
            })
        });
    });

    group.finish();
}

/// Benchmark: POST /get-proof (tree sizes: 10 to 200)
fn bench_get_proof(c: &mut Criterion) {
    let runtime = setup_runtime();
    let (base_url, client) = runtime.block_on(setup_server());

    // Setup a large enough tree once for all proof sizes
    runtime.block_on(async {
        let _ = add_leaves(&client, &base_url, &generate_leaves(LEAVES_FOR_PROOF)).await;
    });

    let mut group = c.benchmark_group("API: POST /get-proof");

    for &tree_size in BATCH_SIZE {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("get_proof_at", tree_size),
            &tree_size,
            |b, &size| {
                b.iter(|| {
                    runtime.block_on(async {
                        let mut rng = rand::rng();
                        let index = black_box(rng.random_range(0..size));
                        let _ = get_proof(&client, &base_url, index).await;
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = custom_criterion();
    targets =
        bench_add_leaf,
        bench_add_leaves_batch,
        bench_get_root,
        bench_get_proof
);
criterion_main!(benches);
