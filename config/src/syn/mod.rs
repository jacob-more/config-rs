use std::{
    convert::Infallible, ffi::OsStr, iter::Peekable, ops::Deref, os::unix::ffi::OsStrExt,
    sync::LazyLock,
};

use bytes::Bytes;
use thiserror::Error;

use crate::{
    ast::{
        OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED, OPERATOR_CLEAR,
        OPERATOR_GROUP, OPERATOR_REMOVE, OPERATOR_RESET,
    },
    ext::IterJoin,
    lex::{
        Span, Token, TokenBinaryOp, TokenGroupingClose, TokenGroupingOpen, TokenSuffixUnaryOp,
        TokenTerminator, TokenValue, Tokenizer, TokenizerBuilder,
    },
};

#[derive(Debug, Error)]
enum ReprSyntaxError {
    #[error(
        "unexpected {expected} but found {token}({}) at {span}",
        OsStr::from_bytes(bytes.deref()).display()
    )]
    Expected {
        expected: &'static str,
        token: &'static str,
        bytes: Bytes,
        span: Span,
    },
    #[error("unexpected {expected} but found end-of-file")]
    ExpectedButFoundEOF { expected: &'static str },
    #[error(
        "in group {}, {error}",
        OsStr::from_bytes(group.deref()).display()
    )]
    InGroup { group: Bytes, error: Box<Self> },
}
impl From<Infallible> for ReprSyntaxError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
#[derive(Debug, Error)]
#[error(transparent)]
pub struct SyntaxError(#[from] ReprSyntaxError);

#[derive(Debug, Clone)]
pub struct SyntaxParser {
    tokenizer: Tokenizer,
}

#[derive(Debug)]
pub struct SyntaxTree<'a> {
    entries: Vec<SyntaxTreeEntry<'a>>,
}

#[derive(Debug)]
enum SyntaxTreeEntry<'a> {
    Group {
        identifier: TokenValue<'a>,
        op: TokenBinaryOp<'a>,
        open: TokenGroupingOpen<'a>,
        entries: Vec<SyntaxTreeEntry<'a>>,
        close: TokenGroupingClose<'a>,
        terminator: Option<TokenTerminator<'a>>,
    },
    BinaryOp {
        identifier: TokenValue<'a>,
        op: TokenBinaryOp<'a>,
        value: TokenValue<'a>,
        terminator: Option<TokenTerminator<'a>>,
    },
    SuffixUnaryOp {
        identifier: TokenValue<'a>,
        op: TokenSuffixUnaryOp<'a>,
        terminator: Option<TokenTerminator<'a>>,
    },
}

impl Default for SyntaxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxParser {
    pub fn new() -> Self {
        static LEXICAL_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
            let mut tokenizer = TokenizerBuilder::new();
            tokenizer.value(concat!(
                r##""(?<qestring>[^"\\]|\\.)*""##, // qstring + escapes
                r"|",
                r##""(?<qstring>[^"\\]*)""##, // qstring
                r"|",
                r"(?<estring>(?:[A-Za-z0-9_./]|\\.)(?:(?:[A-Za-z0-9_./\-:]|\\.)*(?:[A-Za-z0-9_./]|\\.))?)", // raw string + escapes
                r"|",
                r"(?<string>[A-Za-z0-9_./](?:[A-Za-z0-9_./\-:]*[A-Za-z0-9_./])?)", // raw string
            ));
            let suffix_unary_ops = [regex::escape(OPERATOR_RESET), regex::escape(OPERATOR_CLEAR)]
                .join('|')
                .to_string();
            tokenizer.suffix_unary_op(&suffix_unary_ops);
            let binary_ops = [
                regex::escape(OPERATOR_ASSIGN),
                regex::escape(OPERATOR_ASSIGN_IF_UNDEFINED),
                regex::escape(OPERATOR_ADD),
                regex::escape(OPERATOR_REMOVE),
                regex::escape(OPERATOR_GROUP),
            ]
            .join('|')
            .to_string();
            tokenizer.binary_op(&binary_ops);
            tokenizer.grouping_open(r"\{");
            tokenizer.grouping_close(r"\}");
            tokenizer.terminator(r";");
            tokenizer.comment(r"(?-su:#.*)");
            tokenizer.whitespace(r"(?-u:\s|\r|\n)+");
            tokenizer.finalize().unwrap()
        });

        Self {
            tokenizer: LEXICAL_TOKENIZER.clone(),
        }
    }
}

impl<'a> SyntaxTreeEntry<'a> {
    pub fn parse(
        tokens: &mut Peekable<impl Iterator<Item = Token<'a>>>,
    ) -> Result<Self, SyntaxError> {
        let identifier = match tokens.find(|token| {
            !matches!(
                token,
                Token::Whitespace(_) | Token::Comment(_) | Token::Terminator(_)
            )
        }) {
            Some(Token::Value(identifier)) => identifier,
            Some(token) => {
                return Err(ReprSyntaxError::Expected {
                    expected: "key string",
                    token: token.ident(),
                    bytes: token.as_bytes(),
                    span: token.span(),
                }
                .into());
            }
            None => {
                return Err(ReprSyntaxError::ExpectedButFoundEOF {
                    expected: "key string",
                }
                .into());
            }
        };
        let mut entry =
            match tokens.find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_))) {
                Some(Token::BinaryOp(op)) if op.as_slice() == OPERATOR_GROUP.as_bytes() => {
                    let group_open = match tokens
                        .find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_)))
                    {
                        Some(Token::GroupingOpen(value)) => value,
                        Some(token) => {
                            return Err(ReprSyntaxError::Expected {
                                expected: "group opening '{'",
                                token: token.ident(),
                                bytes: token.as_bytes(),
                                span: token.span(),
                            }
                            .into());
                        }
                        None => {
                            return Err(ReprSyntaxError::ExpectedButFoundEOF {
                                expected: "group opening '{'",
                            }
                            .into());
                        }
                    };
                    let mut entries = Vec::new();
                    let group_close = loop {
                        match tokens.next_if(|token| {
                            matches!(
                                token,
                                Token::GroupingClose(_)
                                    | Token::Whitespace(_)
                                    | Token::Comment(_)
                                    | Token::Terminator(_)
                            )
                        }) {
                            Some(Token::GroupingClose(group_close)) => break group_close,
                            Some(_) => (),
                            None => entries.push(Self::parse(tokens).map_err(|error| {
                                ReprSyntaxError::InGroup {
                                    group: identifier.as_bytes(),
                                    error: Box::new(error.0),
                                }
                            })?),
                        }
                    };
                    Self::Group {
                        identifier,
                        op,
                        open: group_open,
                        entries,
                        close: group_close,
                        terminator: None,
                    }
                }
                Some(Token::BinaryOp(op)) => {
                    let value = match tokens
                        .find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_)))
                    {
                        Some(Token::Value(value)) => value,
                        Some(token) => {
                            return Err(ReprSyntaxError::Expected {
                                expected: "value string after operator",
                                token: token.ident(),
                                bytes: token.as_bytes(),
                                span: token.span(),
                            }
                            .into());
                        }
                        None => {
                            return Err(ReprSyntaxError::ExpectedButFoundEOF {
                                expected: "value string after operator",
                            }
                            .into());
                        }
                    };
                    Self::BinaryOp {
                        identifier,
                        op,
                        value,
                        terminator: None,
                    }
                }
                Some(Token::SuffixUnaryOp(op)) => Self::SuffixUnaryOp {
                    identifier,
                    op,
                    terminator: None,
                },
                Some(token) => {
                    return Err(ReprSyntaxError::Expected {
                        expected: "operator after key string",
                        token: token.ident(),
                        bytes: token.as_bytes(),
                        span: token.span(),
                    }
                    .into());
                }
                None => {
                    return Err(ReprSyntaxError::ExpectedButFoundEOF {
                        expected: "operator after key string",
                    }
                    .into());
                }
            };
        while tokens
            .next_if(|token| matches!(token, Token::Whitespace(_) | Token::Comment(_)))
            .is_some()
        {}
        let terminator = match &mut entry {
            Self::Group { terminator, .. } => terminator,
            Self::BinaryOp { terminator, .. } => terminator,
            Self::SuffixUnaryOp { terminator, .. } => terminator,
        };
        *terminator = tokens.next_if_map(|token| match token {
            Token::Terminator(terminator) => Ok(terminator),
            _ => Err(token),
        });
        Ok(entry)
    }
}

impl SyntaxParser {
    pub fn parse<'a>(&self, bytes: &'a Bytes) -> Result<SyntaxTree<'a>, SyntaxError> {
        let mut tokens = self.tokenizer.tokenize(bytes).peekable();
        let mut entries = Vec::new();
        while tokens.peek().is_some() {
            entries.push(SyntaxTreeEntry::parse(&mut tokens)?);
        }
        Ok(SyntaxTree { entries })
    }
}
