# Merkle Tree API in Rust

High-performance Merkle tree server written in Rust using `axum`, featuring dual implementations (in-memory and persistent), thread-safe concurrent operations, and comprehensive ACID-compliant storage via LMDB.

## Features
- Incremental Merkle Tree with SHA-3 Keccak256
- Dual implementation: In-memory (heap) and persistent (LMDB)
- Caching of tree levels for fast root/proof computation
- REST API for adding leaves and querying proofs
- Thread-safe concurrent operations with RwLock
- Comprehensive test coverage including concurrency tests
- Optional benchmarking and load testing via criterion and reqwest
- LMDB-based persistence with ACID transactions

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
Comprehensive test suite covering all components:
```bash
# Run all tests in the project
cargo test 

# Run specific test modules
cargo test --test merkle_tree
cargo test --test lmdb_tree
cargo test --test concurrency
cargo test --test storage
```

### Run
Server starts on port `8080` by default with dual tree implementations:
```bash
# Run with default settings
cargo run

# Configure port and LMDB storage path
export PORT=3000
export STORAGE_PATH=./custom_merkle.db
cargo run
```

## API Endpoints
This API exposes dual implementations via different route prefixes:

### Heap-based Routes (In-Memory)
| Method | Route            | Description                         |
|--------|------------------|-------------------------------------|
| POST   | `/add-leaf`      | Adds a single leaf (hex string) to the tree |
| POST   | `/add-leaves`    | Adds multiple leaves in one request |
| GET    | `/get-num-leaves`| Returns the current number of leaves |
| GET    | `/get-root`      | Returns the Merkle root (hex encoded) |
| POST   | `/get-proof`     | Returns a Merkle proof for the given leaf index |

### LMDB-based Routes (Persistent)
| Method | Route                  | Description                         |
|--------|------------------------|-------------------------------------|
| POST   | `/lmdb/add-leaf`       | Adds a single leaf with persistence |
| POST   | `/lmdb/add-leaves`     | Adds multiple leaves with persistence |
| GET    | `/lmdb/get-num-leaves` | Returns leaves count from database |
| GET    | `/lmdb/get-root`       | Returns root hash from database |
| POST   | `/lmdb/get-proof`      | Returns proof generated from database |


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

# Add to heap-based tree
curl -X POST $BASE_URL/add-leaf \
  -H "Content-Type: application/json" \
  -d '{"leaf": "6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50"}'

# Add to persistent LMDB tree
curl -X POST $BASE_URL/lmdb/add-leaf \
  -H "Content-Type: application/json" \
  -d '{"leaf": "6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50"}'
```

- Add Multiple Leaves: `/add-leaves`

```bash
echo -n "new data" | openssl dgst -sha256
echo -n "more data" | openssl dgst -sha256

# Add to heap-based tree
curl -X POST $BASE_URL/add-leaves \
  -H "Content-Type: application/json" \
  -d '{"leaves": ["737165b08ad9b72940af2167aae90fb7eb3b52faf641c0590d36f857adbe451d", "d5b7f828235a92d3d280fa08f3ddb9e5b6947123b44091c92db7594aa1408614"]}'

# Add to persistent LMDB tree
curl -X POST $BASE_URL/lmdb/add-leaves \
  -H "Content-Type: application/json" \
  -d '{"leaves": ["737165b08ad9b72940af2167aae90fb7eb3b52faf641c0590d36f857adbe451d", "d5b7f828235a92d3d280fa08f3ddb9e5b6947123b44091c92db7594aa1408614"]}'
```
- Get Number of Leaves:

```bash
# From heap-based tree
curl $BASE_URL/get-num-leaves

# From persistent LMDB tree
curl $BASE_URL/lmdb/get-num-leaves
```

- Get Merkle Root:

```bash
# From heap-based tree
curl $BASE_URL/get-root

# From persistent LMDB tree
curl $BASE_URL/lmdb/get-root
```

- Get Proof for a Leaf:

```bash
# From heap-based tree
curl -X POST $BASE_URL/get-proof \
  -H "Content-Type: application/json" \
  -d '{"index": 0}'

# From persistent LMDB tree
curl -X POST $BASE_URL/lmdb/get-proof \
  -H "Content-Type: application/json" \
  -d '{"index": 0}'
```

## Project Structure

```bash
.
├── benches/                      # Criterion benchmarks (HTTP client tests)
│   ├── api_benchmark.rs          # Async benchmark tests using reqwest + Criterion
│   └── plot_benchmark.rs         # Feature analysis benchmarks
├── tests/                        # Integration and unit tests
│   ├── test_concurrency.rs       # Async concurrency tests
│   ├── test_unit.rs              # Core unit tests
│   ├── test_storage.rs           # LMDB storage tests
│   └── test_lmdb_tree.rs         # LMDB tree implementation tests
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── k6-load-test.js               # K6 load testing script
├── docs/
│   └── AI_PROMPTS.md             # Some notes about AI-assisted development
├── README.md
└── src/
    ├── main.rs                   # Axum API server with dual implementations
    ├── lib.rs                    # Library exports
    ├── merkle_tree.rs            # In-memory Merkle tree implementation
    ├── lmdb_tree.rs              # Persistent LMDB Merkle tree
    └── storage.rs                # LMDB storage abstraction
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

# Load testing with k6
k6 run k6-load-test.js
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

Further benchmark information is availabe at [./docs/BENCH.md](./docs/BENCH.md)
