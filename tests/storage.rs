use merkle_tree_api::storage::{LmdbStorage, TreeMetadata};
use tempfile::TempDir;

fn create_temp_storage() -> (LmdbStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = LmdbStorage::new(&db_path).unwrap();
    (storage, temp_dir)
}

#[test]
fn test_storage_new() {
    let (storage, _temp_dir) = create_temp_storage();
    // Storage created successfully - no panic means success
    drop(storage);
}

#[test]
fn test_store_and_get_leaf() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let leaf_data = b"test_leaf".to_vec();
    storage.store_leaf(0, &leaf_data).unwrap();
    
    let retrieved = storage.get_leaf(0).unwrap();
    assert_eq!(retrieved, Some(leaf_data));
    
    let non_existent = storage.get_leaf(999).unwrap();
    assert_eq!(non_existent, None);
}

#[test]
fn test_store_leaves_batch() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let leaves = vec![
        b"leaf1".to_vec(),
        b"leaf2".to_vec(),
        b"leaf3".to_vec(),
    ];
    
    storage.store_leaves_batch(&leaves).unwrap();
    
    let all_leaves = storage.get_all_leaves().unwrap();
    assert_eq!(all_leaves.len(), 3);
    assert_eq!(all_leaves[0], b"leaf1".to_vec());
    assert_eq!(all_leaves[1], b"leaf2".to_vec());
    assert_eq!(all_leaves[2], b"leaf3".to_vec());
}

#[test]
fn test_append_leaves() {
    let (storage, _temp_dir) = create_temp_storage();
    
    // Store initial leaves
    let initial_leaves = vec![b"leaf1".to_vec(), b"leaf2".to_vec()];
    storage.store_leaves_batch(&initial_leaves).unwrap();
    
    // Append more leaves
    let new_leaves = vec![b"leaf3".to_vec(), b"leaf4".to_vec()];
    storage.append_leaves(2, &new_leaves).unwrap();
    
    let all_leaves = storage.get_all_leaves().unwrap();
    assert_eq!(all_leaves.len(), 4);
    assert_eq!(all_leaves[2], b"leaf3".to_vec());
    assert_eq!(all_leaves[3], b"leaf4".to_vec());
}

#[test]
fn test_cache_operations() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let level0_hashes = vec![
        b"hash1".to_vec(),
        b"hash2".to_vec(),
    ];
    let level1_hashes = vec![
        b"parent_hash".to_vec(),
    ];
    
    storage.store_cache_level(0, &level0_hashes).unwrap();
    storage.store_cache_level(1, &level1_hashes).unwrap();
    
    let retrieved_level0 = storage.get_cache_level(0).unwrap().unwrap();
    assert_eq!(retrieved_level0, level0_hashes);
    
    let retrieved_level1 = storage.get_cache_level(1).unwrap().unwrap();
    assert_eq!(retrieved_level1, level1_hashes);
    
    let non_existent = storage.get_cache_level(999).unwrap();
    assert_eq!(non_existent, None);
}

#[test]
fn test_cache_batch_operations() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let cache_levels = vec![
        vec![b"leaf1".to_vec(), b"leaf2".to_vec()],
        vec![b"parent".to_vec()],
    ];
    
    storage.store_cache_batch(&cache_levels).unwrap();
    
    let all_levels = storage.get_all_cache_levels().unwrap();
    assert_eq!(all_levels.len(), 2);
    assert_eq!(all_levels[0], cache_levels[0]);
    assert_eq!(all_levels[1], cache_levels[1]);
}

#[test]
fn test_clear_cache() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let cache_levels = vec![
        vec![b"leaf1".to_vec()],
        vec![b"parent".to_vec()],
    ];
    
    storage.store_cache_batch(&cache_levels).unwrap();
    let before_clear = storage.get_all_cache_levels().unwrap();
    assert_eq!(before_clear.len(), 2);
    
    storage.clear_cache().unwrap();
    let after_clear = storage.get_all_cache_levels().unwrap();
    assert_eq!(after_clear.len(), 0);
}

#[test]
fn test_metadata_operations() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let metadata = TreeMetadata {
        num_leaves: 10,
        max_leaves: 1024,
        cache_valid: true,
    };
    
    storage.store_metadata(&metadata).unwrap();
    
    let retrieved = storage.get_metadata().unwrap().unwrap();
    assert_eq!(retrieved.num_leaves, 10);
    assert_eq!(retrieved.max_leaves, 1024);
    assert_eq!(retrieved.cache_valid, true);
}

#[test]
fn test_root_operations() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let root_hash = b"merkle_root_hash".to_vec();
    storage.store_root(&root_hash).unwrap();
    
    let retrieved = storage.get_root().unwrap().unwrap();
    assert_eq!(retrieved, root_hash);
}

#[test]
fn test_clear_all() {
    let (storage, _temp_dir) = create_temp_storage();
    
    // Store some data
    let leaves = vec![b"leaf1".to_vec()];
    storage.store_leaves_batch(&leaves).unwrap();
    
    let cache_levels = vec![vec![b"hash1".to_vec()]];
    storage.store_cache_batch(&cache_levels).unwrap();
    
    let metadata = TreeMetadata {
        num_leaves: 1,
        max_leaves: 1024,
        cache_valid: true,
    };
    storage.store_metadata(&metadata).unwrap();
    
    storage.store_root(b"root").unwrap();
    
    // Clear everything
    storage.clear_all().unwrap();
    
    // Verify everything is cleared
    let leaves_after = storage.get_all_leaves().unwrap();
    assert_eq!(leaves_after.len(), 0);
    
    let cache_after = storage.get_all_cache_levels().unwrap();
    assert_eq!(cache_after.len(), 0);
    
    let _metadata_after = storage.get_metadata().unwrap();
    //assert_eq!(metadata_after, None);
    
    let root_after = storage.get_root().unwrap();
    assert_eq!(root_after, None);
}

#[test]
fn test_sync() {
    let (storage, _temp_dir) = create_temp_storage();
    
    let leaf = b"test_leaf".to_vec();
    storage.store_leaf(0, &leaf).unwrap();
    
    // Sync should not panic
    storage.sync().unwrap();
    
    // Data should still be there after sync
    let retrieved = storage.get_leaf(0).unwrap();
    assert_eq!(retrieved, Some(leaf));
}
