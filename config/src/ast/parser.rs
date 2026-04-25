use std::{convert::Infallible, hint::cold_path};

use bytes::Bytes;
use thiserror::Error;

use crate::{
    ast::{
        Ast, AstEntry, OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED, OPERATOR_CLEAR,
        OPERATOR_REMOVE, OPERATOR_RESET,
    },
    ext::IterEscaped,
    lex::TokenValue,
    syn::{SyntaxError, SyntaxParser, SyntaxTreeEntry},
};

const BYTES_OPERATOR_ASSIGN: &[u8] = OPERATOR_ASSIGN.as_bytes();
const BYTES_OPERATOR_ASSIGN_IF_UNDEFINED: &[u8] = OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes();
const BYTES_OPERATOR_ADD: &[u8] = OPERATOR_ADD.as_bytes();
const BYTES_OPERATOR_REMOVE: &[u8] = OPERATOR_REMOVE.as_bytes();
const BYTES_OPERATOR_RESET: &[u8] = OPERATOR_RESET.as_bytes();
const BYTES_OPERATOR_CLEAR: &[u8] = OPERATOR_CLEAR.as_bytes();

#[cfg(test)]
mod property_test;
#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
pub struct AstParser {
    syntax_parser: SyntaxParser,
}

pub struct AstParse {
    parser: AstParser,
    buffer: Bytes,
}

#[derive(Debug, Error)]
pub enum AstParseError {
    #[error(transparent)]
    SyntaxError(SyntaxError),
}
impl From<Infallible> for AstParseError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl Default for AstParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AstParser {
    pub fn new() -> Self {
        Self {
            syntax_parser: SyntaxParser::new(),
        }
    }
}

impl AstParser {
    pub fn parse_reader<R>(self, mut reader: R) -> std::io::Result<AstParse>
    where
        R: std::io::Read,
    {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(AstParse {
            parser: self,
            buffer: Bytes::from(buffer),
        })
    }

    pub fn parse_bytes<B>(self, bytes: B) -> AstParse
    where
        Bytes: From<B>,
    {
        AstParse {
            parser: self,
            buffer: bytes.into(),
        }
    }
}

impl AstParse {
    pub fn parse_into_ast(self) -> Result<Ast, Box<AstParseError>> {
        fn capture_string<'a>(string: TokenValue<'a>) -> Bytes {
            match string
                .captures
                .name("string")
                .or_else(|| string.captures.name("qstring"))
                .map(|matched| string.buffer.slice(matched.range()))
                .or_else(|| {
                    // Replacing escaped characters requires allocating a new
                    // `Bytes` buffer. We'd rather not re-allocate. Hence, why
                    // this is its own capture group.
                    string
                        .captures
                        .name("estring")
                        .or_else(|| string.captures.name("qestring"))
                        .map(|matched| matched.as_bytes().unescaped().copied().collect())
                }) {
                Some(string) => string,
                None => {
                    cold_path();
                    panic!("TokenValue must match a known string variant")
                }
            }
        }

        fn parse_entry<'a>(syn_entry: SyntaxTreeEntry<'a>) -> Result<AstEntry, Box<AstParseError>> {
            Ok(match syn_entry {
                SyntaxTreeEntry::Group {
                    identifier,
                    entries,
                } => {
                    let identifier = capture_string(identifier);
                    AstEntry::new_group(
                        identifier,
                        entries
                            .into_iter()
                            .map(parse_entry)
                            .collect::<Result<Vec<AstEntry>, Box<AstParseError>>>()?,
                    )
                },
                SyntaxTreeEntry::BinaryOp {
                    identifier,
                    op,
                    value,
                } => {
                    let identifier = capture_string(identifier);
                    let value = capture_string(value);
                    match op.as_slice() {
                        BYTES_OPERATOR_ASSIGN => AstEntry::new_assign(identifier, value),
                        BYTES_OPERATOR_ASSIGN_IF_UNDEFINED => AstEntry::new_assign_if_undefined(identifier, value),
                        BYTES_OPERATOR_ADD => AstEntry::new_add(identifier, value),
                        BYTES_OPERATOR_REMOVE => AstEntry::new_remove(identifier, value),
                        _ => {
                            cold_path();
                            panic!("binary operator must match known variant");
                        }
                    }
                }
                SyntaxTreeEntry::SuffixUnaryOp { identifier, op } => {
                    let identifier = capture_string(identifier);
                    match op.as_slice() {
                        BYTES_OPERATOR_RESET => AstEntry::new_reset(identifier),
                        BYTES_OPERATOR_CLEAR => AstEntry::new_clear(identifier),
                        _ => {
                            cold_path();
                            panic!("unary operator must match known variant");
                        }
                    }
                },
            })
        }

        Ok(Ast {
            entries: match self.parser.syntax_parser.parse(&self.buffer) {
                Ok(tree) => tree
                    .entries
                    .into_iter()
                    .map(parse_entry)
                    .collect::<Result<Vec<AstEntry>, Box<AstParseError>>>()?,
                Err(error) => return Err(Box::new(AstParseError::SyntaxError(error))),
            },
        })
    }
}
