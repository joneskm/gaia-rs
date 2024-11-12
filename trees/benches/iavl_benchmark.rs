#![allow(missing_docs)]

#[cfg(feature = "bench")]
use bench::*;
#[cfg(feature = "bench")]
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(feature = "bench")]
use extensions::testing::UnwrapCorrupt;
#[cfg(feature = "bench")]
use pprof::criterion::{Output, PProfProfiler};

#[cfg(feature = "bench")]
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100).with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = iavl_benchmark
}

#[cfg(feature = "bench")]
criterion_main!(benches);

#[cfg(not(feature = "bench"))]
fn main() {}

#[cfg(feature = "bench")]
#[allow(dead_code)]
mod bench {
    use std::{fmt, fs, time::Instant};

    use criterion::{black_box, BenchmarkId, Criterion};
    use database::rocks::RocksDB;
    use rand::{distributions::Standard, Rng};
    use trees::iavl::Tree;

    const DB_DIR: &str = "db";

    #[derive(Debug)]
    struct Params {
        _name: &'static str, // used in Display impl
        init_size: usize,
        block_size: u64,
        key_length: usize,
        data_length: usize,
    }

    impl fmt::Display for Params {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    /// Queries random keys against saved state. Keys are almost certainly not in the tree.
    fn iavl_query_miss_benchmark(c: &mut Criterion, all_params: &Vec<Params>) {
        let mut group = c.benchmark_group("query-miss");
        for params in all_params {
            let (tree, _) = prepare_tree(params);
            group.bench_with_input(BenchmarkId::from_parameter(params), &params, |b, params| {
                b.iter(|| {
                    let key: Vec<u8> = rand::thread_rng()
                        .sample_iter(Standard)
                        .take(params.key_length)
                        .collect();
                    tree.get(black_box(&key));
                })
            });
        }
        group.finish();
    }

    /// Queries keys that are known to be in the tree
    fn iavl_query_hits_benchmark(c: &mut Criterion, all_params: &Vec<Params>) {
        let mut group = c.benchmark_group("query-hits");
        for params in all_params {
            let (tree, keys) = prepare_tree(params);
            group.bench_with_input(BenchmarkId::from_parameter(params), &params, |b, params| {
                b.iter(|| {
                    let key: &Vec<u8> = keys
                        .get(rand::thread_rng().gen_range(0..params.init_size))
                        .unwrap_test();
                    tree.get(black_box(key));
                })
            });
        }
        group.finish();
    }

    fn iavl_update_benchmark(c: &mut Criterion, all_params: &Vec<Params>) {
        let mut group = c.benchmark_group("iavl-update");
        for params in all_params {
            let (mut tree, keys) = prepare_tree(params);
            group.bench_with_input(BenchmarkId::from_parameter(params), &params, |b, params| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    for i in 0..iters {
                        let key: &Vec<u8> = keys
                            .get(rand::thread_rng().gen_range(0..params.init_size))
                            .unwrap_test();

                        let data: Vec<u8> = rand::thread_rng()
                            .sample_iter(Standard)
                            .take(params.data_length)
                            .collect();

                        tree.set(black_box(key.clone()), black_box(data.to_vec()));

                        if i % params.block_size == 0 {
                            commit_tree(&mut tree)
                        }
                    }
                    start.elapsed()
                })
            });
        }
        group.finish();
    }

    fn iavl_range_benchmark(c: &mut Criterion, all_params: &Vec<Params>) {
        let mut group = c.benchmark_group("iavl-range");
        for params in all_params {
            let (tree, _) = prepare_tree(params);
            group.bench_with_input(BenchmarkId::from_parameter(params), &params, |b, _| {
                b.iter(|| {
                    let _range: Vec<(Vec<u8>, Vec<u8>)> = tree.range(..).collect();
                })
            });
        }
        group.finish();
    }

    fn iavl_run_blocks_benchmark(c: &mut Criterion, all_params: &Vec<Params>) {
        let mut group = c.benchmark_group("iavl-run-blocks");
        for params in all_params {
            let (mut tree, keys) = prepare_tree(params);
            group.bench_with_input(BenchmarkId::from_parameter(params), &params, |b, params| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    for i in 0..iters {
                        for _ in 0..params.block_size {
                            // 50% insert, 50% update
                            let key = if i % 2 == 0 {
                                keys.get(rand::thread_rng().gen_range(0..params.init_size))
                                    .unwrap_test()
                                    .clone()
                            } else {
                                rand::thread_rng()
                                    .sample_iter(Standard)
                                    .take(params.key_length)
                                    .collect()
                            };

                            let data: Vec<u8> = rand::thread_rng()
                                .sample_iter(Standard)
                                .take(params.data_length)
                                .collect();

                            tree.get(&key);
                            tree.set(key, data)
                        }

                        commit_tree(&mut tree);
                    }
                    start.elapsed()
                })
            });
        }
        group.finish();
    }

    pub fn iavl_benchmark(c: &mut Criterion) {
        let all_params = vec![
            Params {
                _name: "small",
                init_size: 1000,
                block_size: 100,
                key_length: 4,
                data_length: 10,
            },
            Params {
                _name: "medium",
                init_size: 100_000,
                block_size: 100,
                key_length: 16,
                data_length: 40,
            },
            Params {
                _name: "large",
                init_size: 1_000_000,
                block_size: 100,
                key_length: 16,
                data_length: 40,
            },
        ];

        iavl_query_miss_benchmark(c, &all_params);
        iavl_query_hits_benchmark(c, &all_params);
        iavl_range_benchmark(c, &all_params);
        iavl_update_benchmark(c, &all_params);
        iavl_run_blocks_benchmark(c, &all_params);
    }

    /// Attempts to exactly replicate steps in go IAVL, see https://github.com/cosmos/iavl/blob/7f698ba3fa232c54109e5b4ea42562bbecdb1bf8/benchmarks/bench_test.go#L41-L57
    fn commit_tree(tree: &mut Tree<RocksDB>) {
        let (_, _version) = tree.save_version().unwrap_test();

        // TODO: need to implement this once delete_version has been implemented
        // if version > historySize {
        // 	err = t.DeleteVersion(version - historySize)
        // 	if err != nil {
        // 		b.Errorf("Can't delete: %v", err)
        // 	}
        // }
    }

    fn prepare_tree(params: &Params) -> (Tree<RocksDB>, Vec<Vec<u8>>) {
        // remove previous test DBs
        fs::remove_dir_all(DB_DIR).unwrap_test();
        fs::create_dir(DB_DIR).unwrap_test();

        let db = RocksDB::new(DB_DIR).unwrap_test();
        let mut tree = Tree::new(db, None, params.init_size.try_into().unwrap_test()).unwrap_test();
        let mut keys = Vec::with_capacity(params.init_size);

        for _ in 0..params.init_size {
            let key: Vec<u8> = rand::thread_rng()
                .sample_iter(Standard)
                .take(params.key_length)
                .collect();

            let data: Vec<u8> = rand::thread_rng()
                .sample_iter(Standard)
                .take(params.data_length)
                .collect();

            tree.set(key.clone(), data);
            keys.push(key);
        }

        tree.save_version().unwrap_test();

        (tree, keys)
    }
}
