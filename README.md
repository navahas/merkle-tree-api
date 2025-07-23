# Incremental Merkle Tree API (Axum + Rust)

Merkle tree server written in Rust using `axum`, supporting incremental
updates, proof generation, and verification.

## Features
- Incremental Merkle Tree with SHA-3 Keccak256
- Caching of tree levels for fast root/proof computation
- REST API for adding leaves and querying proofs
- Simple and readable implementation
- Optional benchmarking and load testing via criterion and reqwest

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

- Add Leaf: `/add-leaf`

```bash
echo -n "some data to hash" | openssl dgst -sha256
# Output: (stdin)= 6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50

curl -X POST http://localhost:8080/add-leaf \
  -H "Content-Type: application/json" \
  -d '{"leaf": "6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50"}'
```

- Add Multiple Leaves: `/add-leaves`

```bash
echo -n "new data" | openssl dgst -sha256
echo -n "more data" | openssl dgst -sha256

curl -X POST http://localhost:8080/add-leaves \
  -H "Content-Type: application/json" \
  -d '{"leaves": ["737165b08ad9b72940af2167aae90fb7eb3b52faf641c0590d36f857adbe451d", "d5b7f828235a92d3d280fa08f3ddb9e5b6947123b44091c92db7594aa1408614"]}'
```
- Get Number of Leaves: `/get-num-leaves`

```bash
curl http://localhost:8080/get-num-leaves
```

- Get Merkle Root: `/get-root`

```bash
curl http://localhost:8080/get-root
```

- Get Proof for a Leaf: `/get-proof`

```bash
curl -X POST http://localhost:8080/get-proof \
  -H "Content-Type: application/json" \
  -d '{"index": 0}'
```

## Project Structure

```bash
.
├── benches/                # Benchmarks (async client + Criterion)
├── src/
│   ├── main.rs             # Axum API server
│   └── merkle_tree.rs      # Merkle tree implementation
├── Cargo.toml
├── Dockerfile
├── README.md
```

## Benchmarking

This project includes Criterion-based benchmarks.

Run with:
```bash
cargo bench
```

