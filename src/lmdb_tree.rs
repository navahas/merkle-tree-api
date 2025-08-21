use crate::merkle_tree::MerkleProof;
use crate::storage::{LmdbStorage, TreeMetadata};
use sha3::{Digest, Keccak256};

const MAX_LEVELS: usize = 32;
const MAX_LEAVES: usize = 1 << MAX_LEVELS;

#[derive(Debug)]
pub struct LmdbMerkleTree {
    storage: LmdbStorage,
    max_leaves: usize,
}

impl LmdbMerkleTree {
    pub fn new(storage_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = LmdbStorage::new(storage_path)?;

        let max_leaves = if let Some(metadata) = storage.get_metadata()? {
            metadata.max_leaves
        } else {
            MAX_LEAVES
        };

        Ok(Self {
            storage,
            max_leaves,
        })
    }

    pub fn add_leaf(&self, leaf: Vec<u8>) -> Result<(), &'static str> {
        let current_count = self.num_leaves();
        if current_count >= self.max_leaves {
            return Err("Exceeded max number of leaves in merkle tree");
        }

        self.storage
            .store_leaf(current_count, &leaf)
            .map_err(|_| "Failed to store leaf")?;

        self.recompute_and_store_tree()
            .map_err(|_| "Failed to recompute tree")?;

        Ok(())
    }

    pub fn add_leaves(&self, leaves: Vec<Vec<u8>>) -> Result<(), &'static str> {
        let current_count = self.num_leaves();
        if current_count + leaves.len() > self.max_leaves {
            return Err("Exceeded max number of leaves in merkle tree");
        }

        self.storage
            .append_leaves(current_count, &leaves)
            .map_err(|_| "Failed to store leaves")?;

        self.recompute_and_store_tree()
            .map_err(|_| "Failed to recompute tree")?;

        Ok(())
    }

    pub fn num_leaves(&self) -> usize {
        if let Ok(Some(metadata)) = self.storage.get_metadata() {
            metadata.num_leaves
        } else {
            self.storage
                .get_all_leaves()
                .map(|leaves| leaves.len())
                .unwrap_or(0)
        }
    }

    pub fn root(&self) -> Option<Vec<u8>> {
        if self.num_leaves() == 0 {
            return None;
        }

        self.storage.get_root().ok().flatten()
    }

    pub fn get_proof(&self, index: usize) -> Option<MerkleProof> {
        let num_leaves = self.num_leaves();
        if index >= num_leaves {
            return None;
        }

        let cache_levels = self.storage.get_all_cache_levels().ok()?;
        if cache_levels.is_empty() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut current_index = index;
        let mut current_level = 0;

        for _ in 0..MAX_LEVELS {
            let level_hashes = cache_levels.get(current_level)?;
            let level_size = if current_level == 0 {
                num_leaves
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
                self.hash_pair(&current_hash, &sibling)
            } else {
                self.hash_pair(&sibling, &current_hash)
            };

            current_index /= 2;
        }

        current_hash == root
    }

    fn recompute_and_store_tree(&self) -> Result<(), Box<dyn std::error::Error>> {
        let leaves = self.storage.get_all_leaves()?;

        if leaves.is_empty() {
            self.storage.clear_cache()?;
            let metadata = TreeMetadata {
                num_leaves: 0,
                max_leaves: self.max_leaves,
            };
            self.storage.store_metadata(&metadata)?;
            return Ok(());
        }

        let mut cache_levels = vec![leaves.clone()];
        let mut current_level = 0;

        while cache_levels[current_level].len() > 1 {
            if current_level >= MAX_LEVELS {
                return Err("Exceeded max levels".into());
            }

            let mut next_level_hashes = Vec::new();
            let current_hashes = &cache_levels[current_level];

            for chunk in current_hashes.chunks(2) {
                let left = chunk.get(0).unwrap();
                let right = chunk.get(1).unwrap_or(left);
                let hash = self.hash_pair(left, right);
                next_level_hashes.push(hash);
            }

            cache_levels.push(next_level_hashes);
            current_level += 1;
        }

        self.storage.store_cache_batch(&cache_levels)?;

        if let Some(root_level) = cache_levels.last() {
            if let Some(root) = root_level.first() {
                self.storage.store_root(root)?;
            }
        }

        let metadata = TreeMetadata {
            num_leaves: leaves.len(),
            max_leaves: self.max_leaves,
        };
        self.storage.store_metadata(&metadata)?;
        self.storage.sync()?;

        Ok(())
    }

    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }
}
