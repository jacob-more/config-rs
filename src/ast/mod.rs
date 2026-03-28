use std::{ffi::OsStr, fmt::Display, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use thiserror::Error;

use crate::{ast::parser::AstParser, ext::Join};

mod parser;

pub const OPERATOR_ASSIGN: &str = "=";
pub const OPERATOR_ASSIGN_IF_UNDEFINED: &str = ":=";
pub const OPERATOR_ADD: &str = "+=";
pub const OPERATOR_REMOVE: &str = "-=";
pub const OPERATOR_RESET: &str = "!";

#[derive(Debug, Error)]
#[error(transparent)]
pub struct AstParseError(#[from] parser::AstParseError);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstTree {
    entries: Vec<AstEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstEntry {
    Group { key: Bytes, entries: Vec<AstEntry> },
    Operation { key: Bytes, operation: AstOperation },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstOperation {
    Assign(Bytes),
    AssignIfUndefined(Bytes),
    Add(Bytes),
    Remove(Bytes),
    Reset,
}

impl AstTree {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = &'_ AstEntry> {
        self.entries.iter()
    }

    pub fn into_entries(self) -> impl Iterator<Item = AstEntry> {
        self.entries.into_iter()
    }

    pub fn parse_reader<R>(self, reader: R) -> std::io::Result<Result<Self, AstParseError>>
    where
        R: std::io::Read,
    {
        Ok(AstParser::new()
            .parse_reader(reader)?
            .to_tree()
            .map_err(|e| e.into()))
    }

    pub fn parse_bytes<B>(self, bytes: B) -> Result<Self, AstParseError>
    where
        Bytes: From<B>,
    {
        Ok(AstParser::new().parse_bytes(bytes).to_tree()?)
    }
}

impl Default for AstTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<AstEntry> for AstTree {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl AstEntry {
    pub fn new_group(key: impl Into<Bytes>, values: impl IntoIterator<Item = AstEntry>) -> Self {
        Self::Group {
            key: key.into(),
            entries: values.into_iter().collect(),
        }
    }

    pub fn new_assign(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Assign(value.into()),
        }
    }

    pub fn new_assign_if_undefined(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::AssignIfUndefined(value.into()),
        }
    }

    pub fn new_add(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Add(value.into()),
        }
    }

    pub fn new_remove(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Remove(value.into()),
        }
    }

    pub fn new_reset(key: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Reset,
        }
    }
}

impl Display for AstTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Join::new(self.entries.iter(), ' '))
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
                "{} {op} \"{:?}\";",
                OsStr::from_bytes(key).display(),
                OsStr::from_bytes(value).display()
            )
        }

        match self {
            Self::Group { key: name, entries } => {
                write!(f, "{}: {{", OsStr::from_bytes(name).display())?;
                if entries.is_empty() {
                    write!(f, "{} }}", Join::new(entries.iter(), ' '))?;
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
        }
    }
}
