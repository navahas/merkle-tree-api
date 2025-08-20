use merkle_tree_api::merkle_tree::IncrementalMerkleTree;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_concurrent_add_leaves_and_get_root() {
    let tree = Arc::new(RwLock::new(IncrementalMerkleTree::new()));

    // Prepare test data
    let leaves1: Vec<Vec<u8>> = (0..100)
        .map(|i| format!("leaf{}", i).into_bytes())
        .collect();
    let leaves2: Vec<Vec<u8>> = (100..200)
        .map(|i| format!("leaf{}", i).into_bytes())
        .collect();

    let tree1 = Arc::clone(&tree);
    let tree2 = Arc::clone(&tree);
    let tree3 = Arc::clone(&tree);

    // Task 1: Add first batch of leaves
    let task1 = tokio::spawn(async move {
        let mut t = tree1.write().await;
        t.add_leaves(leaves1).unwrap();
    });

    // Task 2: Add second batch of leaves
    let task2 = tokio::spawn(async move {
        let mut t = tree2.write().await;
        t.add_leaves(leaves2).unwrap();
    });

    // Task 3: Repeatedly try to get root (may fail during updates)
    let task3 = tokio::spawn(async move {
        for _ in 0..10 {
            let mut t = tree3.write().await;
            let _root = t.root();
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    });

    // Wait for all tasks
    let _ = tokio::try_join!(task1, task2, task3);

    // Verify final state
    let mut final_tree = tree.write().await;
    assert_eq!(final_tree.num_leaves(), 200);
    assert!(final_tree.root().is_some());
}

#[tokio::test]
async fn test_concurrent_proof_generation() {
    let tree = Arc::new(RwLock::new(IncrementalMerkleTree::new()));

    // Add initial leaves
    {
        let mut t = tree.write().await;
        let leaves: Vec<Vec<u8>> = (0..50).map(|i| format!("leaf{}", i).into_bytes()).collect();
        t.add_leaves(leaves).unwrap();
    }

    let mut handles = vec![];

    // Spawn multiple tasks to generate proofs concurrently
    for i in 0..10 {
        let tree_clone = Arc::clone(&tree);
        let handle = tokio::spawn(async move {
            let t = tree_clone.read().await;
            let proof = t.get_proof(i % 50);
            proof.is_some()
        });
        handles.push(handle);
    }

    // Wait for all tasks and verify they succeeded
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result);
    }
}