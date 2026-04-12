use std::{
    fmt::{Display, Write},
    hint::black_box,
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use itertools::Itertools;
use rand::{
    SeedableRng,
    distr::{Distribution, SampleString},
};

fn test_ext_iter_join<T, S>(values: &[T], sep: S)
where
    T: Display,
    S: Display,
{
    use config::ext::IterJoin;

    let _ = black_box(values.join(sep).to_string());
}

// Along with the above benchmark case for the join operation, there are also a
// few implementations of the join operation intended to simulate purpose-built
// implementations of the same operation on specific known types. These are
// intended to represent more-optimized implementations written with
// consideration for the underlying types as a point of comparison.

// For simplicity, we allocate enough space to fit any i128. That means we will
// probably massively over-allocate but we should never have to re-allocate.
const MAX_I128_LEN: usize = 40;

fn test_handrolled_i128_join(values: &[i128], sep: &str) {
    fn handrolled_i128_join(values: &[i128], sep: &str) -> String {
        let capacity = (values.len() * MAX_I128_LEN) + (values.len().saturating_sub(1) * sep.len());
        let mut string = String::with_capacity(capacity);
        let mut values = values.iter();
        if let Some(first) = values.next() {
            let _ = string.write_fmt(format_args!("{first}"));
            for value in values {
                string.push_str(sep);
                let _ = string.write_fmt(format_args!("{value}"));
            }
        }
        string
    }

    let _ = black_box(handrolled_i128_join(values, sep));
}

fn test_handrolled_i128_join_small_size_optimized(values: &[i128], sep: &str) {
    fn handrolled_i128_join(values: &[i128], sep: &str) -> String {
        match values {
            &[] => String::new(),
            &[value] => value.to_string(),
            &[first, ref tail @ ..] => {
                let capacity = (values.len() * MAX_I128_LEN) + (tail.len() * sep.len());
                let mut string = String::with_capacity(capacity);
                let _ = string.write_fmt(format_args!("{first}"));
                for value in tail {
                    string.push_str(sep);
                    let _ = string.write_fmt(format_args!("{value}"));
                }
                string
            }
        }
    }

    let _ = black_box(handrolled_i128_join(values, sep));
}

fn test_handrolled_str_join(values: &[&str], sep: &str) {
    fn handrolled_str_join(values: &[&str], sep: &str) -> String {
        let capacity = values.iter().map(|s| s.len()).sum::<usize>()
            + (values.len().saturating_sub(1) * sep.len());
        let mut string = String::with_capacity(capacity);
        let mut values = values.iter();
        if let Some(first) = values.next() {
            string.push_str(first);
            for value in values {
                string.push_str(sep);
                string.push_str(value);
            }
        }
        string
    }

    let _ = black_box(handrolled_str_join(values, sep));
}

fn test_handrolled_str_join_small_size_optimized(values: &[&str], sep: &str) {
    fn handrolled_str_join(values: &[&str], sep: &str) -> String {
        match values {
            &[] => String::new(),
            &[value] => value.to_string(),
            &[first, ref tail @ ..] => {
                let capacity =
                    values.iter().map(|s| s.len()).sum::<usize>() + (tail.len() * sep.len());
                let mut string = String::with_capacity(capacity);
                string.push_str(first);
                for value in tail {
                    string.push_str(sep);
                    string.push_str(value);
                }
                string
            }
        }
    }

    let _ = black_box(handrolled_str_join(values, sep));
}

fn bench_i128_joins(c: &mut Criterion) {
    const TEST_COUNTS: &[usize] = &[0, 1, 2, 3, 5, 10, 32, 100, 1000, 10000];
    const TEST_SEPARATORS: &[&str] = &["", " ", ", "];
    // Using a portable and reproducible random number generator ensures
    // that repeated benchmarks operate on the same data.
    let rng = rand::rngs::Xoshiro256PlusPlus::seed_from_u64(0xBEEF);
    let sample = rand::distr::Uniform::new_inclusive(i128::MIN, i128::MAX)
        .unwrap()
        .sample_iter(rng)
        .take(*TEST_COUNTS.iter().max().unwrap())
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Join_i128");
    for (i, sep) in TEST_COUNTS.iter().cartesian_product(TEST_SEPARATORS) {
        let sample_data = &sample[..*i];
        let id = std::fmt::from_fn(|f| write!(f, "({i}, '{sep}')"));
        group.bench_with_input(BenchmarkId::new("HandrolledI128Join", &id), i, |b, _i| {
            b.iter(|| test_handrolled_i128_join(black_box(sample_data), black_box(*sep)))
        });
        group.bench_with_input(
            BenchmarkId::new("HandrolledI128JoinSmallLenOptimized", &id),
            i,
            |b, _i| {
                b.iter(|| {
                    test_handrolled_i128_join_small_size_optimized(
                        black_box(sample_data),
                        black_box(*sep),
                    )
                })
            },
        );
        group.bench_with_input(BenchmarkId::new("ExtIterJoin", &id), i, |b, _i| {
            b.iter(|| test_ext_iter_join(black_box(sample_data), black_box(*sep)))
        });
    }
    group.finish();
}

fn bench_str_joins(c: &mut Criterion) {
    const TEST_COUNTS: &[usize] = &[0, 1, 2, 3, 5, 10, 32, 100, 1000, 10000];
    const TEST_SEPARATORS: &[&str] = &["", " ", ", "];
    // Using a portable and reproducible random number generator ensures
    // that repeated benchmarks operate on the same data.
    let mut rng = rand::rngs::Xoshiro256PlusPlus::seed_from_u64(0xBABE);
    let lengths = rand::distr::Uniform::new_inclusive(0, 2048)
        .unwrap()
        .sample_iter(&mut rng)
        .take(*TEST_COUNTS.iter().max().unwrap())
        .collect::<Vec<_>>();
    let strings = lengths
        .into_iter()
        .map(|len| rand::distr::StandardUniform.sample_string(&mut rng, len))
        .collect::<Vec<_>>();
    let strs = strings.iter().map(|s| s.as_str()).collect::<Vec<_>>();

    let mut group = c.benchmark_group("Join_str");
    for (i, sep) in TEST_COUNTS.iter().cartesian_product(TEST_SEPARATORS) {
        let sample_data = &strs[..*i];
        let id = std::fmt::from_fn(|f| write!(f, "({i}, '{sep}')"));
        group.bench_with_input(BenchmarkId::new("HandrolledStrJoin", &id), i, |b, _i| {
            b.iter(|| test_handrolled_str_join(black_box(sample_data), black_box(*sep)))
        });
        group.bench_with_input(
            BenchmarkId::new("HandrolledStrJoinSmallLenOptimized", &id),
            i,
            |b, _i| {
                b.iter(|| {
                    test_handrolled_str_join_small_size_optimized(
                        black_box(sample_data),
                        black_box(*sep),
                    )
                })
            },
        );
        group.bench_with_input(BenchmarkId::new("ExtIterJoin", &id), i, |b, _i| {
            b.iter(|| test_handrolled_str_join(black_box(sample_data), black_box(*sep)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_str_joins, bench_i128_joins);
criterion_main!(benches);
