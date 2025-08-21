# Benchmark Guide

Performance benchmarking tools and instructions for comparing heap-based and LMDB-based Merkle tree implementations.

## Prerequisites

### Install Benchmarking Tools

**K6 (Load testing):**
```bash
# macOS
brew install k6

# Ubuntu/Debian
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**oha (Rust-based HTTP benchmark):**
```bash
# Install via cargo
cargo install oha

# Or download binary from: https://github.com/hatoo/oha/releases
```

## Server Setup

Start the Merkle tree server:
```bash
# Default configuration (port 8080)
cargo run --release

# Custom configuration
export PORT=3000
export STORAGE_PATH=./benchmark_merkle.db
cargo run --release
```

## K6 Benchmarks

### Configuration Options

The k6 benchmark script supports environment variables for configuration:

| Variable | Default | Description |
|----------|---------|-------------|
| `BASE_URL` | `http://localhost:8080` | Server base URL |
| `IMPLEMENTATION` | `heap` | Implementation type (`heap` or `lmdb`) |
| `DURATION` | `30s` | Test duration |
| `CONCURRENT_USERS` | `10` | Number of concurrent virtual users |
| `RAMP_UP_TIME` | `10s` | Time to ramp up to target users |
| `RAMP_DOWN_TIME` | `10s` | Time to ramp down from target users |

### Basic Benchmarks

**Heap Implementation (In-Memory):**
```bash
# Basic benchmark - 10 concurrent users for 30 seconds
k6 run k6-bench.js

# High load - 50 concurrent users for 60 seconds
CONCURRENT_USERS=50 DURATION=60s k6 run k6-bench.js

# Extended test - 100 users for 5 minutes
CONCURRENT_USERS=100 DURATION=5m RAMP_UP_TIME=30s RAMP_DOWN_TIME=30s k6 run k6-bench.js
```

**LMDB Implementation (Persistent):**
```bash
# Basic benchmark
IMPLEMENTATION=lmdb k6 run k6-bench.js

# High load test
IMPLEMENTATION=lmdb CONCURRENT_USERS=50 DURATION=60s k6 run k6-bench.js

# Extended test
IMPLEMENTATION=lmdb CONCURRENT_USERS=100 DURATION=5m RAMP_UP_TIME=30s RAMP_DOWN_TIME=30s k6 run k6-bench.js
```

### Comparative Benchmarking

Run both implementations with identical configurations:

```bash
# Heap benchmark
echo "=== Heap Implementation ===" > benchmark-results.txt
CONCURRENT_USERS=20 DURATION=60s k6 run k6-bench.js >> benchmark-results.txt

# LMDB benchmark  
echo "=== LMDB Implementation ===" >> benchmark-results.txt
IMPLEMENTATION=lmdb CONCURRENT_USERS=20 DURATION=60s k6 run k6-bench.js >> benchmark-results.txt
```

### Remote Server Benchmarking

Test against deployed server:
```bash
# Against live deployment
BASE_URL=https://merkle-api.codecrypto.academy k6 run k6-bench.js

# LMDB implementation on remote server
BASE_URL=https://merkle-api.codecrypto.academy IMPLEMENTATION=lmdb k6 run k6-bench.js
```

## oha Benchmarks

### Single Endpoint Benchmarks

**Get Root Performance:**
```bash
# Heap implementation
oha -z 30s -c 50 --http-version 2 http://localhost:8080/get-root > oha-heap-root.txt

# LMDB implementation  
oha -z 30s -c 50 --http-version 2 http://localhost:8080/lmdb/get-root > oha-lmdb-root.txt

# Compare results
echo "=== Heap vs LMDB Root Performance ===" > oha-comparison.txt
echo "Heap:" >> oha-comparison.txt
cat oha-heap-root.txt >> oha-comparison.txt
echo -e "\n\nLMDB:" >> oha-comparison.txt
cat oha-lmdb-root.txt >> oha-comparison.txt
```

**Get Number of Leaves:**
```bash
# Heap
oha -z 30s -c 100 --http-version 2 http://localhost:8080/get-num-leaves > oha-heap-leaves.txt

# LMDB
oha -z 30s -c 100 --http-version 2 http://localhost:8080/lmdb/get-num-leaves > oha-lmdb-leaves.txt
```

**Add Single Leaf (POST):**
```bash
# Prepare JSON payload
echo '{"leaf": "1234567890abcdef1234567890abcdef12345678"}' > leaf-payload.json

# Heap implementation
oha -z 30s -c 20 -m POST -T 'application/json' -d @leaf-payload.json http://localhost:8080/add-leaf > oha-heap-add-leaf.txt

# LMDB implementation
oha -z 30s -c 20 -m POST -T 'application/json' -d @leaf-payload.json http://localhost:8080/lmdb/add-leaf > oha-lmdb-add-leaf.txt
```

### Batch Operations

**Add Multiple Leaves:**
```bash
# Create batch payload
cat > batch-payload.json << 'EOF'
{
  "leaves": [
    "1111111111111111111111111111111111111111",
    "2222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333",
    "4444444444444444444444444444444444444444",
    "5555555555555555555555555555555555555555"
  ]
}
EOF

# Benchmark batch operations
oha -z 30s -c 10 -m POST -T 'application/json' -d @batch-payload.json http://localhost:8080/add-leaves > oha-heap-batch.txt
oha -z 30s -c 10 -m POST -T 'application/json' -d @batch-payload.json http://localhost:8080/lmdb/add-leaves > oha-lmdb-batch.txt
```

### Remote Benchmarking with oha

```bash
# Remote server benchmarks
oha -z 30s -c 50 --http-version 2 https://merkle-api.codecrypto.academy/get-root > oha-remote-heap.txt
oha -z 30s -c 50 --http-version 2 https://merkle-api.codecrypto.academy/lmdb/get-root > oha-remote-lmdb.txt
```

## Advanced Benchmarking

### Scaling Tests

Test performance under increasing load:

```bash
#!/bin/bash
# scaling-test.sh

for users in 10 25 50 100 200; do
  echo "Testing with $users concurrent users..."
  
  # Heap
  echo "=== Heap - $users users ===" >> scaling-results.txt
  CONCURRENT_USERS=$users DURATION=60s k6 run k6-bench.js >> scaling-results.txt
  
  # LMDB
  echo "=== LMDB - $users users ===" >> scaling-results.txt
  IMPLEMENTATION=lmdb CONCURRENT_USERS=$users DURATION=60s k6 run k6-bench.js >> scaling-results.txt
  
  sleep 10 # Cool down between tests
done
```

### Memory Usage Monitoring

Monitor server resource usage during benchmarks:

```bash
# Monitor memory and CPU during benchmark
htop & 
CONCURRENT_USERS=50 DURATION=120s k6 run k6-bench.js

# Or use system monitoring
iostat -x 1 & 
vmstat 1 &
IMPLEMENTATION=lmdb CONCURRENT_USERS=50 DURATION=120s k6 run k6-bench.js
```

## Benchmark Analysis

### Key Metrics to Compare

1. **Throughput (requests/second)**
   - Higher is better
   - Compare across implementations

2. **Response Time (percentiles)**
   - p50, p95, p99 latencies
   - Lower is better

3. **Error Rate**
   - Should be < 1%
   - Monitor 4xx/5xx responses

4. **Resource Usage**
   - Memory consumption
   - CPU utilization
   - Disk I/O (LMDB only)

### Expected Performance Characteristics

**Heap Implementation:**
- ✅ Faster read operations (get-root, get-proof)
- ✅ Lower latency for small datasets
- ❌ Data lost on restart
- ❌ Memory limited

**LMDB Implementation:**
- ✅ Data persistence across restarts
- ✅ Handles larger datasets efficiently
- ✅ ACID transactions
- ❌ Slightly higher latency due to disk I/O
- ❌ More complex error scenarios

### Sample Results Format

```
Performance Summary:
├── Heap Implementation
│   ├── Throughput: 1,245 req/s
│   ├── p95 Latency: 45ms  
│   ├── Error Rate: 0.12%
│   └── Memory Usage: 125MB
└── LMDB Implementation  
    ├── Throughput: 987 req/s
    ├── p95 Latency: 78ms
    ├── Error Rate: 0.08%
    └── Memory Usage: 89MB
```

## Troubleshooting

### Common Issues

1. **Connection Errors**
   - Ensure server is running
   - Check port configuration
   - Verify firewall settings

2. **High Error Rates**
   - Reduce concurrent users
   - Increase ramp-up time
   - Monitor server resources

3. **Inconsistent Results**
   - Run multiple iterations
   - Ensure clean server state between tests
   - Monitor system background activity

### Server Logs

Monitor server output during benchmarks:
```bash
# Run server with detailed logging
RUST_LOG=debug cargo run --release 2>&1 | tee server-bench.log
```

Clean up LMDB database between tests:
```bash
# Remove LMDB database file
rm -f ./merkle_tree.db
```
