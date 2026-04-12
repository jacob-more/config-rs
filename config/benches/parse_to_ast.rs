use std::{fs::read_to_string, hint::black_box, path::PathBuf, sync::LazyLock};

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_path.push("benches");
    examples_path.push("examples");
    examples_path
});
const EXAMPLE_CONFIG_FILES: &[&str] = &["cargo.lock.conf", "root_hints.conf"];

fn parse_to_ast(read_bytes: Bytes) {
    let Ok(_) = black_box(config::ast::AstTree::parse_bytes(read_bytes)) else {
        panic!("error when parsing the bytes into a tree");
    };
}

fn bench_parse_to_ast(c: &mut Criterion) {
    let mut config_path = EXAMPLES_DIRECTORY.clone();
    config_path.push("config_name");

    for file_name in EXAMPLE_CONFIG_FILES {
        let file_data = Bytes::from(
            read_to_string(config_path.with_file_name(file_name))
                .unwrap()
                .into_bytes(),
        );
        c.bench_with_input(
            BenchmarkId::new("ParseToAst", file_name),
            file_name,
            |b, _file_name| b.iter(|| parse_to_ast(black_box(file_data.clone()))),
        );
    }
}
criterion_group!(benches, bench_parse_to_ast);
criterion_main!(benches);
