use std::{fs::File, hint::black_box, io::Read, path::PathBuf, sync::LazyLock};

use bytes::Bytes;
use config::lex::{CONFIG_LEXICAL_TOKENIZER, Tokenizer};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_path.push("benches");
    examples_path.push("examples");
    examples_path
});
const EXAMPLE_CONFIG_FILES: &[&str] = &["cargo.lock.conf", "root_hints.conf"];

fn parse_to_lex(tokenizer: &Tokenizer, read_bytes: Bytes) {
    tokenizer
        .tokenize(&read_bytes)
        .for_each(|token| drop(black_box(token)));
}

fn bench_parse_to_lex(c: &mut Criterion) {
    let mut config_path = EXAMPLES_DIRECTORY.clone();
    config_path.push("config_name");

    for file_name in EXAMPLE_CONFIG_FILES {
        let mut file_data = Vec::new();
        File::open(config_path.with_file_name(file_name))
            .unwrap()
            .read_to_end(&mut file_data)
            .unwrap();
        let file_data = Bytes::from(file_data);
        let tokenizer = CONFIG_LEXICAL_TOKENIZER.clone();
        c.bench_with_input(
            BenchmarkId::new("ParseToLex", file_name),
            file_name,
            |b, _file_name| b.iter(|| parse_to_lex(&tokenizer, black_box(file_data.clone()))),
        );
    }
}
criterion_group!(benches, bench_parse_to_lex);
criterion_main!(benches);
