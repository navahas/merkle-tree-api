use serde::Serialize;
use sha3::{Digest, Keccak256};

// dummy limit to avoid infinite growth (mem leak)
const MAX_LEAVES: usize = 1 << 11; // 2048

#[derive(Debug, Clone, Serialize)]
pub struct MerkleProof {
    pub siblings: Vec<String>,
    pub directions: Vec<bool>, // true = right, false = left
}

#[derive(Debug)]
pub struct IncrementalMerkleTree {
    leaves: Vec<Vec<u8>>,
    // cache: level -> index -> hash
    cached_hashes: Vec<Vec<Vec<u8>>>,
    cached_root: Option<Vec<u8>>,
    cache_valid: bool,
}

impl IncrementalMerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            cached_hashes: Vec::new(),
            cached_root: None,
            cache_valid: true,
        }
    }

    pub fn add_leaf(&mut self, leaf: Vec<u8>) -> Result<(), &'static str> {
        if self.leaves.len() >= MAX_LEAVES {
            return Err("Exceeded max number of leaves in merkle tree");
        }
        self.leaves.push(leaf);
        self.invalidate_cache();
        Ok(())
    }

    pub fn add_leaves(&mut self, mut leaves: Vec<Vec<u8>>) -> Result<(), &'static str> {
        if self.leaves.len() + leaves.len() > MAX_LEAVES {
            return Err("Exceeded max number of leaves in merkle tree");
        }
        self.leaves.append(&mut leaves);
        self.invalidate_cache();
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

    pub fn get_proof(&mut self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        if !self.cache_valid {
            self.compute_tree();
        }

        let mut siblings = Vec::new();
        let mut directions = Vec::new();
        let mut current_index = index;
        let mut current_level = 0;

        loop {
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
                    directions.push(current_index % 2 == 0);
                }
            } else {
                // No sibling, use self
                if let Some(self_hash) = level_hashes.get(current_index) {
                    siblings.push(hex::encode(self_hash));
                    directions.push(false);
                }
            }

            current_index /= 2;
            current_level += 1;
        }

        Some(MerkleProof {
            siblings,
            directions,
        })
    }

    pub fn _verify_proof(&self, leaf: &[u8], proof: &MerkleProof, root: &[u8]) -> bool {
        if proof.siblings.len() != proof.directions.len() {
            return false;
        }

        let mut current_hash = leaf.to_vec();

        for (sibling_hex, is_right) in proof.siblings.iter().zip(proof.directions.iter()) {
            let sibling = match hex::decode(sibling_hex) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            current_hash = if *is_right {
                // Current node is left, sibling is right
                self.hash_pair(&current_hash, &sibling)
            } else {
                // Current node is right, sibling is left
                self.hash_pair(&sibling, &current_hash)
            };
        }

        current_hash == root
    }

    fn compute_tree(&mut self) {
        if self.leaves.is_empty() {
            return;
        }
        
        let max_depth = 64;

        // Level 0: The leaves are the first level of hashes.
        // We clone them to store them in the cache.
        self.cached_hashes.clear();
        self.cached_hashes.push(self.leaves.clone());

        let mut current_level = 0;
        let mut level_size = self.leaves.len();

        while level_size > 1 {
            if current_level >= max_depth {
                println!("Exceeded max number of leaves in merkle tree");
                return
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

    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }

    fn invalidate_cache(&mut self) {
        self.cache_valid = false;
        self.cached_root = None;
        self.cached_hashes.clear();
    }
}

impl Default for IncrementalMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn test_empty_tree() {
        let mut tree = IncrementalMerkleTree::new();
        assert_eq!(tree.num_leaves(), 0);
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_single_leaf() {
        let mut tree = IncrementalMerkleTree::new();
        assert!(tree.add_leaf(hex("leaf1")).is_ok());
        assert_eq!(tree.num_leaves(), 1);
        assert!(tree.root().is_some());
    }

    #[test]
    fn test_multiple_leaves() {
        let mut tree = IncrementalMerkleTree::new();
        assert!(tree.add_leaf(hex("leaf1")).is_ok());
        assert!(tree.add_leaf(hex("leaf2")).is_ok());
        assert!(tree.add_leaf(hex("leaf3")).is_ok());

        assert_eq!(tree.num_leaves(), 3);
        assert!(tree.root().is_some());
    }

    #[test]
    fn test_add_leaves_batch() {
        let mut tree = IncrementalMerkleTree::new();
        let leaves = vec![hex("leaf1"), hex("leaf2"), hex("leaf3")];
        assert!(tree.add_leaves(leaves).is_ok());
        assert_eq!(tree.num_leaves(), 3);
    }

    #[test]
    fn test_proof_generation_valid_and_invalid_index() {
        let mut tree = IncrementalMerkleTree::new();
        let _ = tree.add_leaves(vec![
            hex("a"), hex("b"), hex("c"), hex("d")
        ]);

        let valid_proof = tree.get_proof(0);
        assert!(valid_proof.is_some());

        let invalid_proof = tree.get_proof(999);
        assert!(invalid_proof.is_none());
    }

    #[test]
    fn test_leaf_limit_enforced() {
        let mut tree = IncrementalMerkleTree::new();
        let leaf = vec![0u8; 32];

        for _ in 0..MAX_LEAVES {
            assert!(tree.add_leaf(leaf.clone()).is_ok());
        }

        let result = tree.add_leaf(leaf.clone());
        assert!(result.is_err());
        assert_eq!(tree.num_leaves(), MAX_LEAVES);
    }

    #[test]
    fn test_root_consistency() {
        let mut tree = IncrementalMerkleTree::new();
        let _ = tree.add_leaves(vec![
            hex("x"), hex("y"), hex("z")
        ]);

        let root1 = tree.root();
        let root2 = tree.root();

        assert_eq!(root1, root2);
    }

    #[test]
    fn test_root_changes_after_addition() {
        let mut tree = IncrementalMerkleTree::new();
        let _ = tree.add_leaf(hex("1"));
        let root1 = tree.root();

        let _ = tree.add_leaf(hex("2"));
        let root2 = tree.root();

        assert_ne!(root1, root2);
    }

    #[test]
    fn test_proof_structure() {
        let mut tree = IncrementalMerkleTree::new();
        let _ = tree.add_leaves(vec![
            hex("a"), hex("b"), hex("c"), hex("d")
        ]);

        let proof = tree.get_proof(2);
        assert!(proof.is_some());
        let proof = proof.unwrap();

        assert_eq!(proof.siblings.len(), proof.directions.len());
        assert!(proof.siblings.len() > 0);
    }

    #[test]
    fn test_proof_verification_valid() {
        let mut tree = IncrementalMerkleTree::new();
        let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
        let _ = tree.add_leaves(leaves.clone());

        let root = tree.root().unwrap();
        let proof = tree.get_proof(1).unwrap();

        assert!(tree._verify_proof(&leaves[1], &proof, &root));
    }

    #[test]
    fn test_proof_verification_invalid_leaf() {
        let mut tree = IncrementalMerkleTree::new();
        let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
        let _ = tree.add_leaves(leaves);

        let root = tree.root().unwrap();
        let proof = tree.get_proof(1).unwrap();

        // Wrong leaf data
        assert!(!tree._verify_proof(&hex("wrong"), &proof, &root));
    }

    #[test]
    fn test_proof_verification_invalid_root() {
        let mut tree = IncrementalMerkleTree::new();
        let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
        let _ = tree.add_leaves(leaves.clone());

        let proof = tree.get_proof(1).unwrap();
        let wrong_root = hex("wrong_root");

        assert!(!tree._verify_proof(&leaves[1], &proof, &wrong_root));
    }

    #[test]
    fn test_proof_verification_malformed_proof() {
        let tree = IncrementalMerkleTree::new();
        let leaf = hex("test");
        let root = hex("root");

        // Mismatched siblings and directions length
        let malformed_proof = MerkleProof {
            siblings: vec!["abc".to_string()],
            directions: vec![true, false], // Different length
        };

        assert!(!tree._verify_proof(&leaf, &malformed_proof, &root));

        // Invalid hex in siblings
        let invalid_hex_proof = MerkleProof {
            siblings: vec!["invalid_hex_string".to_string()],
            directions: vec![true],
        };

        assert!(!tree._verify_proof(&leaf, &invalid_hex_proof, &root));
    }

    #[test]
    fn test_proof_verification_single_leaf() {
        let mut tree = IncrementalMerkleTree::new();
        let leaf = hex("single");
        let _ = tree.add_leaf(leaf.clone());

        let root = tree.root().unwrap();
        let proof = tree.get_proof(0).unwrap();

        assert!(tree._verify_proof(&leaf, &proof, &root));
    }
}
