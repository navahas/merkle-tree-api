use merkle_tree_api::{IncrementalMerkleTree, SparseMerkleTree};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Sparse Merkle Tree Test\n");

    let mut sparse_tree = SparseMerkleTree::new("sparse_test.db")?;

    let test_values = vec![
        b"value1".to_vec(),
        b"value2".to_vec(),
        b"value3".to_vec(),
        b"value4".to_vec(),
    ];

    let indices = [0u64, 100u64, 1000u64, 10000u64];

    println!("=== Sparse Tree Operations ===");

    let start = Instant::now();
    for (i, (index, value)) in indices.iter().zip(test_values.iter()).enumerate() {
        sparse_tree.add_leaf(*index, value.clone())?;
        println!("Added leaf {} at index {}", i + 1, index);
    }
    let sparse_add_time = start.elapsed();
    println!("Sparse add time: {:?}", sparse_add_time);

    let root = sparse_tree.get_root()?;
    println!("Root: {}", hex::encode(&root));

    let proof = sparse_tree.get_proof(indices[0])?;
    println!(
        "Proof for index {}: {} siblings",
        indices[0],
        proof.siblings.len()
    );

    let is_valid = sparse_tree.verify_proof(&test_values[0], &proof, &root, indices[0]);
    println!("Proof verification: {}", is_valid);

    println!("\n=== Incremental Tree Comparison ===");

    let mut incremental_tree = IncrementalMerkleTree::new();
    let start = Instant::now();
    for value in &test_values {
        incremental_tree.add_leaf(value.clone())?;
    }
    let incremental_add_time = start.elapsed();
    println!("Incremental add time: {:?}", incremental_add_time);

    if let Some(inc_root) = incremental_tree.root() {
        println!("Incremental root: {}", hex::encode(&inc_root));
    }

    println!("\nPerformance comparison:");
    println!("Sparse: {:?} (O(log n) per operation)", sparse_add_time);
    println!(
        "Incremental: {:?} (O(n) per operation)",
        incremental_add_time
    );

    Ok(())
}
