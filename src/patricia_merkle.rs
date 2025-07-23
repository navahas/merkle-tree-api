use serde::Serialize;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

// tree limit
const MAX_LEVELS: usize = 32;
const MAX_KEYS: usize = 1 << MAX_LEVELS;

#[derive(Debug, Clone, Serialize)]
pub struct PatriciaProof {
    pub path: Vec<u8>,
    pub siblings: Vec<String>,
    pub directions: Vec<bool>, // true = right, false = left
}

#[derive(Debug, Clone)]
struct PatriciaNode {
    key_fragment: Vec<u8>,
    value: Option<Vec<u8>>,
    children: HashMap<u8, Box<PatriciaNode>>,
    hash: Option<Vec<u8>>,
}

impl PatriciaNode {
    fn new() -> Self {
        Self {
            key_fragment: Vec::new(),
            value: None,
            children: HashMap::new(),
            hash: None,
        }
    }

    fn new_with_fragment(fragment: Vec<u8>) -> Self {
        Self {
            key_fragment: fragment,
            value: None,
            children: HashMap::new(),
            hash: None,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn invalidate_hash(&mut self) {
        self.hash = None;
    }
}

#[derive(Debug)]
pub struct PatriciaMerkleTree {
    root: PatriciaNode,
    key_count: usize,
    max_keys: usize,
    cache_valid: bool,
}

impl PatriciaMerkleTree {
    pub fn new() -> Self {
        Self {
            root: PatriciaNode::new(),
            key_count: 0,
            max_keys: MAX_KEYS,
            cache_valid: true,
        }
    }

    pub fn _new_with_max(max_keys: usize) -> Self {
        Self {
            root: PatriciaNode::new(),
            key_count: 0,
            max_keys,
            cache_valid: true,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), &'static str> {
        if self.key_count >= self.max_keys {
            return Err("Exceeded max number of keys in patricia tree");
        }

        let existed = Self::insert_recursive(&mut self.root, &key, value, 0)?;
        if !existed {
            self.key_count += 1;
        }
        self.invalidate_cache();
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        Self::get_recursive(&self.root, key, 0)
    }

    pub fn num_keys(&self) -> usize {
        self.key_count
    }

    pub fn root_hash(&mut self) -> Option<Vec<u8>> {
        if self.key_count == 0 {
            return None;
        }

        if !self.cache_valid {
            Self::compute_hashes(&mut self.root);
            self.cache_valid = true;
        }

        self.root.hash.clone()
    }

    pub fn get_proof(&mut self, key: &[u8]) -> Option<PatriciaProof> {
        if !self.cache_valid {
            Self::compute_hashes(&mut self.root);
            self.cache_valid = true;
        }

        let mut siblings = Vec::new();
        let mut directions = Vec::new();
        let mut path = Vec::new();

        if Self::get_proof_recursive(&self.root, key, 0, &mut siblings, &mut directions, &mut path) {
            Some(PatriciaProof {
                path,
                siblings,
                directions,
            })
        } else {
            None
        }
    }

    pub fn _verify_proof(&self, key: &[u8], value: &[u8], proof: &PatriciaProof, root_hash: &[u8]) -> bool {
        if proof.siblings.len() != proof.directions.len() {
            return false;
        }

        // Start with the leaf hash (key + value)
        let mut current_hash = PatriciaMerkleTree::hash_key_value(key, value);

        // Follow the proof path back to root
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

        current_hash == root_hash
    }

    fn insert_recursive(
        node: &mut PatriciaNode,
        key: &[u8],
        value: Vec<u8>,
        depth: usize,
    ) -> Result<bool, &'static str> {
        if depth >= MAX_LEVELS {
            return Err("Exceeded max depth in patricia tree");
        }

        node.invalidate_hash();

        // If this is a new empty node
        if node.key_fragment.is_empty() && node.value.is_none() && node.children.is_empty() {
            node.key_fragment = key[depth..].to_vec();
            node.value = Some(value);
            return Ok(false); // New key
        }

        // If this node has a key fragment
        if !node.key_fragment.is_empty() {
            let remaining_key = &key[depth..];
            let common_len = Self::common_prefix_length(&node.key_fragment, remaining_key);

            if common_len == node.key_fragment.len() && common_len == remaining_key.len() {
                // Exact match - update value
                let existed = node.value.is_some();
                node.value = Some(value);
                return Ok(existed);
            }

            if common_len < node.key_fragment.len() {
                // Need to split this node
                let old_fragment = node.key_fragment[common_len..].to_vec();
                let old_value = node.value.take();
                let old_children = std::mem::take(&mut node.children);

                // Update current node
                node.key_fragment = node.key_fragment[..common_len].to_vec();

                // Create child for old data
                let mut old_child = PatriciaNode::new_with_fragment(old_fragment[1..].to_vec());
                old_child.value = old_value;
                old_child.children = old_children;
                node.children.insert(old_fragment[0], Box::new(old_child));

                // Handle new key
                if common_len == remaining_key.len() {
                    node.value = Some(value);
                } else {
                    let new_fragment = remaining_key[common_len + 1..].to_vec();
                    let mut new_child = PatriciaNode::new_with_fragment(new_fragment);
                    new_child.value = Some(value);
                    node.children.insert(remaining_key[common_len], Box::new(new_child));
                }
                return Ok(false);
            }

            // common_len == node.key_fragment.len(), continue to children
        }

        let current_depth = depth + node.key_fragment.len();
        if current_depth >= key.len() {
            let existed = node.value.is_some();
            node.value = Some(value);
            return Ok(existed);
        }

        let next_byte = key[current_depth];
        if let Some(child) = node.children.get_mut(&next_byte) {
            Self::insert_recursive(child, key, value, current_depth + 1)
        } else {
            let fragment = key[current_depth + 1..].to_vec();
            let mut new_child = PatriciaNode::new_with_fragment(fragment);
            new_child.value = Some(value);
            node.children.insert(next_byte, Box::new(new_child));
            Ok(false)
        }
    }

    fn get_recursive(node: &PatriciaNode, key: &[u8], depth: usize) -> Option<Vec<u8>> {
        if !node.key_fragment.is_empty() {
            let remaining_key = &key[depth..];
            if remaining_key.len() < node.key_fragment.len() {
                return None;
            }
            if &remaining_key[..node.key_fragment.len()] != &node.key_fragment {
                return None;
            }
        }

        let current_depth = depth + node.key_fragment.len();
        if current_depth == key.len() {
            return node.value.clone();
        }

        if current_depth >= key.len() {
            return None;
        }

        let next_byte = key[current_depth];
        if let Some(child) = node.children.get(&next_byte) {
            Self::get_recursive(child, key, current_depth + 1)
        } else {
            None
        }
    }

    fn get_proof_recursive(
        node: &PatriciaNode,
        key: &[u8],
        depth: usize,
        siblings: &mut Vec<String>,
        directions: &mut Vec<bool>,
        path: &mut Vec<u8>,
    ) -> bool {
        // Add current node's fragment to path
        path.extend_from_slice(&node.key_fragment);

        if !node.key_fragment.is_empty() {
            let remaining_key = &key[depth..];
            if remaining_key.len() < node.key_fragment.len() {
                return false;
            }
            if &remaining_key[..node.key_fragment.len()] != &node.key_fragment {
                return false;
            }
        }

        let current_depth = depth + node.key_fragment.len();
        if current_depth == key.len() {
            return node.value.is_some();
        }

        if current_depth >= key.len() {
            return false;
        }

        let next_byte = key[current_depth];
        path.push(next_byte);

        if let Some(child) = node.children.get(&next_byte) {
            // Add sibling hashes for proof
            for (&byte, sibling) in &node.children {
                if byte != next_byte {
                    if let Some(hash) = &sibling.hash {
                        siblings.push(hex::encode(hash));
                        directions.push(byte > next_byte);
                    }
                }
            }

            return Self::get_proof_recursive(child, key, current_depth + 1, siblings, directions, path);
        }

        false
    }

    fn compute_hashes(node: &mut PatriciaNode) -> Vec<u8> {
        if let Some(ref hash) = node.hash {
            return hash.clone();
        }

        let hash = if node.is_leaf() {
            // Leaf node: hash(key_fragment + value)
            if let Some(ref value) = node.value {
                Self::hash_key_value(&node.key_fragment, value)
            } else {
                Self::hash_data(&node.key_fragment)
            }
        } else {
            // Internal node: hash children and combine
            let mut child_hashes = Vec::new();
            for (&byte, child) in &mut node.children {
                let child_hash = Self::compute_hashes(child);
                child_hashes.push((byte, child_hash));
            }
            child_hashes.sort_by_key(|(byte, _)| *byte);

            let mut combined = node.key_fragment.clone();
            for (_, hash) in child_hashes {
                combined.extend_from_slice(&hash);
            }
            Self::hash_data(&combined)
        };

        node.hash = Some(hash.clone());
        hash
    }

    fn common_prefix_length(a: &[u8], b: &[u8]) -> usize {
        a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
    }

    fn hash_key_value(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(key);
        hasher.update(value);
        hasher.finalize().to_vec()
    }

    fn hash_data(data: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    fn hash_pair(&self, left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Keccak256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().to_vec()
    }

    fn invalidate_cache(&mut self) {
        self.cache_valid = false;
        Self::invalidate_node_hashes(&mut self.root);
    }

    fn invalidate_node_hashes(node: &mut PatriciaNode) {
        node.invalidate_hash();
        for child in node.children.values_mut() {
            Self::invalidate_node_hashes(child);
        }
    }
}

impl Default for PatriciaMerkleTree {
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
        let mut tree = PatriciaMerkleTree::new();
        assert_eq!(tree.num_keys(), 0);
        assert!(tree.root_hash().is_none());
    }

    #[test]
    fn test_single_key() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(hex("key1"), hex("value1")).is_ok());
        assert_eq!(tree.num_keys(), 1);
        assert!(tree.root_hash().is_some());
        assert_eq!(tree.get(&hex("key1")), Some(hex("value1")));
    }

    #[test]
    fn test_multiple_keys() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(hex("key1"), hex("value1")).is_ok());
        assert!(tree.insert(hex("key2"), hex("value2")).is_ok());
        assert!(tree.insert(hex("key3"), hex("value3")).is_ok());

        assert_eq!(tree.num_keys(), 3);
        assert!(tree.root_hash().is_some());
        assert_eq!(tree.get(&hex("key1")), Some(hex("value1")));
        assert_eq!(tree.get(&hex("key2")), Some(hex("value2")));
        assert_eq!(tree.get(&hex("key3")), Some(hex("value3")));
    }

    #[test]
    fn test_update_existing_key() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(hex("key1"), hex("value1")).is_ok());
        assert_eq!(tree.num_keys(), 1);

        assert!(tree.insert(hex("key1"), hex("new_value")).is_ok());
        assert_eq!(tree.num_keys(), 1); // Should not increase
        assert_eq!(tree.get(&hex("key1")), Some(hex("new_value")));
    }

    #[test]
    fn test_get_nonexistent_key() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(hex("key1"), hex("value1")).is_ok());
        assert_eq!(tree.get(&hex("nonexistent")), None);
    }

    #[test]
    fn test_common_prefix_keys() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(hex("test"), hex("value1")).is_ok());
        assert!(tree.insert(hex("testing"), hex("value2")).is_ok());
        assert!(tree.insert(hex("tea"), hex("value3")).is_ok());

        assert_eq!(tree.num_keys(), 3);
        assert_eq!(tree.get(&hex("test")), Some(hex("value1")));
        assert_eq!(tree.get(&hex("testing")), Some(hex("value2")));
        assert_eq!(tree.get(&hex("tea")), Some(hex("value3")));
    }

    #[test]
    fn test_proof_generation_valid_and_invalid_key() {
        let mut tree = PatriciaMerkleTree::new();
        let _ = tree.insert(hex("key1"), hex("value1"));
        let _ = tree.insert(hex("key2"), hex("value2"));

        let valid_proof = tree.get_proof(&hex("key1"));
        assert!(valid_proof.is_some());

        let invalid_proof = tree.get_proof(&hex("nonexistent"));
        assert!(invalid_proof.is_none());
    }

    #[test]
    fn test_max_keys_enforced() {
        let test_max_keys = 10;
        let mut tree = PatriciaMerkleTree::_new_with_max(test_max_keys);

        for i in 0..test_max_keys {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            assert!(tree.insert(key, value).is_ok());
        }

        // This should fail
        let result = tree.insert(hex("overflow"), hex("value"));
        assert!(result.is_err());
        assert_eq!(tree.num_keys(), test_max_keys);
    }

    #[test]
    fn test_root_hash_consistency() {
        let mut tree = PatriciaMerkleTree::new();
        let _ = tree.insert(hex("key1"), hex("value1"));
        let _ = tree.insert(hex("key2"), hex("value2"));

        let hash1 = tree.root_hash();
        let hash2 = tree.root_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_root_hash_changes_after_insertion() {
        let mut tree = PatriciaMerkleTree::new();
        let _ = tree.insert(hex("key1"), hex("value1"));
        let hash1 = tree.root_hash();

        let _ = tree.insert(hex("key2"), hex("value2"));
        let hash2 = tree.root_hash();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_proof_structure() {
        let mut tree = PatriciaMerkleTree::new();
        let _ = tree.insert(hex("key1"), hex("value1"));
        let _ = tree.insert(hex("key2"), hex("value2"));

        let proof = tree.get_proof(&hex("key1"));
        assert!(proof.is_some());
        let proof = proof.unwrap();

        assert_eq!(proof.siblings.len(), proof.directions.len());
        assert!(!proof.path.is_empty());
    }

    #[test]
    fn test_proof_verification_valid() {
        let mut tree = PatriciaMerkleTree::new();
        let key = hex("test_key");
        let value = hex("test_value");
        let _ = tree.insert(key.clone(), value.clone());

        let root_hash = tree.root_hash().unwrap();
        let proof = tree.get_proof(&key).unwrap();

        assert!(tree._verify_proof(&key, &value, &proof, &root_hash));
    }

    #[test]
    fn test_proof_verification_invalid_value() {
        let mut tree = PatriciaMerkleTree::new();
        let key = hex("test_key");
        let value = hex("test_value");
        let _ = tree.insert(key.clone(), value);

        let root_hash = tree.root_hash().unwrap();
        let proof = tree.get_proof(&key).unwrap();

        // Wrong value
        assert!(!tree._verify_proof(&key, &hex("wrong_value"), &proof, &root_hash));
    }

    #[test]
    fn test_proof_verification_invalid_root() {
        let mut tree = PatriciaMerkleTree::new();
        let key = hex("test_key");
        let value = hex("test_value");
        let _ = tree.insert(key.clone(), value.clone());

        let proof = tree.get_proof(&key).unwrap();
        let wrong_root = hex("wrong_root");

        assert!(!tree._verify_proof(&key, &value, &proof, &wrong_root));
    }

    #[test]
    fn test_proof_verification_malformed_proof() {
        let tree = PatriciaMerkleTree::new();
        let key = hex("test");
        let value = hex("value");
        let root = hex("root");

        // Mismatched siblings and directions length
        let malformed_proof = PatriciaProof {
            path: vec![],
            siblings: vec!["abc".to_string()],
            directions: vec![true, false], // Different length
        };

        assert!(!tree._verify_proof(&key, &value, &malformed_proof, &root));

        // Invalid hex in siblings
        let invalid_hex_proof = PatriciaProof {
            path: vec![],
            siblings: vec!["invalid_hex_string".to_string()],
            directions: vec![true],
        };

        assert!(!tree._verify_proof(&key, &value, &invalid_hex_proof, &root));
    }

    #[test]
    fn test_key_prefix_edge_cases() {
        let mut tree = PatriciaMerkleTree::new();
        
        // Test keys where one is prefix of another
        assert!(tree.insert(hex("a"), hex("value_a")).is_ok());
        assert!(tree.insert(hex("ab"), hex("value_ab")).is_ok());
        assert!(tree.insert(hex("abc"), hex("value_abc")).is_ok());

        assert_eq!(tree.get(&hex("a")), Some(hex("value_a")));
        assert_eq!(tree.get(&hex("ab")), Some(hex("value_ab")));
        assert_eq!(tree.get(&hex("abc")), Some(hex("value_abc")));
        assert_eq!(tree.num_keys(), 3);
    }

    #[test]
    fn test_empty_key() {
        let mut tree = PatriciaMerkleTree::new();
        assert!(tree.insert(vec![], hex("empty_key_value")).is_ok());
        assert_eq!(tree.get(&vec![]), Some(hex("empty_key_value")));
        assert_eq!(tree.num_keys(), 1);
    }
}
