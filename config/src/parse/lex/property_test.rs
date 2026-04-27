use std::hint::black_box;

use bytes::Bytes;
use proptest::prelude::*;
use proptest_derive::Arbitrary;

use crate::parse::lex::{Token, Tokenizer};

trait AsBytes {
    fn bytes(&self) -> impl Iterator<Item = u8>;
}
trait AsToken: AsBytes {
    fn as_input(&self) -> Bytes {
        Vec::from_iter(self.bytes()).into()
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropWhitespace {
    #[proptest(regex = r"(?-u:\s|\r|\n)+")]
    val: Vec<u8>,
}
impl AsBytes for PropWhitespace {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.val.iter().copied()
    }
}
impl AsToken for PropWhitespace {}
impl std::fmt::Debug for PropWhitespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropWhitespace({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
enum PropValue {
    Quoted(#[proptest(regex = r##""(?<qstring>(?s-u)[^"\\]*)""##)] Vec<u8>),
    QuotedEscaped(#[proptest(regex = r##""(?<qestring>(?s-u)(?:[^"\\]|\\.)*)""##)] Vec<u8>),
    Raw(
        #[proptest(regex = r"(?<string>(?u-s)[A-Za-z0-9_./](?:[A-Za-z0-9_./\-]*[A-Za-z0-9_./])?)")]
        Vec<u8>,
    ),
    RawEscaped(
        #[proptest(
            regex = r"(?<estring>(?s-u)(?:[A-Za-z0-9_./]|\\.)(?:(?:[A-Za-z0-9_./\-]|\\.)*(?:[A-Za-z0-9_./]|\\.))?)"
        )]
        Vec<u8>,
    ),
}
impl AsBytes for PropValue {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        match self {
            Self::Quoted(items) => items,
            Self::QuotedEscaped(items) => items,
            Self::Raw(items) => items,
            Self::RawEscaped(items) => items,
        }
        .iter()
        .copied()
    }
}
impl AsToken for PropValue {}
impl std::fmt::Debug for PropValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropValue::")?;
        match self {
            Self::Quoted(_) => write!(f, "Quoted")?,
            Self::QuotedEscaped(_) => write!(f, "QuotedEscaped")?,
            Self::Raw(_) => write!(f, "Raw")?,
            Self::RawEscaped(_) => write!(f, "RawEscaped")?,
        }
        write!(f, "({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropBinaryOp {
    #[proptest(regex = r"=|:=|\+=|-=|:")]
    val: Vec<u8>,
}
impl AsBytes for PropBinaryOp {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.val.iter().copied()
    }
}
impl AsToken for PropBinaryOp {}
impl std::fmt::Debug for PropBinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropBinaryOp({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropSuffixUnaryOp {
    #[proptest(regex = r"!!|!")]
    val: Vec<u8>,
}
impl AsBytes for PropSuffixUnaryOp {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.val.iter().copied()
    }
}
impl AsToken for PropSuffixUnaryOp {}
impl std::fmt::Debug for PropSuffixUnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropSuffixUnaryOp({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropGroupingOpen;
impl AsBytes for PropGroupingOpen {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        "{".as_bytes().iter().copied()
    }
}
impl AsToken for PropGroupingOpen {}
impl std::fmt::Debug for PropGroupingOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropGroupingOpen({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropGroupingClose;
impl AsBytes for PropGroupingClose {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        "}".as_bytes().iter().copied()
    }
}
impl AsToken for PropGroupingClose {}
impl std::fmt::Debug for PropGroupingClose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropGroupingClose({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropTerminator;
impl AsBytes for PropTerminator {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        ";".as_bytes().iter().copied()
    }
}
impl AsToken for PropTerminator {}
impl std::fmt::Debug for PropTerminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropTerminator({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
struct PropComment {
    #[proptest(regex = r"(?-su:#.*\n)")]
    val: Vec<u8>,
}
impl AsBytes for PropComment {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.val.iter().copied()
    }
}
impl AsToken for PropComment {}
impl std::fmt::Debug for PropComment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropComment({:?})", self.as_input())
    }
}

#[derive(Arbitrary, Clone, PartialEq, Eq, Hash)]
enum PropToken {
    Whitespace(PropWhitespace),
    Value(PropValue),
    BinaryOp(PropBinaryOp),
    SuffixUnaryOp(PropSuffixUnaryOp),
    GroupingOpen(PropGroupingOpen),
    GroupingClose(PropGroupingClose),
    Terminator(PropTerminator),
    Comment(PropComment),
}
impl AsBytes for PropToken {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        let bytes: Box<dyn Iterator<Item = u8>> = match self {
            Self::Whitespace(token) => Box::new(token.bytes()),
            Self::Value(token) => Box::new(token.bytes()),
            Self::BinaryOp(token) => Box::new(token.bytes()),
            Self::SuffixUnaryOp(token) => Box::new(token.bytes()),
            Self::GroupingOpen(token) => Box::new(token.bytes()),
            Self::GroupingClose(token) => Box::new(token.bytes()),
            Self::Terminator(token) => Box::new(token.bytes()),
            Self::Comment(token) => Box::new(token.bytes()),
        };
        bytes
    }
}
impl AsToken for PropToken {}
impl PropToken {
    fn matches(&self, other: &Token) -> bool {
        match (self, other) {
            (Self::Whitespace(a), Token::Whitespace(b)) => a.as_input() == b.as_bytes(),
            (Self::Value(a), Token::Value(b)) => a.as_input() == b.as_bytes(),
            (Self::BinaryOp(a), Token::BinaryOp(b)) => a.as_input() == b.as_bytes(),
            (Self::SuffixUnaryOp(a), Token::SuffixUnaryOp(b)) => a.as_input() == b.as_bytes(),
            (Self::GroupingOpen(a), Token::GroupingOpen(b)) => a.as_input() == b.as_bytes(),
            (Self::GroupingClose(a), Token::GroupingClose(b)) => a.as_input() == b.as_bytes(),
            (Self::Terminator(a), Token::Terminator(b)) => a.as_input() == b.as_bytes(),
            (Self::Comment(a), Token::Comment(b)) => a.as_input() == b.as_bytes(),
            (_, _) => false,
        }
    }
}
impl std::fmt::Debug for PropToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropToken::")?;
        match self {
            Self::Whitespace(_) => write!(f, "Whitespace")?,
            Self::Value(_) => write!(f, "Value")?,
            Self::BinaryOp(_) => write!(f, "BinaryOp")?,
            Self::SuffixUnaryOp(_) => write!(f, "SuffixUnaryOp")?,
            Self::GroupingOpen(_) => write!(f, "GroupingOpen")?,
            Self::GroupingClose(_) => write!(f, "GroupingClose")?,
            Self::Terminator(_) => write!(f, "Terminator")?,
            Self::Comment(_) => write!(f, "Comment")?,
        }
        write!(f, "({:?})", self.as_input())
    }
}

proptest! {
    #[test]
    fn no_panics(input: Vec<u8>) {
        let tokenizer = Tokenizer::new();
        tokenizer
            .tokenize(&Bytes::from(input))
            .for_each(|token| drop(black_box(token)));
    }

    #[test]
    fn one_token(input_token: PropToken) {
        let tokenizer = Tokenizer::new();
        let input = input_token.as_input();
        let mut tokens = tokenizer.tokenize(&input);
        let next_token = tokens.next();
        assert!(
            next_token
                .as_ref()
                .is_some_and(|token| input_token.matches(&token)),
            "expected {input_token:?} but found {next_token:?}"
        );
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn two_tokens(input_token_1: PropToken, input_token_2: PropToken) {
        if matches!(input_token_1, PropToken::BinaryOp(_) | PropToken::SuffixUnaryOp(_)) {
            prop_assume!(!matches!(input_token_2, PropToken::BinaryOp(_) | PropToken::SuffixUnaryOp(_)));
        }
        if matches!(input_token_1, PropToken::Whitespace(_)) {
            prop_assume!(!matches!(input_token_2, PropToken::Whitespace(_)));
        }
        if matches!(input_token_1, PropToken::Value(PropValue::Raw(_) | PropValue::RawEscaped(_))) {
            prop_assume!(!matches!(input_token_2, PropToken::Value(PropValue::Raw(_) | PropValue::RawEscaped(_))));
        }

        let tokenizer = Tokenizer::new();
        let input = Bytes::from_iter(input_token_1.bytes().chain(input_token_2.bytes()));
        let mut tokens = tokenizer.tokenize(&input);
        let first_token = tokens.next();
        assert!(
            first_token
                .as_ref()
                .is_some_and(|token| input_token_1.matches(&token)),
            "expected {input_token_1:?} but found {first_token:?}"
        );
        let second_token = tokens.next();
        assert!(
            second_token
                .as_ref()
                .is_some_and(|token| input_token_2.matches(&token)),
            "expected {input_token_2:?} but found {second_token:?}"
        );
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn many_tokens(input_tokens in any::<Vec<PropToken>>().prop_map(|mut tokens| {
        let mut last_was_rawstr = false;
        let mut last_was_whitespace = false;
        let mut last_was_op = false;
        tokens.retain(|token| {
            let this_is_whitespace = matches!(token, PropToken::Whitespace(_));
            let this_is_rawstr = matches!(token, PropToken::Value(PropValue::Raw(_) | PropValue::RawEscaped(_)));
            let this_is_op = matches!(token, PropToken::BinaryOp(_) | PropToken::SuffixUnaryOp(_));
            if this_is_whitespace && last_was_whitespace {
                return false;
            }
            if this_is_rawstr && last_was_rawstr {
                return false;
            }
            if this_is_op && last_was_op {
                return false;
            }
            last_was_whitespace = this_is_whitespace;
            last_was_rawstr = this_is_rawstr;
            last_was_op = this_is_op;
            true
        });
        tokens
    })) {
        let tokenizer = Tokenizer::new();
        let input = Bytes::from_iter(input_tokens.iter().flat_map(|token| token.bytes()));
        let mut tokens = tokenizer.tokenize(&input);
        for input_token in input_tokens {
            let next_token = tokens.next();
            assert!(
                next_token
                    .as_ref()
                    .is_some_and(|token| input_token.matches(&token)),
                "expected {input_token:?} but found {next_token:?}"
            );
        }
        assert_eq!(tokens.next(), None);
    }
}
