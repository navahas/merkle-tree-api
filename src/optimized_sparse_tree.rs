use crate::storage::LmdbStorage;
use serde::Serialize;
use sha3::{Digest, Keccak256};

const TREE_DEPTH: usize = 32;
const MAX_LEAVES: u64 = 1u64 << TREE_DEPTH;

#[derive(Debug, Clone, Serialize)]
pub struct OptimizedSparseMerkleProof {
    pub siblings: Vec<String>,
}

#[derive(Debug)]
pub struct OptimizedSparseMerkleTree {
    storage: LmdbStorage,
    empty_hash: Vec<u8>,
    depth: usize,
}

impl OptimizedSparseMerkleTree {
    pub fn new(storage_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = LmdbStorage::new(storage_path)?;
        let empty_hash = Self::compute_empty_hash();
        
        Ok(Self {
            storage,
            empty_hash,
            depth: TREE_DEPTH,
        })
    }

    pub fn add_leaf(&mut self, index: u64, value: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        if index >= MAX_LEAVES {
            return Err("Index exceeds maximum tree capacity".into());
        }

        let leaf_hash = self.hash_leaf(&value);
        let mut path_updates = Vec::with_capacity(self.depth + 1);
        
        path_updates.push((0, index, leaf_hash.clone()));

        let mut current_hash = leaf_hash;
        let mut current_index = index;

        for level in 0..self.depth {
            let sibling_index = current_index ^ 1;
            let sibling_hash = self.storage.get_node(level, sibling_index)?
                .unwrap_or_else(|| self.empty_hash.clone());

            let parent_hash = if current_index & 1 == 0 {
                self.hash_pair(&current_hash, &sibling_hash)
            } else {
                self.hash_pair(&sibling_hash, &current_hash)
            };

            current_index >>= 1;
            let parent_level = level + 1;

            path_updates.push((parent_level, current_index, parent_hash.clone()));
            current_hash = parent_hash;
        }

        self.storage.store_path_batch(&path_updates)?;
        
        Ok(())
    }

    pub fn get_root(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.storage.get_node(self.depth, 0)
            .map(|opt| opt.unwrap_or_else(|| self.empty_hash.clone()))
    }

    pub fn get_proof(&self, index: u64) -> Result<OptimizedSparseMerkleProof, Box<dyn std::error::Error>> {
        if index >= MAX_LEAVES {
            return Err("Index exceeds maximum tree capacity".into());
        }

        let mut siblings = Vec::new();
        let mut current_index = index;

        for level in 0..self.depth {
            let sibling_index = current_index ^ 1;
            let sibling_hash = self.storage.get_node(level, sibling_index)?
                .unwrap_or_else(|| self.empty_hash.clone());
            
            siblings.push(hex::encode(sibling_hash));
            current_index >>= 1;
        }

        Ok(OptimizedSparseMerkleProof { siblings })
    }

    pub fn verify_proof(
        &self,
        leaf_value: &[u8],
        proof: &OptimizedSparseMerkleProof,
        root: &[u8],
        index: u64,
    ) -> bool {
        if index >= MAX_LEAVES || proof.siblings.len() != self.depth {
            return false;
        }

        let mut current_hash = self.hash_leaf(leaf_value);
        let mut current_index = index;

        for sibling_hex in &proof.siblings {
            let sibling = match hex::decode(sibling_hex) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            current_hash = if current_index & 1 == 0 {
                self.hash_pair(&current_hash, &sibling)
            } else {
                self.hash_pair(&sibling, &current_hash)
            };

            current_index >>= 1;
        }

        current_hash == root
    }

    fn compute_empty_hash() -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(&[0u8]);
        hasher.finalize().to_vec()
    }

    fn hash_leaf(&self, value: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(value);
        hasher.finalize().to_vec()
    }

    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }
}