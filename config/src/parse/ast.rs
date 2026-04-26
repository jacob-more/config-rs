use std::{
    convert::Infallible, ffi::OsStr, fmt::Display, hint::cold_path, os::unix::ffi::OsStrExt,
};

use bytes::Bytes;
use display_as_debug_derive::DisplayAsDebug;
use thiserror::Error;

use crate::{
    ext::{IterEscaped, IterJoin},
    parse::{
        BYTES_OPERATOR_ADD, BYTES_OPERATOR_ASSIGN, BYTES_OPERATOR_ASSIGN_IF_UNDEFINED,
        BYTES_OPERATOR_CLEAR, BYTES_OPERATOR_REMOVE, BYTES_OPERATOR_RESET, OPERATOR_ADD,
        OPERATOR_ASSIGN, OPERATOR_ASSIGN_IF_UNDEFINED, OPERATOR_CLEAR, OPERATOR_REMOVE,
        OPERATOR_RESET,
        lex::TokenValue,
        syn::{SyntaxError, SyntaxParser, SyntaxTreeEntry},
    },
};

#[cfg(test)]
mod property_test;
#[cfg(test)]
mod test;

#[derive(DisplayAsDebug, Error)]
#[error(transparent)]
pub struct AstParseError(#[from] SyntaxError);
impl From<Infallible> for AstParseError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ast {
    pub(super) entries: Vec<AstEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AstEntry {
    Group { key: Bytes, group: Vec<AstEntry> },
    Operation { key: Bytes, operation: AstOperation },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AstOperation {
    Assign(Bytes),
    AssignIfUndefined(Bytes),
    Add(Bytes),
    Remove(Bytes),
    Reset,
    Clear,
}

impl FromIterator<AstEntry> for Ast {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl AstEntry {
    fn new_group(key: impl Into<Bytes>, values: impl IntoIterator<Item = AstEntry>) -> Self {
        Self::Group {
            key: key.into(),
            group: values.into_iter().collect(),
        }
    }

    fn new_assign(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Assign(value.into()),
        }
    }

    fn new_assign_if_undefined(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::AssignIfUndefined(value.into()),
        }
    }

    fn new_add(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Add(value.into()),
        }
    }

    fn new_remove(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Remove(value.into()),
        }
    }

    fn new_reset(key: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Reset,
        }
    }

    fn new_clear(key: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Clear,
        }
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.entries.iter().join(' '))
    }
}

impl Display for AstEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn display_key_op_value(
            f: &mut std::fmt::Formatter<'_>,
            key: &Bytes,
            op: &str,
            value: &Bytes,
        ) -> std::fmt::Result {
            write!(
                f,
                "{} {op} \"{}\";",
                OsStr::from_bytes(key).display(),
                OsStr::from_bytes(value).display()
            )
        }

        match self {
            Self::Group { key: name, group } => {
                write!(f, "{}: {{", OsStr::from_bytes(name).display())?;
                if group.is_empty() {
                    write!(f, "{} }}", group.iter().join(' '))?;
                } else {
                    write!(f, "}};")?;
                }
                Ok(())
            }
            Self::Operation {
                key,
                operation: AstOperation::Assign(value),
            } => display_key_op_value(f, key, OPERATOR_ASSIGN, value),
            Self::Operation {
                key,
                operation: AstOperation::AssignIfUndefined(value),
            } => display_key_op_value(f, key, OPERATOR_ASSIGN_IF_UNDEFINED, value),
            Self::Operation {
                key,
                operation: AstOperation::Add(value),
            } => display_key_op_value(f, key, OPERATOR_ADD, value),
            Self::Operation {
                key,
                operation: AstOperation::Remove(value),
            } => display_key_op_value(f, key, OPERATOR_REMOVE, value),
            Self::Operation {
                key,
                operation: AstOperation::Reset,
            } => write!(f, "{} {OPERATOR_RESET};", OsStr::from_bytes(key).display()),
            Self::Operation {
                key,
                operation: AstOperation::Clear,
            } => write!(f, "{} {OPERATOR_CLEAR};", OsStr::from_bytes(key).display()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AstParser {
    syntax_parser: SyntaxParser,
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

    pub fn parse<B>(&self, bytes: B) -> Result<Ast, AstParseError>
    where
        Bytes: From<B>,
    {
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

        fn parse_entry<'a>(syn_entry: SyntaxTreeEntry<'a>) -> Result<AstEntry, AstParseError> {
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
                            .collect::<Result<Vec<AstEntry>, AstParseError>>()?,
                    )
                }
                SyntaxTreeEntry::BinaryOp {
                    identifier,
                    op,
                    value,
                } => {
                    let identifier = capture_string(identifier);
                    let value = capture_string(value);
                    match op.as_slice() {
                        BYTES_OPERATOR_ASSIGN => AstEntry::new_assign(identifier, value),
                        BYTES_OPERATOR_ASSIGN_IF_UNDEFINED => {
                            AstEntry::new_assign_if_undefined(identifier, value)
                        }
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
                }
            })
        }

        let buffer = bytes.into();
        Ok(Ast {
            entries: self
                .syntax_parser
                .parse(&buffer)?
                .entries
                .into_iter()
                .map(parse_entry)
                .collect::<Result<Vec<AstEntry>, AstParseError>>()?,
        })
    }
}
