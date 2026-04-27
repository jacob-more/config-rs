use rstest::rstest;

use crate::parse::lex::{
    TokenBinaryOp, TokenComment, TokenGroupingClose, TokenGroupingOpen, TokenSuffixUnaryOp,
    TokenTerminator, TokenUnknown, TokenValue, TokenWhitespace, Tokenizer,
};

#[test]
fn tokenizer_compiles() {
    // Will panic if tokenizer does not compile. If other tests were run first,
    // a lock may be poisoned, which will also result in a panic.
    let _tokenizer = Tokenizer::new();
}

#[rstest]
#[case(" ")]
#[case("\t")]
#[case("\r")]
#[case("\r\n")]
#[case(" \t\r\n")]
#[case(" \t\r\n ")]
fn tokenize_whitespace(#[case] whitespace: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(whitespace.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::Whitespace(TokenWhitespace {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case(r"a")]
#[case(r"A")]
#[case(r"ab")]
#[case(r"aB")]
#[case(r"Ab")]
#[case(r"AB")]
#[case(r"abc")]
#[case(r"ABC")]
#[case(r"0")]
#[case(r"01")]
#[case(r"012")]
#[case(r"0a")]
#[case(r"a0")]
#[case(r"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_./")]
#[case(r"_")]
#[case(r".")]
#[case(r"/")]
#[case(r#""a""#)]
#[case(r#""A""#)]
#[case(r#""ab""#)]
#[case(r#""aB""#)]
#[case(r#""Ab""#)]
#[case(r#""AB""#)]
#[case(r#""abc""#)]
#[case(r#""ABC""#)]
#[case(r#""0""#)]
#[case(r#""01""#)]
#[case(r#""012""#)]
#[case(r#""0a""#)]
#[case(r#""a0""#)]
#[case(r#""ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_./""#)]
#[case(r#""_""#)]
#[case(r#"".""#)]
#[case(r#""/""#)]
#[case(r#""-""#)]
#[case(r#"":""#)]
#[case(r#""""#)]
#[case(r#"" ""#)]
#[case(r#""space separated string""#)]
#[case(
    r#""newline
separated
string""#
)]
#[case(r"\a")]
#[case(r"\A")]
#[case(r"\a\b")]
#[case(r"\a\B")]
#[case(r"\A\b")]
#[case(r"\A\B")]
#[case(r"\a\b\c")]
#[case(r"\A\B\C")]
#[case(r"\0")]
#[case(r"\0\1")]
#[case(r"\0\1\2")]
#[case(r"\0\a")]
#[case(r"\a\0")]
#[case(r"\AB\CDEFGH\IJKLMNOPQ\RSTUVWXYZabcdefghi\\jklm\\\nopqrstuvwxyz0123456789\-\:\_\.\/")]
#[case(r"\_")]
#[case(r"\.")]
#[case(r"\&")]
#[case(r"\!")]
#[case(r"\=")]
#[case(r"\+\=")]
#[case(r"\ ")]
fn tokenize_value(#[case] value: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(value.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    let first_token = token_iter.next();
    assert!(matches!(
        &first_token,
        Some(Token::Value(TokenValue { .. }))
    ));
    assert_eq!(first_token.unwrap().as_slice(), value.as_bytes());
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case(crate::parse::OPERATOR_ASSIGN)]
#[case(crate::parse::OPERATOR_ASSIGN_IF_UNDEFINED)]
#[case(crate::parse::OPERATOR_ADD)]
#[case(crate::parse::OPERATOR_REMOVE)]
#[case(crate::parse::OPERATOR_GROUP)]
fn tokenize_binary_op(#[case] op: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(op.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::BinaryOp(TokenBinaryOp {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case(crate::parse::OPERATOR_CLEAR)]
#[case(crate::parse::OPERATOR_RESET)]
fn tokenize_suffix_unary_op(#[case] op: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(op.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::SuffixUnaryOp(TokenSuffixUnaryOp {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case("{")]
fn tokenize_group_open(#[case] open: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(open.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::GroupingOpen(TokenGroupingOpen {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case("}")]
fn tokenize_group_close(#[case] close: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(close.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::GroupingClose(TokenGroupingClose {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case(";")]
fn tokenize_terminator(#[case] terminator: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(terminator.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::Terminator(TokenTerminator {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case("#")]
#[case("#\n")]
#[case("# ")]
#[case("# \n")]
#[case("# this is some text")]
#[case("# this is some text\n")]
#[case("# this is some text wit & *() strange characters \r but not a newline")]
#[case("# this is some text wit & *() strange characters \r but not a newline\n")]
#[case("# \"a string in a comment? oh my!\"")]
#[case("# \"a string in a comment? oh my!\"\n")]
#[case("# COMMENTED_OUT=KEY_VALUE_PAIR;")]
#[case("# COMMENTED_OUT=KEY_VALUE_PAIR;\n")]
#[case("# COMMENTED_OUT: { KEY_VALUE_PAIR=IN_A_GROUP; }; ")]
#[case("# COMMENTED_OUT: { KEY_VALUE_PAIR=IN_A_GROUP; };\n")]
fn tokenize_comment(#[case] comment: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(comment.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::Comment(TokenComment {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}

#[rstest]
#[case("&")]
#[case("&*@")]
#[case("`")]
#[case("^$<>")]
fn tokenize_unknown(#[case] unknown: &'static str) {
    use bytes::Bytes;

    use crate::parse::lex::Token;

    let buffer = Bytes::from(unknown.as_bytes());
    let tokenizer = Tokenizer::new();
    let mut token_iter = tokenizer.tokenize(&buffer);
    assert_eq!(
        token_iter.next(),
        Some(Token::Unknown(TokenUnknown {
            buffer: &buffer,
            start: 0,
            end: buffer.len()
        }))
    );
    assert_eq!(token_iter.next(), None);
}
