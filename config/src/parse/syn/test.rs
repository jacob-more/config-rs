use crate::parse::syn::SyntaxParser;

#[test]
fn parser_compiles() {
    let _ = SyntaxParser::compile();
    // Will panic if parser does not compile. If other tests were run first, a
    // lock may be poisoned, which will also result in a panic. The call to
    // `compile()` should test the exact same thing, but this just double-checks
    // that there is not some unexpected issue with one and not the other.
    let _ = SyntaxParser::new();
}
