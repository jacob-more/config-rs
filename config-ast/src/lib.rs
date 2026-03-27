use std::{ffi::OsStr, fmt::Display, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use display_ext::Join;
use thiserror::Error;

use crate::parser::AstParser;

pub mod parser;

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
    Group {
        name: Bytes,
        entries: Vec<AstEntry>,
    },
    KeyOpValue {
        key: Bytes,
        operator: Bytes,
        value: Bytes,
    },
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
    fn new_key_value(
        key: impl Into<Bytes>,
        operator: impl Into<Bytes>,
        value: impl Into<Bytes>,
    ) -> Self {
        Self::KeyOpValue {
            key: key.into(),
            operator: operator.into(),
            value: value.into(),
        }
    }

    pub fn new_group(key: impl Into<Bytes>, values: impl IntoIterator<Item = AstEntry>) -> Self {
        Self::Group {
            name: key.into(),
            entries: values.into_iter().collect(),
        }
    }

    pub fn new_assign(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(OPERATOR_ASSIGN.as_bytes()), value)
    }

    pub fn new_assign_if_undefined(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(
            key,
            Bytes::from_static(OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes()),
            value,
        )
    }

    pub fn new_add(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(OPERATOR_ADD.as_bytes()), value)
    }

    pub fn new_remove(key: impl Into<Bytes>, value: impl Into<Bytes>) -> Self {
        Self::new_key_value(key, Bytes::from_static(OPERATOR_REMOVE.as_bytes()), value)
    }

    pub fn new_reset(key: impl Into<Bytes>) -> Self {
        Self::new_key_value(
            key,
            Bytes::from_static(OPERATOR_RESET.as_bytes()),
            Bytes::new(),
        )
    }
}

impl Display for AstTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Join::new(self.entries.iter(), ' '))
    }
}

impl Display for AstEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Group { name, entries } => {
                write!(f, "{}: {{", OsStr::from_bytes(name).display())?;
                if entries.is_empty() {
                    write!(f, "{} }}", Join::new(entries.iter(), ", "))?;
                } else {
                    write!(f, "}};")?;
                }
                Ok(())
            }
            Self::KeyOpValue {
                key,
                operator,
                value,
            } => {
                write!(
                    f,
                    "{} {} {};",
                    OsStr::from_bytes(key).display(),
                    OsStr::from_bytes(operator).display(),
                    OsStr::from_bytes(value).display()
                )
            }
        }
    }
}
