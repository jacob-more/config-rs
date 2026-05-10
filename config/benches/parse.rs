use std::{
    fs::File,
    hint::{black_box, cold_path},
    io::Read,
    path::PathBuf,
    sync::LazyLock,
};

use bytes::Bytes;
use config::parse::__private::{AstParser, SyntaxParser, Tokenizer};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_path.push("benches");
    examples_path.push("examples");
    examples_path
});

fn bench_cases() -> impl Iterator<Item = (String, Bytes)> {
    walkdir::WalkDir::new(&*EXAMPLES_DIRECTORY)
        .follow_links(false)
        .same_file_system(true)
        .sort_by_file_name()
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let mut buffer = Vec::new();
            File::open(entry.path())
                .unwrap()
                .read_to_end(&mut buffer)
                .unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                Bytes::from(buffer),
            )
        })
}

fn parse_to_lex(tokenizer: &Tokenizer, read_bytes: &Bytes) {
    tokenizer
        .tokenize(read_bytes)
        .for_each(|token| drop(black_box(token)));
}
fn bench_parse_to_lex(c: &mut Criterion) {
    for (name, data) in bench_cases() {
        c.bench_with_input(
            BenchmarkId::new("FreshParseToLex", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_lex(&Tokenizer::new(), black_box(&data))),
        );
    }
    for (name, data) in bench_cases() {
        let tokenizer = Tokenizer::new();
        // The first run through will include time associated with lazy-regex
        // compilation, which we only want to see in the Fresh* benchmarks.
        parse_to_lex(&tokenizer, &data);
        c.bench_with_input(
            BenchmarkId::new("ParseToLex", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_lex(&tokenizer, black_box(&data))),
        );
    }
}

fn parse_to_syn(parser: &SyntaxParser, read_bytes: &Bytes) {
    let Ok(_) = black_box(parser.parse(&read_bytes)) else {
        cold_path();
        panic!("error when parsing the bytes syntax");
    };
}
fn bench_parse_to_syn(c: &mut Criterion) {
    for (name, data) in bench_cases() {
        c.bench_with_input(
            BenchmarkId::new("FreshParseToSyn", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_syn(&SyntaxParser::new(), black_box(&data))),
        );
    }
    for (name, data) in bench_cases() {
        let parser = SyntaxParser::new();
        // The first run through will include time associated with lazy-regex
        // compilation, which we only want to see in the Fresh* benchmarks.
        parse_to_syn(&parser, &data);
        c.bench_with_input(
            BenchmarkId::new("ParseToSyn", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_syn(&parser, black_box(&data))),
        );
    }
}

fn parse_to_ast(ast_parser: &AstParser, read_bytes: Bytes) {
    let Ok(_) = black_box(ast_parser.parse(read_bytes)) else {
        cold_path();
        panic!("error when parsing the bytes into a tree");
    };
}
fn bench_parse_to_ast(c: &mut Criterion) {
    for (name, data) in bench_cases() {
        c.bench_with_input(
            BenchmarkId::new("FreshParseToAst", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_ast(&AstParser::compile(), black_box(data.clone()))),
        );
    }
    for (name, data) in bench_cases() {
        let parser = AstParser::new();
        // The first run through will include time associated with lazy-regex
        // compilation, which we only want to see in the Fresh* benchmarks.
        parse_to_ast(&parser, data.clone());
        c.bench_with_input(
            BenchmarkId::new("ParseToAst", &name),
            &name,
            |b, _file_name| b.iter(|| parse_to_ast(&parser, black_box(data.clone()))),
        );
    }
}

fn bench_compile_lex_tokenizer(c: &mut Criterion) {
    c.bench_function("CompileLexTokenizer", |bencher| {
        bencher.iter(|| drop(black_box(Tokenizer::compile())));
    });
}

criterion_group!(
    benches,
    bench_compile_lex_tokenizer,
    bench_parse_to_lex,
    bench_parse_to_syn,
    bench_parse_to_ast,
);
criterion_main!(benches);
