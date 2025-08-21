use merkle_tree_api::lmdb_tree::LmdbMerkleTree;
use tempfile::TempDir;

fn create_temp_tree() -> (LmdbMerkleTree, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let tree = LmdbMerkleTree::new(db_path.to_str().unwrap()).unwrap();
    (tree, temp_dir)
}

fn hex(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn test_lmdb_tree_new() {
    let (_tree, _temp_dir) = create_temp_tree();
    // Tree created successfully - no panic means success
}

#[test]
fn test_empty_tree() {
    let (tree, _temp_dir) = create_temp_tree();
    assert_eq!(tree.num_leaves(), 0);
    assert!(tree.root().is_none());
}

#[test]
fn test_single_leaf() {
    let (tree, _temp_dir) = create_temp_tree();

    assert!(tree.add_leaf(hex("leaf1")).is_ok());
    assert_eq!(tree.num_leaves(), 1);
    assert!(tree.root().is_some());
}

#[test]
fn test_multiple_leaves() {
    let (tree, _temp_dir) = create_temp_tree();

    assert!(tree.add_leaf(hex("leaf1")).is_ok());
    assert!(tree.add_leaf(hex("leaf2")).is_ok());
    assert!(tree.add_leaf(hex("leaf3")).is_ok());

    assert_eq!(tree.num_leaves(), 3);
    assert!(tree.root().is_some());
}

#[test]
fn test_add_leaves_batch() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("leaf1"), hex("leaf2"), hex("leaf3")];
    assert!(tree.add_leaves(leaves).is_ok());
    assert_eq!(tree.num_leaves(), 3);
    assert!(tree.root().is_some());
}

#[test]
fn test_proof_generation_valid_and_invalid_index() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    assert!(tree.add_leaves(leaves).is_ok());

    let valid_proof = tree.get_proof(0);
    assert!(valid_proof.is_some());

    let invalid_proof = tree.get_proof(999);
    assert!(invalid_proof.is_none());
}

#[test]
fn test_root_consistency() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("x"), hex("y"), hex("z")];
    assert!(tree.add_leaves(leaves).is_ok());

    let root1 = tree.root();
    let root2 = tree.root();

    assert_eq!(root1, root2);
    assert!(root1.is_some());
}

#[test]
fn test_root_changes_after_addition() {
    let (tree, _temp_dir) = create_temp_tree();

    assert!(tree.add_leaf(hex("1")).is_ok());
    let root1 = tree.root();

    assert!(tree.add_leaf(hex("2")).is_ok());
    let root2 = tree.root();

    assert_ne!(root1, root2);
    assert!(root1.is_some());
    assert!(root2.is_some());
}

#[test]
fn test_proof_structure() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    assert!(tree.add_leaves(leaves).is_ok());

    let proof = tree.get_proof(2);
    assert!(proof.is_some());
    let proof = proof.unwrap();

    assert!(proof.siblings.len() > 0);
}

#[test]
fn test_proof_verification_valid() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    assert!(tree.add_leaves(leaves.clone()).is_ok());

    let root = tree.root().unwrap();
    let proof = tree.get_proof(1).unwrap();

    assert!(tree.verify_proof(&leaves[1], &proof, &root, 1));
}

#[test]
fn test_proof_verification_invalid_leaf() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    assert!(tree.add_leaves(leaves).is_ok());

    let root = tree.root().unwrap();
    let proof = tree.get_proof(1).unwrap();

    // Wrong leaf data
    assert!(!tree.verify_proof(&hex("wrong"), &proof, &root, 1));
}

#[test]
fn test_proof_verification_invalid_root() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaves = vec![hex("a"), hex("b"), hex("c"), hex("d")];
    assert!(tree.add_leaves(leaves.clone()).is_ok());

    let proof = tree.get_proof(1).unwrap();
    let wrong_root = hex("wrong_root");

    assert!(!tree.verify_proof(&leaves[1], &proof, &wrong_root, 1));
}

#[test]
fn test_proof_verification_single_leaf() {
    let (tree, _temp_dir) = create_temp_tree();

    let leaf = hex("single");
    assert!(tree.add_leaf(leaf.clone()).is_ok());

    let root = tree.root().unwrap();
    let proof = tree.get_proof(0).unwrap();

    assert!(tree.verify_proof(&leaf, &proof, &root, 0));
}

#[test]
fn test_persistence_across_instances() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.to_str().unwrap();

    // First instance - add some data
    {
        let tree = LmdbMerkleTree::new(db_path_str).unwrap();
        let leaves = vec![hex("persist1"), hex("persist2"), hex("persist3")];
        assert!(tree.add_leaves(leaves).is_ok());
        assert_eq!(tree.num_leaves(), 3);
    }

    // Second instance - should load existing data
    {
        let tree = LmdbMerkleTree::new(db_path_str).unwrap();
        assert_eq!(tree.num_leaves(), 3);
        assert!(tree.root().is_some());

        // Add more data
        assert!(tree.add_leaf(hex("persist4")).is_ok());
        assert_eq!(tree.num_leaves(), 4);
    }

    // Third instance - should have all data
    {
        let tree = LmdbMerkleTree::new(db_path_str).unwrap();
        assert_eq!(tree.num_leaves(), 4);
        assert!(tree.root().is_some());

        // Should be able to get proofs for all indices
        for i in 0..4 {
            assert!(tree.get_proof(i).is_some());
        }
    }
}

#[test]
fn test_large_batch_operations() {
    let (tree, _temp_dir) = create_temp_tree();

    // Add 100 leaves in batches
    let mut all_leaves = Vec::new();
    for batch in 0..10 {
        let batch_leaves: Vec<Vec<u8>> = (0..10)
            .map(|i| format!("leaf_{}_{}", batch, i).into_bytes())
            .collect();
        all_leaves.extend(batch_leaves.clone());
        assert!(tree.add_leaves(batch_leaves).is_ok());
    }

    assert_eq!(tree.num_leaves(), 100);
    assert!(tree.root().is_some());

    // Test proof generation for various indices
    assert!(tree.get_proof(0).is_some());
    assert!(tree.get_proof(50).is_some());
    assert!(tree.get_proof(99).is_some());
    assert!(tree.get_proof(100).is_none());
}

#[test]
fn test_mixed_single_and_batch_operations() {
    let (tree, _temp_dir) = create_temp_tree();

    // Add single leaf
    assert!(tree.add_leaf(hex("single1")).is_ok());
    assert_eq!(tree.num_leaves(), 1);

    // Add batch
    let batch = vec![hex("batch1"), hex("batch2")];
    assert!(tree.add_leaves(batch).is_ok());
    assert_eq!(tree.num_leaves(), 3);

    // Add another single leaf
    assert!(tree.add_leaf(hex("single2")).is_ok());
    assert_eq!(tree.num_leaves(), 4);

    // Verify all operations worked
    assert!(tree.root().is_some());
    for i in 0..4 {
        assert!(tree.get_proof(i).is_some());
    }
}
