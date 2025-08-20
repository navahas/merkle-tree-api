import http from 'k6/http';
import { check, sleep } from 'k6';

const BASE_URL = 'http://localhost:8080';

export const options = {
  stages: [
    { duration: '10s', target: 5 },   // Ramp up
    { duration: '30s', target: 10 },  // Stay at 10 users
    { duration: '10s', target: 0 },   // Ramp down
  ],
};

// Generate test data
function generateLeaf() {
  return Math.random().toString(16).substring(2, 18).padStart(16, '0');
}

function generateLeaves(count) {
  return Array.from({ length: count }, () => generateLeaf());
}

export default function () {
  // Mix of operations to stress test concurrency
  const operations = [
    () => addSingleLeaf(),
    () => addBatchLeaves(),
    () => getRoot(),
    () => getNumLeaves(),
    () => getProof(),
  ];

  // Randomly select operation
  const operation = operations[Math.floor(Math.random() * operations.length)];
  operation();
  
  sleep(0.1); // Small delay between requests
}

function addSingleLeaf() {
  const leaf = generateLeaf();
  const response = http.post(`${BASE_URL}/add-leaf`, 
    JSON.stringify({ leaf }),
    { headers: { 'Content-Type': 'application/json' } }
  );
  
  check(response, {
    'add-leaf status is 200': (r) => r.status === 200,
  });
}

function addBatchLeaves() {
  const leaves = generateLeaves(Math.floor(Math.random() * 5) + 1);
  const response = http.post(`${BASE_URL}/add-leaves`, 
    JSON.stringify({ leaves }),
    { headers: { 'Content-Type': 'application/json' } }
  );
  
  check(response, {
    'add-leaves status is 200': (r) => r.status === 200,
  });
}

function getRoot() {
  const response = http.get(`${BASE_URL}/get-root`);
  
  check(response, {
    'get-root status is 200 or 400': (r) => [200, 400].includes(r.status),
    'get-root has valid response': (r) => {
      if (r.status === 200) {
        const body = JSON.parse(r.body);
        return body.root && typeof body.root === 'string';
      }
      return true; // 400 is valid for empty tree
    },
  });
}

function getNumLeaves() {
  const response = http.get(`${BASE_URL}/get-num-leaves`);
  
  check(response, {
    'get-num-leaves status is 200': (r) => r.status === 200,
    'get-num-leaves has valid count': (r) => {
      const body = JSON.parse(r.body);
      return typeof body.num_leaves === 'number' && body.num_leaves >= 0;
    },
  });
}

function getProof() {
  // First get number of leaves to determine valid index
  const leavesResponse = http.get(`${BASE_URL}/get-num-leaves`);
  if (leavesResponse.status !== 200) return;
  
  const numLeaves = JSON.parse(leavesResponse.body).num_leaves;
  if (numLeaves === 0) return; // Skip if tree is empty
  
  const index = Math.floor(Math.random() * numLeaves);
  const response = http.post(`${BASE_URL}/get-proof`, 
    JSON.stringify({ index }),
    { headers: { 'Content-Type': 'application/json' } }
  );
  
  check(response, {
    'get-proof status is 200 or 400': (r) => [200, 400].includes(r.status),
    'get-proof has valid structure': (r) => {
      if (r.status === 200) {
        const body = JSON.parse(r.body);
        return body.proof && Array.isArray(body.proof.siblings);
      }
      return true; // 400 might occur during concurrent mutations
    },
  });
}
