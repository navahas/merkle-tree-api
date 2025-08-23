use crate::storage::LmdbStorage;
use serde::Serialize;
use sha3::{Digest, Keccak256};

/// Use <= 64 so we can index nodes with u64.
/// If you later want 256, change storage keying to accept wider indices (e.g., hex path).
pub const DEFAULT_DEPTH: usize = 32;

#[derive(Debug, Clone, Serialize)]
pub struct SparseMerkleProof {
    /// One sibling per level from leaf (level 0) up to (depth - 1), hex-encoded
    pub siblings: Vec<String>,
}

#[derive(Debug)]
pub struct SparseMerkleTree {
    storage: LmdbStorage,
    /// empty[level] is the default hash for a subtree whose height == level
    /// empty[0] is the default *leaf*; empty[depth] is the empty root
    empty: Vec<[u8; 32]>,
    depth: usize,
}

impl SparseMerkleTree {
    /// Create a new SMT over LMDB, with `depth` <= 64 (fits u64 indices).
    pub fn new(storage_path: &str, depth: usize) -> Result<Self, Box<dyn std::error::Error>> {
        assert!(depth > 0 && depth <= 64, "depth must be in 1..=64");
        let storage = LmdbStorage::new(storage_path)?;
        let empty = Self::compute_empty(depth);
        Ok(Self {
            storage,
            empty,
            depth,
        })
    }

    /// Convenience: keep your default depth (32).
    pub fn new_default(storage_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new(storage_path, DEFAULT_DEPTH)
    }

    /* ----------------------------- Public API ----------------------------- */

    /// Insert or update a (key,value) pair.
    /// Path is derived from the first `depth` bits of Keccak256(key).
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let kh = keccak(key);
        let leaf_hash = self.hash_leaf(&kh, value);
        let mut idx = self.leaf_index_from_key_hash(&kh);

        // Collect all updates and write in one RW txn (atomic & fast).
        let mut updates: Vec<(usize, u64, Vec<u8>)> = Vec::with_capacity(self.depth + 1);

        // Store leaf at level 0.
        updates.push((0, idx, leaf_hash.to_vec()));

        // Walk up to the root.
        let mut cur = leaf_hash;
        for level in 0..self.depth {
            let sib_idx = idx ^ 1;
            let sib = self
                .storage
                .get_node(level, sib_idx)?
                .unwrap_or_else(|| self.empty[level].to_vec());

            let parent = if idx & 1 == 0 {
                self.hash_internal(&cur, &sib)
            } else {
                self.hash_internal(&sib, &cur)
            };

            idx >>= 1;
            updates.push((level + 1, idx, parent.to_vec()));
            cur = parent;
        }

        // Batch write (single transaction).
        self.storage.store_path_batch(&updates)?;
        Ok(())
    }

    /// Return the current root. If never written, return the empty root.
    pub fn root(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(self
            .storage
            .get_node(self.depth, 0)?
            .unwrap_or_else(|| self.empty[self.depth].to_vec()))
    }

    /// Generate a membership (or absence) proof for `key`.
    /// Siblings default to the correct per-level empty when not materialized.
    pub fn get_proof(&self, key: &[u8]) -> Result<SparseMerkleProof, Box<dyn std::error::Error>> {
        let kh = keccak(key);
        let mut idx = self.leaf_index_from_key_hash(&kh);

        let mut siblings = Vec::with_capacity(self.depth);
        for level in 0..self.depth {
            let sib_idx = idx ^ 1;
            let sib = self
                .storage
                .get_node(level, sib_idx)?
                .unwrap_or_else(|| self.empty[level].to_vec());
            siblings.push(hex::encode(sib));
            idx >>= 1;
        }
        Ok(SparseMerkleProof { siblings })
    }

    /// Verify a membership proof (key,value) against `root`.
    pub fn verify_membership(
        &self,
        key: &[u8],
        value: &[u8],
        proof: &SparseMerkleProof,
        root: &[u8],
    ) -> bool {
        if proof.siblings.len() != self.depth {
            return false;
        }
        let kh = keccak(key);
        let mut cur = self.hash_leaf(&kh, value);
        let mut idx = self.leaf_index_from_key_hash(&kh);

        for sib_hex in &proof.siblings {
            let sib = match hex::decode(sib_hex) {
                Ok(b) => b,
                Err(_) => return false,
            };
            cur = if idx & 1 == 0 {
                self.hash_internal(&cur, &sib)
            } else {
                self.hash_internal(&sib, &cur)
            };
            idx >>= 1;
        }
        cur.as_slice() == root
    }

    /// Verify a *non-membership* proof for `key` against `root`.
    /// This treats the leaf hash as the default empty leaf and checks the recomputed root.
    pub fn verify_non_membership(
        &self,
        key: &[u8],
        proof: &SparseMerkleProof,
        root: &[u8],
    ) -> bool {
        if proof.siblings.len() != self.depth {
            return false;
        }
        let kh = keccak(key);
        let mut cur = self.empty[0]; // empty leaf
        let mut idx = self.leaf_index_from_key_hash(&kh);

        for sib_hex in &proof.siblings {
            let sib = match hex::decode(sib_hex) {
                Ok(b) => b,
                Err(_) => return false,
            };
            cur = if idx & 1 == 0 {
                self.hash_internal(&cur, &sib)
            } else {
                self.hash_internal(&sib, &cur)
            };
            idx >>= 1;
        }
        cur.as_slice() == root
    }

    /* ---------------------------- Internals ------------------------------- */

    /// Per-level empties with domain separation:
    /// empty[0]   = H(0x02)         // empty leaf seed
    /// empty[i+1] = H(0x01 || empty[i] || empty[i])  // internal from two empties
    fn compute_empty(depth: usize) -> Vec<[u8; 32]> {
        let mut out = Vec::with_capacity(depth + 1);

        let mut h0 = Keccak256::new();
        h0.update([0x02u8]); // domain: empty seed
        out.push(h0.finalize().into());

        for i in 0..depth {
            let mut hi = Keccak256::new();
            hi.update([0x01u8]); // domain: internal
            hi.update(out[i]);
            hi.update(out[i]);
            out.push(hi.finalize().into());
        }
        out
    }

    /// Hash a leaf with domain separation and key binding:
    /// H(0x00 || H(key) || H(value))
    fn hash_leaf(&self, key_hash: &[u8; 32], value: &[u8]) -> [u8; 32] {
        let mut hv = Keccak256::new();
        hv.update(value);
        let vh: [u8; 32] = hv.finalize().into();

        let mut h = Keccak256::new();
        h.update([0x00u8]); // domain: leaf
        h.update(key_hash);
        h.update(vh);
        h.finalize().into()
    }

    /// Hash an internal node with domain separation:
    /// H(0x01 || left || right)
    fn hash_internal(&self, left: &[u8], right: &[u8]) -> [u8; 32] {
        let mut h = Keccak256::new();
        h.update([0x01u8]); // domain: internal
        h.update(left);
        h.update(right);
        h.finalize().into()
    }

    /// Convert H(key) into a u64 leaf index using the first `depth` bits (MSB-first).
    #[inline]
    fn leaf_index_from_key_hash(&self, kh: &[u8; 32]) -> u64 {
        let mut idx: u64 = 0;
        for i in 0..self.depth {
            idx <<= 1;
            idx |= bit_at(kh, i) as u64;
        }
        idx
    }
}

#[inline]
fn keccak(data: &[u8]) -> [u8; 32] {
    let mut h = Keccak256::new();
    h.update(data);
    h.finalize().into()
}

/// MSB-first bit access: i in [0..256)
#[inline]
fn bit_at(hash: &[u8; 32], i: usize) -> u8 {
    debug_assert!(i < 256);
    let byte = hash[i / 8];
    let off = 7 - (i % 8);
    (byte >> off) & 1
}
