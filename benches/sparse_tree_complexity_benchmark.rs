use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use merkle_tree_api::{SparseMerkleTree, OptimizedSparseMerkleTree};
use rand::{random};
use std::hint::black_box;
use std::time::Duration;
use tempfile::TempDir;

fn generate_test_leaf() -> Vec<u8> {
    format!("test_leaf_{:08x}", random::<u32>()).into_bytes()
}

fn bench_sparse_tree_scaling_individual_adds(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sparse Tree Individual Add Scaling");
    group.sample_size(10);

    for &tree_size in &[10, 50, 100, 500, 1000] {
        // Original implementation - should show O(n) due to individual DB transactions
        group.bench_with_input(
            BenchmarkId::new("original_sparse_add_to_existing", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("sparse_bench.db");
                        let mut tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        
                        // Pre-populate tree
                        for i in 0..tree_size {
                            let leaf = format!("existing_leaf_{}", i).into_bytes();
                            tree.add_leaf(i as u64, leaf).unwrap();
                        }
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let new_leaf = generate_test_leaf();
                        // Add to a new position that doesn't conflict
                        let new_index = (tree_size + 1000) as u64;
                        tree.add_leaf(black_box(new_index), black_box(new_leaf)).unwrap();
                    },
                );
            },
        );

        // Optimized implementation - should show O(log n) - constant time
        group.bench_with_input(
            BenchmarkId::new("optimized_sparse_add_to_existing", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("optimized_sparse_bench.db");
                        let mut tree = OptimizedSparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        
                        // Pre-populate tree
                        for i in 0..tree_size {
                            let leaf = format!("existing_leaf_{}", i).into_bytes();
                            tree.add_leaf(i as u64, leaf).unwrap();
                        }
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let new_leaf = generate_test_leaf();
                        // Add to a new position that doesn't conflict
                        let new_index = (tree_size + 1000) as u64;
                        tree.add_leaf(black_box(new_index), black_box(new_leaf)).unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_sparse_tree_scaling_empty_tree_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sparse Tree Empty Tree Add");
    group.sample_size(10);

    // Test adding to different positions in empty trees
    for &index in &[0, 100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("original_sparse_empty_add", index),
            &index,
            |b, &index| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("sparse_empty_bench.db");
                        let tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let leaf = generate_test_leaf();
                        tree.add_leaf(black_box(index as u64), black_box(leaf)).unwrap();
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_sparse_empty_add", index),
            &index,
            |b, &index| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("optimized_empty_bench.db");
                        let tree = OptimizedSparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let leaf = generate_test_leaf();
                        tree.add_leaf(black_box(index as u64), black_box(leaf)).unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_sparse_tree_proof_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sparse Tree Proof Operations");
    group.sample_size(10);

    for &tree_size in &[10, 100, 1000] {
        // Proof generation comparison
        group.bench_with_input(
            BenchmarkId::new("original_sparse_get_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("proof_bench.db");
                        let mut tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        
                        for i in 0..tree_size {
                            let leaf = format!("proof_leaf_{}", i).into_bytes();
                            tree.add_leaf(i as u64, leaf).unwrap();
                        }
                        (tree, temp_dir)
                    },
                    |(tree, _temp_dir)| {
                        let proof = tree.get_proof(black_box(0)).unwrap();
                        black_box(proof);
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_sparse_get_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("optimized_proof_bench.db");
                        let mut tree = OptimizedSparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        
                        for i in 0..tree_size {
                            let leaf = format!("proof_leaf_{}", i).into_bytes();
                            tree.add_leaf(i as u64, leaf).unwrap();
                        }
                        (tree, temp_dir)
                    },
                    |(tree, _temp_dir)| {
                        let proof = tree.get_proof(black_box(0)).unwrap();
                        black_box(proof);
                    },
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = sparse_complexity_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20));
    targets = bench_sparse_tree_scaling_individual_adds,
              bench_sparse_tree_scaling_empty_tree_add,
              bench_sparse_tree_proof_operations
);
criterion_main!(sparse_complexity_benches);
