use std::{convert::Infallible, ffi::OsStr, iter::Peekable, ops::Deref, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use thiserror::Error;

use crate::{
    ast::OPERATOR_GROUP,
    lex::{
        CONFIG_LEXICAL_TOKENIZER, Span, Token, TokenBinaryOp, TokenGroupingClose,
        TokenGroupingOpen, TokenSuffixUnaryOp, TokenTerminator, TokenValue, Tokenizer,
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
