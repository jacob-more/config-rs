use std::{convert::Infallible, ffi::OsStr, fmt::Display, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use thiserror::Error;

use crate::{ast::parser::AstParser, ext::IterJoin};

mod parser;

pub const OPERATOR_ASSIGN: &str = "=";
pub const OPERATOR_ASSIGN_IF_UNDEFINED: &str = ":=";
pub const OPERATOR_ADD: &str = "+=";
pub const OPERATOR_REMOVE: &str = "-=";
pub const OPERATOR_RESET: &str = "!";
pub const OPERATOR_CLEAR: &str = "!!";

#[derive(Debug, Error)]
#[error(transparent)]
pub struct AstParseError(#[from] Box<parser::AstParseError>);
impl From<Infallible> for AstParseError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    entries: Vec<AstEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstEntry {
    Group { key: Bytes, group: AstGroup },
    Operation { key: Bytes, operation: AstOperation },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstOperation {
    Assign(Bytes),
    AssignIfUndefined(Bytes),
    Add(Bytes),
    Remove(Bytes),
    Reset,
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstGroup(Vec<AstEntry>);

impl AstGroup {
    pub fn entries(&self) -> impl Iterator<Item = &'_ AstEntry> {
        self.0.iter()
    }

    pub fn into_entries(self) -> impl Iterator<Item = AstEntry> {
        self.0.into_iter()
    }
}

impl Ast {
    pub const fn new() -> Self {
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

    pub fn from_reader<R>(reader: R) -> std::io::Result<Result<Self, AstParseError>>
    where
        R: std::io::Read,
    {
        Ok(AstParser::new()
            .parse_reader(reader)?
            .parse_into_ast()
            .map_err(|e| e.into()))
    }

    pub fn from_bytes<B>(bytes: B) -> Result<Self, AstParseError>
    where
        Bytes: From<B>,
    {
        Ok(AstParser::new().parse_bytes(bytes).parse_into_ast()?)
    }
}

impl Default for Ast {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<AstEntry> for Ast {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl FromIterator<AstEntry> for AstGroup {
    fn from_iter<T: IntoIterator<Item = AstEntry>>(entries: T) -> Self {
        Self(entries.into_iter().collect())
    }
}

impl AstEntry {
    pub fn new_group(key: impl Into<Bytes>, values: impl IntoIterator<Item = AstEntry>) -> Self {
        Self::Group {
            key: key.into(),
            group: values.into_iter().collect(),
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

    pub fn new_clear(key: impl Into<Bytes>) -> Self {
        Self::Operation {
            key: key.into(),
            operation: AstOperation::Clear,
        }
    }

    pub(crate) fn key(&self) -> &Bytes {
        match self {
            Self::Group { key, .. } => key,
            Self::Operation { key, .. } => key,
        }
    }

    pub(crate) fn display_key(&self) -> impl Display {
        OsStr::from_bytes(self.key()).display()
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
                if group.0.is_empty() {
                    write!(f, "{} }}", group.0.iter().join(' '))?;
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
