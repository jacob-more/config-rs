use std::{
    fs::File,
    hint::{black_box, cold_path},
    io::Read,
    path::PathBuf,
    sync::LazyLock,
};

use bytes::Bytes;
use config::syn::SyntaxParser;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_path.push("benches");
    examples_path.push("examples");
    examples_path
});
const EXAMPLE_CONFIG_FILES: &[&str] = &["cargo.lock.conf", "root_hints.conf"];

fn parse_to_syn(parser: &SyntaxParser, read_bytes: Bytes) {
    let Ok(_) = black_box(parser.parse(&read_bytes)) else {
        cold_path();
        panic!("error when parsing the bytes syntax");
    };
}

fn bench_parse_to_syn(c: &mut Criterion) {
    let mut config_path = EXAMPLES_DIRECTORY.clone();
    config_path.push("config_name");

    for file_name in EXAMPLE_CONFIG_FILES {
        let mut file_data = Vec::new();
        File::open(config_path.with_file_name(file_name))
            .unwrap()
            .read_to_end(&mut file_data)
            .unwrap();
        let file_data = Bytes::from(file_data);
        let parser = SyntaxParser::new();
        c.bench_with_input(
            BenchmarkId::new("ParseToSyn", file_name),
            file_name,
            |b, _file_name| b.iter(|| parse_to_syn(&parser, black_box(file_data.clone()))),
        );
    }
}
criterion_group!(benches, bench_parse_to_syn);
criterion_main!(benches);
