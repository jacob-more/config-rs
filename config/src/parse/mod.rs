use bytes::Bytes;
use display_as_debug_derive::DisplayAsDebug;
use thiserror::Error;

use crate::parse::ast::AstParser;

mod ast;
mod lex;
mod syn;

/// This module is not considered part of the public API for this crate. It is
/// only exposed so that implementation details of this crate can be
/// benchmarked. It is not recommended that consumers of this crate use anything
/// in this module.
pub mod __private {
    pub use super::ast::*;
    pub use super::lex::*;
    pub use super::syn::*;
}

pub(crate) use ast::*;

pub(crate) const OPERATOR_GROUP: &str = ":";
pub(crate) const OPERATOR_ASSIGN: &str = "=";
pub(crate) const OPERATOR_ASSIGN_IF_UNDEFINED: &str = ":=";
pub(crate) const OPERATOR_ADD: &str = "+=";
pub(crate) const OPERATOR_REMOVE: &str = "-=";
pub(crate) const OPERATOR_RESET: &str = "!";
pub(crate) const OPERATOR_CLEAR: &str = "!!";

const BYTES_OPERATOR_ASSIGN: &[u8] = OPERATOR_ASSIGN.as_bytes();
const BYTES_OPERATOR_ASSIGN_IF_UNDEFINED: &[u8] = OPERATOR_ASSIGN_IF_UNDEFINED.as_bytes();
const BYTES_OPERATOR_ADD: &[u8] = OPERATOR_ADD.as_bytes();
const BYTES_OPERATOR_REMOVE: &[u8] = OPERATOR_REMOVE.as_bytes();
const BYTES_OPERATOR_RESET: &[u8] = OPERATOR_RESET.as_bytes();
const BYTES_OPERATOR_CLEAR: &[u8] = OPERATOR_CLEAR.as_bytes();

#[derive(Debug, Default, Clone)]
pub struct Parser {
    parser: AstParser,
}

#[derive(Debug)]
pub struct ParseEntryIter(std::vec::IntoIter<AstEntry>);

#[derive(DisplayAsDebug, Error)]
#[error(transparent)]
pub struct ParseError(ast::AstParseError);

#[derive(Debug)]
pub enum RawEntry {
    Group { key: Bytes, body: RawGroup },
    Operation { key: Bytes, body: RawOperation },
}
#[derive(Debug)]
pub struct RawGroup(pub(crate) Vec<AstEntry>);
#[derive(Debug)]
pub struct RawOperation(pub(crate) AstOperation);

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_file<P>(&mut self, path: P) -> std::io::Result<Result<ParseEntryIter, ParseError>>
    where
        P: AsRef<std::path::Path>,
    {
        self.parse_reader(std::fs::File::open(path)?)
    }

    pub fn parse_reader<R>(
        &self,
        mut reader: R,
    ) -> std::io::Result<Result<ParseEntryIter, ParseError>>
    where
        R: std::io::Read,
    {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(self.parse_bytes(buffer))
    }

    pub fn parse_bytes<B>(&self, bytes: B) -> Result<ParseEntryIter, ParseError>
    where
        Bytes: From<B>,
    {
        self.parser
            .parse(bytes)
            .map_err(ParseError)
            .map(|ast| ParseEntryIter(ast.entries.into_iter()))
    }
}

impl Iterator for ParseEntryIter {
    type Item = RawEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(RawEntry::new)
    }
}

impl RawEntry {
    pub(crate) fn new(entry: AstEntry) -> Self {
        match entry {
            AstEntry::Group { key, group } => Self::Group {
                key,
                body: RawGroup(group),
            },
            AstEntry::Operation { key, operation } => Self::Operation {
                key,
                body: RawOperation(operation),
            },
        }
    }

    pub(crate) fn display_key(&self) -> impl std::fmt::Display {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        match self {
            Self::Group { key, body: _ } => OsStr::from_bytes(key).display(),
            Self::Operation { key, body: _ } => OsStr::from_bytes(key).display(),
        }
    }
}
