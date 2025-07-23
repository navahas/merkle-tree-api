use serde::Serialize;
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use hex::{encode as hex_encode, decode as hex_decode};

const MAX_LEVELS: usize = 32;
const MAX_KEYS: usize = 1 << MAX_LEVELS;

#[derive(Debug, Clone, Serialize)]
pub struct VerkleProof {
    pub path: Vec<u8>,
    pub siblings: Vec<String>,
    pub indices: Vec<usize>,
    pub siblings_per_level: Vec<usize>,
}

#[derive(Debug, Clone)]
struct VerkleNode {
    commitment: Option<Vec<u8>>,
    values: BTreeMap<u8, Vec<u8>>,
    children: BTreeMap<u8, Box<VerkleNode>>,
}

impl VerkleNode {
    fn new() -> Self {
        Self {
            commitment: None,
            values: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    fn invalidate(&mut self) {
        self.commitment = None;
        for child in self.children.values_mut() {
            child.invalidate();
        }
    }
}

#[derive(Debug)]
pub struct VerkleTree {
    root: VerkleNode,
    key_count: usize,
    max_keys: usize,
    cache_valid: bool,
}

impl VerkleTree {
    pub fn new() -> Self {
        Self {
            root: VerkleNode::new(),
            key_count: 0,
            max_keys: MAX_KEYS,
            cache_valid: false,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), &'static str> {
        if self.key_count >= self.max_keys {
            return Err("Tree is full");
        }
        let existed = Self::insert_rec(&mut self.root, &key, &value, 0);
        if !existed {
            self.key_count += 1;
        }
        self.root.invalidate();
        self.cache_valid = false;
        Ok(())
    }

    fn insert_rec(node: &mut VerkleNode, key: &[u8], value: &[u8], depth: usize) -> bool {
        if key.is_empty() {
            return node.values.insert(0, value.to_vec()).is_some();
        }
        if depth == key.len() - 1 {
            let b = key[depth];
            return node.values.insert(b, value.to_vec()).is_some();
        }
        if depth < key.len() {
            let b = key[depth];
            let child = node.children
                .entry(b)
                .or_insert_with(|| Box::new(VerkleNode::new()));
            return Self::insert_rec(child, key, value, depth + 1);
        }
        false
    }

    pub fn get_proof(&mut self, key: &[u8]) -> Option<VerkleProof> {
        self.compute_commitments();
        let mut path = Vec::new();
        let mut siblings = Vec::new();
        let mut indices = Vec::new();
        let mut per_level = Vec::new();
        if Self::build_proof(
            &self.root, key, 0,
            &mut path, &mut siblings, &mut indices, &mut per_level
        ) {
            Some(VerkleProof { path, siblings, indices, siblings_per_level: per_level })
        } else {
            None
        }
    }

    fn build_proof(
        node: &VerkleNode,
        key: &[u8],
        depth: usize,
        path: &mut Vec<u8>,
        siblings: &mut Vec<String>,
        indices: &mut Vec<usize>,
        per_level: &mut Vec<usize>,
    ) -> bool {
        if depth >= key.len() {
            return false;
        }
        let byte = key[depth];
        path.push(byte);

        // Leaf level
        if depth == key.len() - 1 {
            if !node.values.contains_key(&byte) {
                return false;
            }
            let mut count = 0;

            // 1) value‐siblings
            for (&k, v) in node.values.iter() {
                if k != byte {
                    let leaf_h = Self::hash_leaf(&[k], v);
                    siblings.push(hex_encode(&leaf_h));
                    indices.push(k as usize);
                    count += 1;
                }
            }
            // 2) child‐siblings
            for (&k, child) in node.children.iter() {
                let commit = child.commitment.as_ref().unwrap();
                siblings.push(hex_encode(commit));
                indices.push(k as usize);
                count += 1;
            }

            per_level.push(count);
            return true;
        }

        // Internal node: descend
        if let Some(child) = node.children.get(&byte) {
            let mut count = 0;

            // 1) value‐siblings at this level
            for (&k, v) in node.values.iter() {
                let leaf_h = Self::hash_leaf(&[k], v);
                siblings.push(hex_encode(&leaf_h));
                indices.push(k as usize);
                count += 1;
            }
            // 2) child‐siblings at this level (excluding the branch we follow)
            for (&k, sib) in node.children.iter() {
                if k != byte {
                    let commit = sib.commitment.as_ref().unwrap();
                    siblings.push(hex_encode(commit));
                    indices.push(k as usize);
                    count += 1;
                }
            }

            per_level.push(count);
            return Self::build_proof(child, key, depth + 1, path, siblings, indices, per_level);
        }

        false
    }

    pub fn root_commitment(&mut self) -> Option<Vec<u8>> {
        self.compute_commitments();
        self.root.commitment.clone()
    }

    fn compute_commitments(&mut self) {
        if !self.cache_valid {
            Self::compute_commitments_rec(&mut self.root);
            self.cache_valid = true;
        }
    }

    fn compute_commitments_rec(node: &mut VerkleNode) -> Vec<u8> {
        let mut data = Vec::new();
        // 1) each value‐slot: k || hash_leaf(k, v)
        for (&k, v) in node.values.iter() {
            let leaf_h = Self::hash_leaf(&[k], v);
            data.push(k);
            data.extend_from_slice(&leaf_h);
        }
        // 2) each child‐slot: k || child_commit
        for (&k, child) in node.children.iter_mut() {
            let commit = Self::compute_commitments_rec(child);
            data.push(k);
            data.extend_from_slice(&commit);
        }
        let hash = Self::hash_node(&data);
        node.commitment = Some(hash.clone());
        hash
    }

    pub fn _verify_proof(
        &self,
        key: &[u8],
        value: &[u8],
        proof: &VerkleProof,
        expected_root: &[u8],
    ) -> bool {
        self.simulate_commitment(key, value, proof) == expected_root
    }

    pub fn simulate_commitment(
        &self,
        key: &[u8],
        value: &[u8],
        proof: &VerkleProof,
    ) -> Vec<u8> {
        // Must match both shapes
        if proof.siblings.len() != proof.indices.len()
            || proof.path.len() != proof.siblings_per_level.len()
        {
            return vec![];
        }

        let depth = proof.path.len();
        let mut sib_idx = 0;

        // --- Leaf level reconstruction ---
        let last = *key.last().unwrap_or(&0);
        let mut entries = Vec::new();

        // a) our own leaf‐commit
        let my_leaf = Self::hash_leaf(&[last], value);
        entries.push((last, my_leaf));

        // b) siblings at leaf
        let n_leaf = proof.siblings_per_level[depth - 1];
        for _ in 0..n_leaf {
            let idx = proof.indices[sib_idx] as u8;
            let sib_h = hex_decode(&proof.siblings[sib_idx]).unwrap_or_default();
            entries.push((idx, sib_h));
            sib_idx += 1;
        }

        // c) sort by slot then hash
        entries.sort_by_key(|(k, _)| *k);
        let mut buf = Vec::new();
        for (k, h) in &entries {
            buf.push(*k);
            buf.extend_from_slice(h);
        }
        let mut commit = Self::hash_node(&buf);

        // --- Ascend ancestors, bottom‐up ---
        for (i, &slot) in proof.path[..depth - 1].iter().enumerate().rev() {
            // a) collect siblings _first_
            let mut lvl = Vec::new();
            let n = proof.siblings_per_level[i];
            for _ in 0..n {
                let idx = proof.indices[sib_idx] as u8;
                let sib_h = hex_decode(&proof.siblings[sib_idx]).unwrap_or_default();
                lvl.push((idx, sib_h));
                sib_idx += 1;
            }

            // b) then our child‐commit
            lvl.push((slot, commit.clone()));

            // c) stable‐sort by slot (equal‐key entries keep this order)
            lvl.sort_by_key(|(k, _)| *k);

            // d) pack & hash
            let mut buf2 = Vec::new();
            for (k, h) in &lvl {
                buf2.push(*k);
                buf2.extend_from_slice(h);
            }
            commit = Self::hash_node(&buf2);
        }

        commit
    }

    fn hash_leaf(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut h = Keccak256::new();
        h.update(b"leaf:");
        h.update(key);
        h.update(value);
        h.finalize().to_vec()
    }

    fn hash_node(data: &[u8]) -> Vec<u8> {
        let mut h = Keccak256::new();
        h.update(b"node:");
        h.update(data);
        h.finalize().to_vec()
    }
}

impl Default for VerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_and_verification() {
        let mut tree = VerkleTree::new();
        let key = b"mykey".to_vec();
        let value = b"myvalue".to_vec();
        tree.insert(key.clone(), value.clone()).unwrap();
        let root = tree.root_commitment().unwrap();

        let proof = tree.get_proof(&key).expect("Proof generation failed");
        assert_eq!(tree.simulate_commitment(&key, &value, &proof), root);
        assert!(tree._verify_proof(&key, &value, &proof, &root));
    }

    #[test]
    fn test_insert_multiple_keys_and_root_changes() {
        let mut tree = VerkleTree::new();
        let k1 = b"a".to_vec(); let v1 = b"1".to_vec();
        tree.insert(k1.clone(), v1.clone()).unwrap();
        let r1 = tree.root_commitment().unwrap();

        let k2 = b"b".to_vec(); let v2 = b"2".to_vec();
        tree.insert(k2.clone(), v2.clone()).unwrap();
        let r2 = tree.root_commitment().unwrap();

        assert_ne!(r1, r2);
        let p1 = tree.get_proof(&k1).unwrap();
        assert!(tree._verify_proof(&k1, &v1, &p1, &r2));
        let p2 = tree.get_proof(&k2).unwrap();
        assert!(tree._verify_proof(&k2, &v2, &p2, &r2));
    }

    #[test]
    fn test_overwrite_key_does_not_increase_count() {
        let mut tree = VerkleTree::new();
        tree.insert(b"x".to_vec(), b"old".to_vec()).unwrap();
        let c1 = tree.key_count;
        tree.insert(b"x".to_vec(), b"new".to_vec()).unwrap();
        assert_eq!(c1, tree.key_count);
    }

    #[test]
    fn test_tree_full_error() {
        let mut tree = VerkleTree::new();
        tree.max_keys = 1;
        assert!(tree.insert(b"a".to_vec(), b"1".to_vec()).is_ok());
        assert!(tree.insert(b"b".to_vec(), b"2".to_vec()).is_err());
    }

    #[test]
    fn test_get_proof_nonexistent_key() {
        let mut tree = VerkleTree::new();
        tree.insert(b"foo".to_vec(), b"bar".to_vec()).unwrap();
        assert!(tree.get_proof(&b"baz".to_vec()).is_none());
    }

    #[test]
    fn test_prefix_and_child_keys() {
        let mut tree = VerkleTree::new();
        let full = vec![1,2,3];
        let pref = vec![1,2];
        tree.insert(full.clone(), b"F".to_vec()).unwrap();
        tree.insert(pref.clone(), b"I".to_vec()).unwrap();
        let root = tree.root_commitment().unwrap();

        let p_pref = tree.get_proof(&pref).unwrap();
        assert!(tree._verify_proof(&pref, &b"I".to_vec(), &p_pref, &root));
        let p_full = tree.get_proof(&full).unwrap();
        assert!(tree._verify_proof(&full, &b"F".to_vec(), &p_full, &root));
    }

    #[test]
    fn test_empty_key_insert_and_no_proof() {
        let mut tree = VerkleTree::new();
        tree.insert(Vec::new(), b"E".to_vec()).unwrap();
        let _ = tree.root_commitment().unwrap();
        assert!(tree.get_proof(&Vec::new()).is_none());
    }

    #[test]
    fn test_multiple_leaf_siblings_count() {
        let mut tree = VerkleTree::new();
        tree.insert(vec![0,1], b"A".to_vec()).unwrap();
        tree.insert(vec![0,2], b"B".to_vec()).unwrap();
        tree.insert(vec![0,3], b"C".to_vec()).unwrap();
        let root = tree.root_commitment().unwrap();

        let p = tree.get_proof(&vec![0,2]).unwrap();
        assert_eq!(*p.siblings_per_level.last().unwrap(), 2);
        assert!(tree._verify_proof(&vec![0,2], &b"B".to_vec(), &p, &root));
    }

    #[test]
    fn test_simulate_with_invalid_proof() {
        let tree = VerkleTree::new();
        let bad = VerkleProof {
            path: vec![1,2,3],
            siblings: vec!["xx".into()],
            indices: vec![0,1],
            siblings_per_level: vec![1,1,1],
        };
        assert!(tree.simulate_commitment(&b"".to_vec(), &b"".to_vec(), &bad).is_empty());
    }

    #[test]
    fn test_default_new_equivalence() {
        let a = VerkleTree::new();
        let b = VerkleTree::default();
        assert_eq!(a.key_count, b.key_count);
        assert_eq!(a.max_keys, b.max_keys);
        assert_eq!(a.cache_valid, b.cache_valid);
    }
}
