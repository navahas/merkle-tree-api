import http from 'k6/http';
import { check, sleep } from 'k6';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const IMPLEMENTATION = __ENV.IMPLEMENTATION || 'heap'; // 'heap' or 'lmdb'
const DURATION = __ENV.DURATION || '30s';
const CONCURRENT_USERS = parseInt(__ENV.CONCURRENT_USERS || '10');
const RAMP_UP_TIME = __ENV.RAMP_UP_TIME || '10s';
const RAMP_DOWN_TIME = __ENV.RAMP_DOWN_TIME || '10s';

// Route prefix based on implementation
const ROUTE_PREFIX = IMPLEMENTATION === 'lmdb' ? '/lmdb' : '';

export const options = {
  stages: [
    { duration: RAMP_UP_TIME, target: CONCURRENT_USERS },
    { duration: DURATION, target: CONCURRENT_USERS },
    { duration: RAMP_DOWN_TIME, target: 0 },
  ],
  thresholds: {
    http_req_failed: ['rate<0.01'], // <1% errors
    http_req_duration: ['p(95)<500'], // 95% under 500ms
    checks: ['rate>0.95'], // >95% checks pass
  },
};

export function setup() {
  console.log(`Benchmarking ${IMPLEMENTATION} implementation`);
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Route prefix: ${ROUTE_PREFIX}`);
  console.log(`Duration: ${DURATION}, Users: ${CONCURRENT_USERS}`);
  
  // Pre-populate with some data for consistent benchmarking
  const initialLeaves = generateLeaves(50);
  const response = http.post(`${BASE_URL}${ROUTE_PREFIX}/add-leaves`, 
    JSON.stringify({ leaves: initialLeaves }),
    { headers: { 'Content-Type': 'application/json' } }
  );
  
  if (response.status !== 200) {
    console.warn(`Failed to pre-populate tree: ${response.status}`);
  }
  
  return { initialLeafCount: 50 };
}

export default function (data) {
  const scenario = Math.random();
  
  if (scenario < 0.3) {
    benchAddSingleLeaf();
  } else if (scenario < 0.5) {
    benchAddBatchLeaves();
  } else if (scenario < 0.7) {
    benchGetRoot();
  } else if (scenario < 0.9) {
    benchGetNumLeaves();
  } else {
    benchGetProof();
  }
  
  sleep(0.05); // Small delay between operations
}

function generateLeaf() {
  return Math.random().toString(16).substring(2, 18).padStart(16, '0');
}

function generateLeaves(count) {
  return Array.from({ length: count }, () => generateLeaf());
}

function benchAddSingleLeaf() {
  const leaf = generateLeaf();
  const response = http.post(`${BASE_URL}${ROUTE_PREFIX}/add-leaf`, 
    JSON.stringify({ leaf }),
    { 
      headers: { 'Content-Type': 'application/json' },
      tags: { operation: 'add_single_leaf', implementation: IMPLEMENTATION }
    }
  );
  
  check(response, {
    'add-leaf success': (r) => r.status === 200,
  }, { operation: 'add_single_leaf' });
}

function benchAddBatchLeaves() {
  const batchSize = Math.floor(Math.random() * 10) + 1; // 1-10 leaves
  const leaves = generateLeaves(batchSize);
  const response = http.post(`${BASE_URL}${ROUTE_PREFIX}/add-leaves`, 
    JSON.stringify({ leaves }),
    { 
      headers: { 'Content-Type': 'application/json' },
      tags: { operation: 'add_batch_leaves', implementation: IMPLEMENTATION, batch_size: batchSize }
    }
  );
  
  check(response, {
    'add-leaves success': (r) => r.status === 200,
  }, { operation: 'add_batch_leaves' });
}

function benchGetRoot() {
  const response = http.get(`${BASE_URL}${ROUTE_PREFIX}/get-root`, {
    tags: { operation: 'get_root', implementation: IMPLEMENTATION }
  });
  
  check(response, {
    'get-root success': (r) => [200, 400].includes(r.status),
    'get-root valid response': (r) => {
      if (r.status === 200) {
        const body = JSON.parse(r.body);
        return body.root && typeof body.root === 'string' && body.root.length > 0;
      }
      return true; // 400 acceptable for empty tree
    },
  }, { operation: 'get_root' });
}

function benchGetNumLeaves() {
  const response = http.get(`${BASE_URL}${ROUTE_PREFIX}/get-num-leaves`, {
    tags: { operation: 'get_num_leaves', implementation: IMPLEMENTATION }
  });
  
  check(response, {
    'get-num-leaves success': (r) => r.status === 200,
    'get-num-leaves valid count': (r) => {
      const body = JSON.parse(r.body);
      return typeof body.num_leaves === 'number' && body.num_leaves >= 0;
    },
  }, { operation: 'get_num_leaves' });
}

function benchGetProof() {
  // Get current leaf count first
  const leavesResponse = http.get(`${BASE_URL}${ROUTE_PREFIX}/get-num-leaves`);
  if (leavesResponse.status !== 200) return;
  
  const numLeaves = JSON.parse(leavesResponse.body).num_leaves;
  if (numLeaves === 0) return; // Skip if tree is empty
  
  const index = Math.floor(Math.random() * numLeaves);
  const response = http.post(`${BASE_URL}${ROUTE_PREFIX}/get-proof`, 
    JSON.stringify({ index }),
    { 
      headers: { 'Content-Type': 'application/json' },
      tags: { operation: 'get_proof', implementation: IMPLEMENTATION }
    }
  );
  
  check(response, {
    'get-proof success': (r) => [200, 400].includes(r.status),
    'get-proof valid structure': (r) => {
      if (r.status === 200) {
        const body = JSON.parse(r.body);
        return body.proof && Array.isArray(body.proof.siblings);
      }
      return true; // 400 might occur during mutations
    },
  }, { operation: 'get_proof' });
}

export function teardown(data) {
  console.log(`Benchmark completed for ${IMPLEMENTATION} implementation`);
}