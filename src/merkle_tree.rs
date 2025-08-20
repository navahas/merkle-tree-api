use super::storage::{LmdbStorage, TreeMetadata};
use serde::Serialize;
use sha3::{Digest, Keccak256};

// tree limit
const MAX_LEVELS: usize = 32;
const MAX_LEAVES: usize = 1 << MAX_LEVELS;

#[derive(Debug, Clone, Serialize)]
pub struct MerkleProof {
    pub siblings: Vec<String>,
}

#[derive(Debug)]
pub struct IncrementalMerkleTree {
    leaves: Vec<Vec<u8>>,
    max_leaves: usize,
    // cache: level -> index -> hash
    pub cached_hashes: Vec<Vec<Vec<u8>>>,
    cached_root: Option<Vec<u8>>,
    cache_valid: bool,
    storage: Option<LmdbStorage>,
}

impl IncrementalMerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            max_leaves: MAX_LEAVES,
            cached_hashes: Vec::new(),
            cached_root: None,
            cache_valid: true,
            storage: None,
        }
    }

    pub fn new_with_storage(storage_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = LmdbStorage::new(storage_path)?;
        let mut tree = Self {
            leaves: Vec::new(),
            max_leaves: MAX_LEAVES,
            cached_hashes: Vec::new(),
            cached_root: None,
            cache_valid: true,
            storage: Some(storage),
        };
        tree.load_from_storage()?;
        Ok(tree)
    }

    pub fn _new_with_max(max_leaves: usize) -> Self {
        Self {
            leaves: Vec::new(),
            max_leaves,
            cached_hashes: Vec::new(),
            cached_root: None,
            cache_valid: true,
            storage: None,
        }
    }

    pub fn add_leaf(&mut self, leaf: Vec<u8>) -> Result<(), &'static str> {
        if self.leaves.len() >= self.max_leaves {
            return Err("Exceeded max number of leaves in merkle tree");
        }
        self.leaves.push(leaf.clone());
        self.compute_tree();

        if let Some(ref storage) = self.storage {
            let _ = storage.store_leaf(self.leaves.len() - 1, &leaf);
            self.save_to_storage();
        }

        Ok(())
    }

    pub fn add_leaves(&mut self, mut leaves: Vec<Vec<u8>>) -> Result<(), &'static str> {
        if self.leaves.len() + leaves.len() > self.max_leaves {
            return Err("Exceeded max number of leaves in merkle tree");
        }
        let start_index = self.leaves.len();
        self.leaves.append(&mut leaves);
        self.compute_tree();

        if let Some(ref storage) = self.storage {
            let leaves_to_store = &self.leaves[start_index..];
            let _ = storage.append_leaves(start_index, leaves_to_store);
            self.save_to_storage();
        }

        Ok(())
    }

    pub fn num_leaves(&self) -> usize {
        self.leaves.len()
    }

    pub fn root(&mut self) -> Option<Vec<u8>> {
        if self.leaves.is_empty() {
            return None;
        }

        if self.cache_valid && self.cached_root.is_some() {
            return self.cached_root.clone();
        }

        self.compute_tree();
        self.cached_root.clone()
    }

    pub fn get_proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        if self.cached_hashes.is_empty() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut current_index = index;
        let mut current_level = 0;

        for _ in 0..MAX_LEVELS {
            let level_hashes = self.cached_hashes.get(current_level)?;
            let level_size = if current_level == 0 {
                self.leaves.len()
            } else {
                level_hashes.len()
            };

            if level_size <= 1 {
                break;
            }

            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < level_size {
                if let Some(sibling_hash) = level_hashes.get(sibling_index) {
                    siblings.push(hex::encode(sibling_hash));
                }
            } else {
                // No sibling, use self
                if let Some(self_hash) = level_hashes.get(current_index) {
                    siblings.push(hex::encode(self_hash));
                }
            }

            current_index /= 2;
            current_level += 1;
        }

        Some(MerkleProof { siblings })
    }

    pub fn verify_proof(
        &self,
        leaf: &[u8],
        proof: &MerkleProof,
        root: &[u8],
        leaf_index: usize,
    ) -> bool {
        let mut current_hash = leaf.to_vec();
        let mut current_index = leaf_index;

        for sibling_hex in proof.siblings.iter() {
            let sibling = match hex::decode(sibling_hex) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            current_hash = if current_index % 2 == 0 {
                // Current node is left, sibling is right
                self.hash_pair(&current_hash, &sibling)
            } else {
                // Current node is right, sibling is left
                self.hash_pair(&sibling, &current_hash)
            };

            current_index /= 2;
        }

        current_hash == root
    }

    fn compute_tree(&mut self) {
        if self.leaves.is_empty() {
            self.cached_hashes.clear();
            self.cached_root = None;
            self.cache_valid = true;
            return;
        }

        // Level 0: The leaves are the first level of hashes.
        // We clone them to store them in the cache.
        self.cached_hashes.clear();
        self.cached_hashes.push(self.leaves.clone());

        let mut current_level = 0;
        let mut level_size = self.leaves.len();

        while level_size > 1 {
            if current_level >= MAX_LEVELS {
                println!("Exceeded max number of leaves in merkle tree");
                return;
            }

            let mut next_level_hashes = Vec::new();
            let current_hashes = &self.cached_hashes[current_level];

            for chunk in current_hashes.chunks(2) {
                let left = chunk.get(0).unwrap();
                // Get the right element, or if it doesn't exist (odd number of hashes),
                // use the left element as the right.
                let right = chunk.get(1).unwrap_or(left);

                let hash = self.hash_pair(left, right);
                next_level_hashes.push(hash);
            }

            self.cached_hashes.push(next_level_hashes);
            current_level += 1;
            level_size = self.cached_hashes[current_level].len();
        }

        // Cache root
        if let Some(root_vec) = self.cached_hashes.last() {
            if let Some(root) = root_vec.first() {
                self.cached_root = Some(root.clone());
            }
        }

        self.cache_valid = true;
    }

    fn load_from_storage(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref storage) = self.storage {
            if let Some(metadata) = storage.get_metadata()? {
                self.max_leaves = metadata.max_leaves;
                self.cache_valid = metadata.cache_valid;
            }

            self.leaves = storage.get_all_leaves()?;
            self.cached_hashes = storage.get_all_cache_levels()?;
            self.cached_root = storage.get_root()?;
        }
        Ok(())
    }

    fn save_to_storage(&self) {
        if let Some(ref storage) = self.storage {
            let metadata = TreeMetadata {
                num_leaves: self.leaves.len(),
                max_leaves: self.max_leaves,
                cache_valid: self.cache_valid,
            };

            let _ = storage.store_metadata(&metadata);
            let _ = storage.store_cache_batch(&self.cached_hashes);

            if let Some(ref root) = self.cached_root {
                let _ = storage.store_root(root);
            }

            let _ = storage.sync();
        }
    }

    pub fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref storage) = self.storage {
            let metadata = TreeMetadata {
                num_leaves: self.leaves.len(),
                max_leaves: self.max_leaves,
                cache_valid: self.cache_valid,
            };

            storage.store_leaves_batch(&self.leaves)?;
            storage.store_metadata(&metadata)?;
            storage.store_cache_batch(&self.cached_hashes)?;

            if let Some(ref root) = self.cached_root {
                storage.store_root(root)?;
            }

            storage.sync()?;
        }
        Ok(())
    }

    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }
}

impl Default for IncrementalMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}
