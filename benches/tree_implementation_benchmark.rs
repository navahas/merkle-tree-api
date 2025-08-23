use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use merkle_tree_api::lmdb_tree::LmdbMerkleTree;
use merkle_tree_api::merkle_tree::IncrementalMerkleTree;
use merkle_tree_api::sparse_merkle_tree::SparseMerkleTree;
use rand::{Rng, random};
use std::hint::black_box;
use tempfile::TempDir;

fn generate_test_data(size: usize) -> Vec<Vec<u8>> {
    (0..size)
        .map(|i| format!("test_leaf_{:08x}", i as u32 ^ random::<u32>()).into_bytes())
        .collect()
}

fn setup_heap_tree_with_data(size: usize) -> IncrementalMerkleTree {
    let mut tree = IncrementalMerkleTree::new();
    let leaves = generate_test_data(size);
    tree.add_leaves(leaves).unwrap();
    tree
}

fn setup_lmdb_tree_with_data(size: usize) -> (LmdbMerkleTree, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let tree = LmdbMerkleTree::new(db_path.to_str().unwrap()).unwrap();
    let leaves = generate_test_data(size);
    tree.add_leaves(leaves).unwrap();
    (tree, temp_dir)
}

fn setup_sparse_tree_with_data(size: usize) -> (SparseMerkleTree, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("sparse_bench.db");
    let mut tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
    let leaves = generate_test_data(size);
    for (i, leaf) in leaves.into_iter().enumerate() {
        tree.add_leaf(i as u64, leaf).unwrap();
    }
    (tree, temp_dir)
}

fn bench_write_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Write Operations");
    group.sample_size(10); // Reduce samples for faster benchmarking

    // Single leaf insertion
    for &size in &[1, 5, 10, 25] {
        group.bench_with_input(
            BenchmarkId::new("heap_add_single_leaf", size),
            &size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        let mut tree = IncrementalMerkleTree::new();
                        if size > 1 {
                            let initial_leaves = generate_test_data(size - 1);
                            tree.add_leaves(initial_leaves).unwrap();
                        }
                        tree
                    },
                    |mut tree| {
                        let leaf = format!("new_leaf_{:08x}", random::<u32>()).into_bytes();
                        tree.add_leaf(black_box(leaf)).unwrap();
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lmdb_add_single_leaf", size),
            &size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let tree = LmdbMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        if size > 1 {
                            let initial_leaves = generate_test_data(size - 1);
                            tree.add_leaves(initial_leaves).unwrap();
                        }
                        (tree, temp_dir)
                    },
                    |(tree, _temp_dir)| {
                        let leaf = format!("new_leaf_{:08x}", random::<u32>()).into_bytes();
                        tree.add_leaf(black_box(leaf)).unwrap();
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sparse_add_single_leaf", size),
            &size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("sparse_bench.db");
                        let mut tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        if size > 1 {
                            let initial_leaves = generate_test_data(size - 1);
                            for (i, leaf) in initial_leaves.into_iter().enumerate() {
                                tree.add_leaf(i as u64, leaf).unwrap();
                            }
                        }
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let leaf = format!("new_leaf_{:08x}", random::<u32>()).into_bytes();
                        tree.add_leaf(black_box(size as u64), black_box(leaf)).unwrap();
                    },
                );
            },
        );
    }

    // Batch leaf insertion
    for &batch_size in &[5, 10, 25, 50] {
        group.bench_with_input(
            BenchmarkId::new("heap_add_batch_leaves", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_with_setup(
                    || IncrementalMerkleTree::new(),
                    |mut tree| {
                        let leaves = generate_test_data(batch_size);
                        tree.add_leaves(black_box(leaves)).unwrap();
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lmdb_add_batch_leaves", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let tree = LmdbMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(tree, _temp_dir)| {
                        let leaves = generate_test_data(batch_size);
                        tree.add_leaves(black_box(leaves)).unwrap();
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sparse_add_batch_leaves", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("sparse_bench.db");
                        let tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let leaves = generate_test_data(batch_size);
                        for (i, leaf) in leaves.into_iter().enumerate() {
                            tree.add_leaf(black_box(i as u64), black_box(leaf)).unwrap();
                        }
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_read_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Read Operations");
    group.sample_size(10);

    for &tree_size in &[10, 50, 100] {
        // Get root benchmarks
        group.bench_with_input(
            BenchmarkId::new("heap_get_root", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || setup_heap_tree_with_data(tree_size),
                    |mut tree| {
                        black_box(tree.root());
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lmdb_get_root", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || setup_lmdb_tree_with_data(tree_size),
                    |(tree, _temp_dir)| {
                        black_box(tree.root());
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sparse_get_root", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || setup_sparse_tree_with_data(tree_size),
                    |(tree, _temp_dir)| {
                        black_box(tree.get_root());
                    },
                );
            },
        );

        // Get number of leaves
        group.bench_with_input(
            BenchmarkId::new("heap_num_leaves", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || setup_heap_tree_with_data(tree_size),
                    |tree| {
                        black_box(tree.num_leaves());
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lmdb_num_leaves", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || setup_lmdb_tree_with_data(tree_size),
                    |(tree, _temp_dir)| {
                        black_box(tree.num_leaves());
                    },
                );
            },
        );

        // Get proof benchmarks
        if tree_size >= 10 {
            // Only test proof generation for trees with reasonable size
            group.bench_with_input(
                BenchmarkId::new("heap_get_proof", tree_size),
                &tree_size,
                |b, &tree_size| {
                    b.iter_with_setup(
                        || setup_heap_tree_with_data(tree_size),
                        |tree| {
                            let mut rng = rand::rng();
                            let index = rng.random_range(0..tree_size);
                            black_box(tree.get_proof(black_box(index)));
                        },
                    );
                },
            );

            group.bench_with_input(
                BenchmarkId::new("lmdb_get_proof", tree_size),
                &tree_size,
                |b, &tree_size| {
                    b.iter_with_setup(
                        || setup_lmdb_tree_with_data(tree_size),
                        |(tree, _temp_dir)| {
                            let mut rng = rand::rng();
                            let index = rng.random_range(0..tree_size);
                            black_box(tree.get_proof(black_box(index)));
                        },
                    );
                },
            );

            group.bench_with_input(
                BenchmarkId::new("sparse_get_proof", tree_size),
                &tree_size,
                |b, &tree_size| {
                    b.iter_with_setup(
                        || setup_sparse_tree_with_data(tree_size),
                        |(tree, _temp_dir)| {
                            let mut rng = rand::rng();
                            let index = rng.random_range(0..tree_size as u64);
                            black_box(tree.get_proof(black_box(index)));
                        },
                    );
                },
            );
        }
    }

    group.finish();
}

fn bench_proof_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof Verification");
    group.sample_size(10);

    for &tree_size in &[10, 50, 100] {
        // Heap implementation proof verification
        group.bench_with_input(
            BenchmarkId::new("heap_verify_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let mut tree = setup_heap_tree_with_data(tree_size);
                        let leaves = generate_test_data(tree_size);
                        let mut rng = rand::rng();
                        let index = rng.random_range(0..tree_size);
                        let leaf = &leaves[index];
                        let proof = tree.get_proof(index).unwrap();
                        let root = tree.root().unwrap();
                        (tree, leaf.clone(), proof, root, index)
                    },
                    |(tree, leaf, proof, root, index)| {
                        black_box(tree.verify_proof(
                            black_box(&leaf),
                            black_box(&proof),
                            black_box(&root),
                            black_box(index),
                        ));
                    },
                );
            },
        );

        // LMDB implementation proof verification
        group.bench_with_input(
            BenchmarkId::new("lmdb_verify_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let (tree, temp_dir) = setup_lmdb_tree_with_data(tree_size);
                        let leaves = generate_test_data(tree_size);
                        let mut rng = rand::rng();
                        let index = rng.random_range(0..tree_size);
                        let leaf = &leaves[index];
                        let proof = tree.get_proof(index).unwrap();
                        let root = tree.root().unwrap();
                        (tree, temp_dir, leaf.clone(), proof, root, index)
                    },
                    |(tree, _temp_dir, leaf, proof, root, index)| {
                        black_box(tree.verify_proof(
                            black_box(&leaf),
                            black_box(&proof),
                            black_box(&root),
                            black_box(index),
                        ));
                    },
                );
            },
        );

        // Sparse implementation proof verification
        group.bench_with_input(
            BenchmarkId::new("sparse_verify_proof", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let (tree, temp_dir) = setup_sparse_tree_with_data(tree_size);
                        let leaves = generate_test_data(tree_size);
                        let mut rng = rand::rng();
                        let index = rng.random_range(0..tree_size as u64);
                        let leaf = &leaves[index as usize];
                        let proof = tree.get_proof(index).unwrap();
                        let root = tree.get_root().unwrap();
                        (tree, temp_dir, leaf.clone(), proof, root, index)
                    },
                    |(tree, _temp_dir, leaf, proof, root, index)| {
                        black_box(tree.verify_proof(
                            black_box(&leaf),
                            black_box(&proof),
                            black_box(&root),
                            black_box(index),
                        ));
                    },
                );
            },
        );
    }

    group.finish();
}

fn bench_scaling_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scaling Performance");
    group.sample_size(10); // Fewer samples for large operations

    // Test how performance scales with tree size
    for &tree_size in &[100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("heap_full_build", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter(|| {
                    let mut tree = IncrementalMerkleTree::new();
                    let leaves = generate_test_data(tree_size);
                    tree.add_leaves(black_box(leaves)).unwrap();
                    black_box(tree.root());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lmdb_full_build", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let tree = LmdbMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(tree, _temp_dir)| {
                        let leaves = generate_test_data(tree_size);
                        tree.add_leaves(black_box(leaves)).unwrap();
                        black_box(tree.root());
                    },
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sparse_full_build", tree_size),
            &tree_size,
            |b, &tree_size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("sparse_bench.db");
                        let tree = SparseMerkleTree::new(db_path.to_str().unwrap()).unwrap();
                        (tree, temp_dir)
                    },
                    |(mut tree, _temp_dir)| {
                        let leaves = generate_test_data(tree_size);
                        for (i, leaf) in leaves.into_iter().enumerate() {
                            tree.add_leaf(black_box(i as u64), black_box(leaf)).unwrap();
                        }
                        black_box(tree.get_root());
                    },
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    tree_benches,
    bench_write_operations,
    bench_read_operations,
    bench_proof_verification,
    bench_scaling_performance
);
criterion_main!(tree_benches);
