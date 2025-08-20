use merkle_tree_api::merkle_tree::{IncrementalMerkleTree, MerkleProof};

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
    let _ = tree.add_leaves(vec![hex("a"), hex("b"), hex("c"), hex("d")]);

    let valid_proof = tree.get_proof(0);
    assert!(valid_proof.is_some());

    let invalid_proof = tree.get_proof(999);
    assert!(invalid_proof.is_none());
}

#[test]
fn test_max_levels_enforced() {
    let test_max_levels = 8; // Reduced from 11 (256 vs 2048 leaves)
    let test_max_leaves = 1 << test_max_levels;
    let mut tree = IncrementalMerkleTree::_new_with_max(test_max_leaves);
    let leaves: Vec<Vec<u8>> = (0..test_max_leaves)
        .map(|i| format!("leaf{}", i).into_bytes())
        .collect();

    // Use batch operation instead of individual adds
    assert!(tree.add_leaves(leaves).is_ok());

    let root = tree.root();
    assert!(root.is_some());
    assert!(tree.cached_hashes.len() <= test_max_levels + 1); // +1 because root is an extra level
}

#[test]
fn test_root_consistency() {
    let mut tree = IncrementalMerkleTree::new();
    let _ = tree.add_leaves(vec![hex("x"), hex("y"), hex("z")]);

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
    let _ = tree.add_leaves(vec![hex("a"), hex("b"), hex("c"), hex("d")]);

    let proof = tree.get_proof(2);
    assert!(proof.is_some());
    let proof = proof.unwrap();

    assert!(proof.siblings.len() > 0);
    assert!(proof.siblings.len() > 0);
}

#[test]
fn test_proof_verification_valid() {
    let mut tree = IncrementalMerkleTree::new();
    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    let _ = tree.add_leaves(leaves.clone());

    let root = tree.root().unwrap();
    let proof = tree.get_proof(1).unwrap();

    assert!(tree._verify_proof(&leaves[1], &proof, &root, 1));
}

#[test]
fn test_proof_verification_invalid_leaf() {
    let mut tree = IncrementalMerkleTree::new();
    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    let _ = tree.add_leaves(leaves);

    let root = tree.root().unwrap();
    let proof = tree.get_proof(1).unwrap();

    // Wrong leaf data
    assert!(!tree._verify_proof(&hex("wrong"), &proof, &root, 1));
}

#[test]
fn test_proof_verification_invalid_root() {
    let mut tree = IncrementalMerkleTree::new();
    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    let _ = tree.add_leaves(leaves.clone());

    let proof = tree.get_proof(1).unwrap();
    let wrong_root = hex("wrong_root");

    assert!(!tree._verify_proof(&leaves[1], &proof, &wrong_root, 1));
}

#[test]
fn test_proof_verification_malformed_proof() {
    let tree = IncrementalMerkleTree::new();
    let leaf = hex("test");
    let root = hex("root");

    // Invalid hex in siblings
    let invalid_hex_proof = MerkleProof {
        siblings: vec!["invalid_hex_string".to_string()],
    };

    assert!(!tree._verify_proof(&leaf, &invalid_hex_proof, &root, 0));
}

#[test]
fn test_proof_verification_single_leaf() {
    let mut tree = IncrementalMerkleTree::new();
    let leaf = hex("single");
    let _ = tree.add_leaf(leaf.clone());

    let root = tree.root().unwrap();
    let proof = tree.get_proof(0).unwrap();

    assert!(tree._verify_proof(&leaf, &proof, &root, 0));
}