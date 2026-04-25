use std::{convert::Infallible, ffi::OsStr, iter::Peekable, ops::Deref, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use display_as_debug_derive::DisplayAsDebug;
use thiserror::Error;

use crate::{
    ast::OPERATOR_GROUP,
    lex::{
        CONFIG_LEXICAL_TOKENIZER, Span, Token, TokenBinaryOp, TokenSuffixUnaryOp, TokenValue,
        Tokenizer,
    },
};

#[derive(DisplayAsDebug, Error)]
enum ReprSyntaxError {
    #[error(
        "expected {expected} but found {token}({}) at {span}",
        OsStr::from_bytes(bytes.deref()).display()
    )]
    Expected {
        expected: &'static str,
        token: &'static str,
        bytes: Bytes,
        span: Span,
    },
    #[error(
        "expected {expected} after '{} {}' but found {token}({}) at {span}",
        OsStr::from_bytes(key.deref()).display(),
        OsStr::from_bytes(operation.deref()).display(),
        OsStr::from_bytes(bytes.deref()).display(),
    )]
    ExpectedValueAfterBinaryOperation {
        key: Bytes,
        operation: Bytes,
        expected: &'static str,
        token: &'static str,
        bytes: Bytes,
        span: Span,
    },
    #[error("expected {expected} but found end-of-file")]
    ExpectedButFoundEOF { expected: &'static str },
    #[error(
        "expected {expected} after '{} {}' but found end-of-file",
        OsStr::from_bytes(key.deref()).display(),
        OsStr::from_bytes(operation.deref()).display(),
    )]
    ExpectedValueAfterBinaryOperationButFoundEOF {
        key: Bytes,
        operation: Bytes,
        expected: &'static str,
    },
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
#[derive(DisplayAsDebug, Error)]
#[error(transparent)]
pub struct SyntaxError(#[from] Box<ReprSyntaxError>);
impl From<ReprSyntaxError> for SyntaxError {
    fn from(value: ReprSyntaxError) -> Self {
        SyntaxError(Box::new(value))
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxParser {
    tokenizer: Tokenizer,
}

#[derive(Debug)]
pub struct SyntaxTree<'a> {
    pub(crate) entries: Vec<SyntaxTreeEntry<'a>>,
}

#[derive(Debug)]
pub(crate) enum SyntaxTreeEntry<'a> {
    Group {
        identifier: TokenValue<'a>,
        entries: Vec<SyntaxTreeEntry<'a>>,
    },
    BinaryOp {
        identifier: TokenValue<'a>,
        op: TokenBinaryOp<'a>,
        value: TokenValue<'a>,
    },
    SuffixUnaryOp {
        identifier: TokenValue<'a>,
        op: TokenSuffixUnaryOp<'a>,
    },
}

impl Default for SyntaxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxParser {
    pub fn new() -> Self {
        Self {
            tokenizer: CONFIG_LEXICAL_TOKENIZER.clone(),
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
        let entry =
            match tokens.find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_))) {
                Some(Token::BinaryOp(op)) if op.as_slice() == OPERATOR_GROUP.as_bytes() => {
                    let _group_open = match tokens
                        .find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_)))
                    {
                        Some(Token::GroupingOpen(value)) => value,
                        Some(token) => {
                            return Err(ReprSyntaxError::ExpectedValueAfterBinaryOperation {
                                key: identifier.as_bytes(),
                                operation: op.as_bytes(),
                                expected: "group opening '{'",
                                token: token.ident(),
                                bytes: token.as_bytes(),
                                span: token.span(),
                            }
                            .into());
                        }
                        None => {
                            return Err(
                                ReprSyntaxError::ExpectedValueAfterBinaryOperationButFoundEOF {
                                    key: identifier.as_bytes(),
                                    operation: op.as_bytes(),
                                    expected: "group opening '{'",
                                }
                                .into(),
                            );
                        }
                    };
                    let mut entries = Vec::new();
                    let _group_close = loop {
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
                                    error: error.0,
                                }
                            })?),
                        }
                    };
                    Self::Group {
                        identifier,
                        entries,
                    }
                }
                Some(Token::BinaryOp(op)) => {
                    let value = match tokens
                        .find(|token| !matches!(token, Token::Whitespace(_) | Token::Comment(_)))
                    {
                        Some(Token::Value(value)) => value,
                        Some(token) => {
                            return Err(ReprSyntaxError::ExpectedValueAfterBinaryOperation {
                                key: identifier.as_bytes(),
                                operation: op.as_bytes(),
                                expected: "value string",
                                token: token.ident(),
                                bytes: token.as_bytes(),
                                span: token.span(),
                            }
                            .into());
                        }
                        None => {
                            return Err(
                                ReprSyntaxError::ExpectedValueAfterBinaryOperationButFoundEOF {
                                    key: identifier.as_bytes(),
                                    operation: op.as_bytes(),
                                    expected: "value string",
                                }
                                .into(),
                            );
                        }
                    };
                    Self::BinaryOp {
                        identifier,
                        op,
                        value,
                    }
                }
                Some(Token::SuffixUnaryOp(op)) => Self::SuffixUnaryOp { identifier, op },
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
        let _terminator = tokens.next_if_map(|token| match token {
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
        loop {
            match tokens.peek() {
                Some(Token::Whitespace(_) | Token::Comment(_) | Token::Terminator(_)) => {
                    tokens.next();
                }
                Some(_) => entries.push(SyntaxTreeEntry::parse(&mut tokens)?),
                None => break,
            }
        }
        Ok(SyntaxTree { entries })
    }
}
