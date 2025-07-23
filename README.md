# Merkle Tree API in Rust

Merkle tree server written in Rust using `axum`, supporting incremental
updates, proof generation, and verification.

## Features
- Incremental Merkle Tree with SHA-3 Keccak256
- Caching of tree levels for fast root/proof computation
- REST API for adding leaves and querying proofs
- Simple and readable implementation
- Optional benchmarking and load testing via criterion and reqwest

### AI Assistance

Insights on the AI-assisted implementation are available at [./docs/AI_PROMPTS.md](./docs/AI_PROMPTS.md).

### Deployment

The service is deployed on a VPS inside a Docker container, as part of a Docker
network managed by **Traefik** as a reverse proxy. This setup handles routing
and HTTPS termination automatically. The API is publicly accessible at:

- [https://merkle-api.codecrypto.academy](https://merkle-api.codecrypto.academy)

> [!NOTE]
> Previous local benchmark results are served at the `/benchmarks` path. See [`criterion-docs` branch](https://github.com/navahas/merkle-tree-api/tree/criterion-docs) for details.

## Getting Started

### Tests
To run all unit tests for the Merkle tree logic and proof verification.
```bash
cargo test
```

### Run
Server starts on port `8080` by default. (override with PORT env var)
```bash
# export PORT=
cargo run
```

## API Endpoints
This API exposes the following routes to interact with the Merkle tree:

| Method | Route            | Description                         |
|--------|------------------|-------------------------------------|
| POST   | `/add-leaf`      | Adds a single leaf (hex string) to the tree |
| POST   | `/add-leaves`    | Adds multiple leaves in one request |
| GET    | `/get-num-leaves`| Returns the current number of leaves |
| GET    | `/get-root`      | Returns the Merkle root (hex encoded) |
| POST   | `/get-proof`     | Returns a Merkle proof for the given leaf index |

You can test the API directly in the deployed resource, without running it
locally. Set the BASE_URL environment variable accordingly:
```bash
# To use the live deployment
export BASE_URL=https://merkle-api.codecrypto.academy

# To use a local server
export BASE_URL=http://localhost:8080
```

- Add Leaf: `/add-leaf`

```bash
echo -n "some data to hash" | openssl dgst -sha256
# Output: (stdin)= 6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50

curl -X POST $BASE_URL/add-leaf \
  -H "Content-Type: application/json" \
  -d '{"leaf": "6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50"}'
```

- Add Multiple Leaves: `/add-leaves`

```bash
echo -n "new data" | openssl dgst -sha256
echo -n "more data" | openssl dgst -sha256

curl -X POST $BASE_URL/add-leaves \
  -H "Content-Type: application/json" \
  -d '{"leaves": ["737165b08ad9b72940af2167aae90fb7eb3b52faf641c0590d36f857adbe451d", "d5b7f828235a92d3d280fa08f3ddb9e5b6947123b44091c92db7594aa1408614"]}'
```
- Get Number of Leaves: `/get-num-leaves`

```bash
curl $BASE_URL/get-num-leaves
```

- Get Merkle Root: `/get-root`

```bash
curl $BASE_URL/get-root
```

- Get Proof for a Leaf: `/get-proof`

```bash
curl -X POST $BASE_URL/get-proof \
  -H "Content-Type: application/json" \
  -d '{"index": 0}'
```

## Project Structure

```bash
.
├── benches/                      # Criterion benchmarks (HTTP client tests)
│   └── api_benchmark.rs          # Async benchmark tests using reqwest + Criterion
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── docs/
│   └── AI_PROMPTS.md             # Some notes about AI-assisted development
├── README.md
└── src/
    ├── main.rs                   # Axum API server
    └── merkle_tree.rs            # Merkle tree implementation
```

## Benchmarking

This project includes two Criterion-based benchmark suites:

- `benches/plot_benchmark.rs`:
Measures typical Merkle tree operations for feature-oriented analysis (e.g., adding leaves, getting root/proofs).

- `benches/api_benchmark.rs`:
Focuses on how performance scales with batch size and proof tree depth, enabling comparative plots via BenchmarkId.

Run with:
```bash
cargo bench --bench api_benchmark
cargo bench --bench plot_benchmark
```

> [!IMPORTANT]  
> get_root can be the most expensive operation when the Merkle tree has many
> leaves and the root must be recalculated from scratch. However, due to
> caching, it’s usually fast unless new leaves were recently added.

- get_root is only expensive when recalculation is triggered.
- After adding leaves, the root is recomputed with linear complexity (O(n)), then cached for constant-time access (O(1)).
- By contrast, get_proof always runs with logarithmic complexity (O(log n)), regardless of the tree size or updates.

> [!NOTE]
> O() (Big O notation) describes how performance scales with input size:
> 
> - O(1) → constant time (fast and does not depend on input size)
> - O(n) → time grows linearly with the number of leaves
> - O(log n) → time grows slowly as the number of leaves increases (like doubling the tree size only adds one extra step)


Future benchmarks should explore how get_root scales at various tree sizes
(e.g., 10, 100, 1000 leaves) to capture worst-case recomputation costs.

### Examples results
- api_benchmark:
```bash
API: POST /add-leaf/add_leaf_single
          time:   [331.60 ps 333.31 ps 335.67 ps]

Benchmarking API: POST /add-leaves/add_leaves_batch/10: Collecting 10 samples in estimated 2.0000 s (3.5B iterations
API: POST /add-leaves/add_leaves_batch/10
          time:   [554.56 ps 555.54 ps 556.64 ps]
                        thrpt:  [17.965 Gelem/s 18.000 Gelem/s 18.032 Gelem/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high severe
Benchmarking API: POST /add-leaves/add_leaves_batch/50: Collecting 10 samples in estimated 2.0000 s (3.6B iterations
API: POST /add-leaves/add_leaves_batch/50
          time:   [557.13 ps 558.21 ps 559.76 ps]
                        thrpt:  [89.324 Gelem/s 89.572 Gelem/s 89.746 Gelem/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high severe
Benchmarking API: POST /add-leaves/add_leaves_batch/100: Collecting 10 samples in estimated 2.0000 s (3.6B iteration
API: POST /add-leaves/add_leaves_batch/100
          time:   [556.11 ps 557.98 ps 559.72 ps]
          thrpt:  [178.66 Gelem/s 179.22 Gelem/s 179.82 Gelem/s]
Benchmarking API: POST /add-leaves/add_leaves_batch/200: Collecting 10 samples in estimated 2.0000 s (3.5B iteration
API: POST /add-leaves/add_leaves_batch/200
          time:   [557.68 ps 569.87 ps 589.44 ps]
          thrpt:  [339.31 Gelem/s 350.96 Gelem/s 358.63 Gelem/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high severe

API: GET /get-root/get_root_fixed_tree
          time:   [66.662 µs 67.615 µs 68.201 µs]

API: POST /get-proof/get_proof_at/10
          time:   [107.03 µs 108.67 µs 111.10 µs]
          thrpt:  [9.0009 Kelem/s 9.2024 Kelem/s 9.3431 Kelem/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) low mild
API: POST /get-proof/get_proof_at/50
          time:   [105.27 µs 105.41 µs 105.59 µs]
          thrpt:  [9.4705 Kelem/s 9.4865 Kelem/s 9.4992 Kelem/s]
API: POST /get-proof/get_proof_at/100
          time:   [105.51 µs 107.58 µs 109.02 µs]
          thrpt:  [9.1725 Kelem/s 9.2951 Kelem/s 9.4776 Kelem/s]
Found 2 outliers among 10 measurements (20.00%)
  2 (20.00%) high mild
API: POST /get-proof/get_proof_at/200
          time:   [99.501 µs 99.917 µs 101.11 µs]
          thrpt:  [9.8899 Kelem/s 10.008 Kelem/s 10.050 Kelem/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
```

- plot_benchmark
```bash
Merkle Tree Operations/add single leaf
          time:   [327.80 ps 329.13 ps 330.36 ps]
          change: [−5.3692% −2.4743% −0.7964%] (p = 0.04 < 0.05)
          Change within noise threshold.
Found 10 outliers among 100 measurements (10.00%)
  8 (8.00%) low mild
  1 (1.00%) high mild
  1 (1.00%) high severe
Benchmarking Merkle Tree Operations/add batch (10 leaves): Collecting 100 samples in estimated 5.0000 s (16B iterati
Merkle Tree Operations/add batch (10 leaves)
          time:   [319.50 ps 321.84 ps 325.49 ps]
          change: [+0.8070% +1.2988% +1.8849%] (p = 0.00 < 0.05)
          Change within noise threshold.
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe
Benchmarking Merkle Tree Operations/add batch (50 leaves): Collecting 100 samples in estimated 5.0000 s (16B iterati
Merkle Tree Operations/add batch (50 leaves)
          time:   [317.88 ps 318.10 ps 318.35 ps]
          change: [−0.0339% +0.1742% +0.3828%] (p = 0.13 > 0.05)
          No change in performance detected.
Found 13 outliers among 100 measurements (13.00%)
  1 (1.00%) low mild
  7 (7.00%) high mild
  5 (5.00%) high severe
Merkle Tree Operations/get root
          time:   [457.85 ps 463.15 ps 470.56 ps]
          change: [−4.8428% −1.8442% +0.1550%] (p = 0.19 > 0.05)
          No change in performance detected.
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) high mild
  4 (4.00%) high severe
Merkle Tree Operations/get proof
          time:   [378.63 ps 413.30 ps 457.41 ps]
          change: [+8.7988% +21.393% +33.710%] (p = 0.00 < 0.05)
          Performance has regressed.
Found 13 outliers among 100 measurements (13.00%)
  5 (5.00%) high mild
  8 (8.00%) high severe

Batch Size Comparison/batch size 1
          time:   [573.11 ps 588.37 ps 611.27 ps]
          change: [+3.8496% +13.059% +25.976%] (p = 0.01 < 0.05)
          Performance has regressed.
Found 12 outliers among 100 measurements (12.00%)
  2 (2.00%) high mild
  10 (10.00%) high severe
Batch Size Comparison/batch size 5
          time:   [575.03 ps 593.02 ps 622.94 ps]
          change: [+4.4389% +6.3807% +8.9107%] (p = 0.00 < 0.05)
          Performance has regressed.
Found 10 outliers among 100 measurements (10.00%)
  8 (8.00%) high mild
  2 (2.00%) high severe
Batch Size Comparison/batch size 10
          time:   [580.79 ps 603.27 ps 641.24 ps]
          change: [+5.3099% +7.4942% +10.463%] (p = 0.00 < 0.05)
          Performance has regressed.
Found 12 outliers among 100 measurements (12.00%)
  4 (4.00%) high mild
  8 (8.00%) high severe
Batch Size Comparison/batch size 25
          time:   [576.14 ps 583.97 ps 593.51 ps]
          change: [+3.7989% +5.1552% +6.6336%] (p = 0.00 < 0.05)
          Performance has regressed.
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) high mild
  6 (6.00%) high severe
Batch Size Comparison/batch size 50
          time:   [560.41 ps 575.19 ps 605.18 ps]
          change: [−1.4257% −0.0078% +2.4013%] (p = 0.99 > 0.05)
          No change in performance detected.
Found 8 outliers among 100 measurements (8.00%)
  7 (7.00%) high mild
  1 (1.00%) high severe
Batch Size Comparison/batch size 100
          time:   [560.50 ps 562.68 ps 565.46 ps]
          change: [+2.0514% +2.8074% +3.6005%] (p = 0.00 < 0.05)
          Performance has regressed.
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high severe

Single vs Batch/10 single operations
          time:   [333.76 ps 345.25 ps 367.56 ps]
          change: [−3.0286% +0.7471% +3.9933%] (p = 0.75 > 0.05)
          No change in performance detected.
Found 4 outliers among 100 measurements (4.00%)
  3 (3.00%) high mild
  1 (1.00%) high severe
Single vs Batch/1 batch of 10
          time:   [320.25 ps 322.09 ps 324.67 ps]
          change: [−0.5887% +1.1933% +3.4277%] (p = 0.27 > 0.05)
          No change in performance detected.
Found 17 outliers among 100 measurements (17.00%)
  1 (1.00%) high mild
  16 (16.00%) high severe
```
