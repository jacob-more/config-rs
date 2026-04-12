use std::{collections::HashMap, ffi::OsStr, fmt::Debug, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use thiserror::Error;

pub mod ast;
pub mod ext;

pub mod cval;
pub(crate) mod header;
pub(crate) mod history;

mod access_control_list;
mod list;
mod set;
mod value;

pub use cval::*;

pub use access_control_list::*;
pub use list::*;
pub use set::*;
pub use value::*;

pub mod derive {
    pub use config_derive::*;
}

use crate::ast::{AstEntry, AstGroup, AstOperation, AstTree};
#[derive(Debug)]
pub enum Operation<T: ICval> {
    Assign(Cval<T>),
    AssignIfUndefined(Cval<T>),
    Add(Cval<T>),
    Remove(Cval<T>),
    Reset,
    Clear,
}
impl<T> Clone for Operation<T>
where
    T: ICval,
{
    fn clone(&self) -> Self {
        match self {
            Self::Assign(value) => Self::Assign(value.clone()),
            Self::AssignIfUndefined(value) => Self::AssignIfUndefined(value.clone()),
            Self::Add(value) => Self::Add(value.clone()),
            Self::Remove(value) => Self::Remove(value.clone()),
            Self::Reset => Self::Reset,
            Self::Clear => Self::Clear,
        }
    }
}

#[derive(Debug, Error)]
enum ReprParseConfigOperationError {
    #[error(transparent)]
    ParseInteger(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    ParseUtf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ParseIpAddress(#[from] std::net::AddrParseError),
    #[error("invalid boolean value")]
    ParseBoolean,
    #[error("char must be represented by exactly one character")]
    ParseChar,
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ConfigParseOperationError(#[from] ReprParseConfigOperationError);
macro_rules! impl_from_config_parse_error {
    ($ty:ty) => {
        impl From<$ty> for ConfigParseOperationError {
            fn from(value: $ty) -> Self {
                ConfigParseOperationError(ReprParseConfigOperationError::from(value))
            }
        }
    };
}
impl_from_config_parse_error!(std::num::ParseIntError);
impl_from_config_parse_error!(std::num::ParseFloatError);
impl_from_config_parse_error!(std::str::Utf8Error);
impl_from_config_parse_error!(std::net::AddrParseError);

#[derive(Debug, Error)]
pub enum ConfigParseError {
    #[error("{} is not a valid key", .0.display_key())]
    UnknownKey(AstEntry),
    #[error("{} is not a valid group key although it might be a valid operation key", .0.display_key())]
    UnknownGroupKey(AstEntry),
    #[error("{} is not a valid operation key although it might be a valid group key", .0.display_key())]
    UnknownOperationKey(AstEntry),
    #[error(transparent)]
    Group(#[from] ConfigParseGroupError),
    #[error(transparent)]
    Operation(#[from] ConfigParseOperationError),
}

#[derive(Debug, Error)]
pub enum ConfigParseGroupError {
    #[error("{} is not a valid key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownKey { group: Bytes, entry: AstEntry },
    #[error("{} is not a valid group key although it might be a valid operation key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownGroupKey { group: Bytes, entry: AstEntry },
    #[error("{} is not a valid operation key although it might be a valid group key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownOperationKey { group: Bytes, entry: AstEntry },
    #[error("error in group {}: {error}", OsStr::from_bytes(.group).display())]
    Group {
        group: Bytes,
        error: Box<ConfigParseGroupError>,
    },
    #[error("error in group {}: {error}", OsStr::from_bytes(.group).display())]
    Operation {
        group: Bytes,
        error: ConfigParseOperationError,
    },
}

pub trait Config {
    type Err;

    fn parse_ast(&mut self, ast: AstTree) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
}

pub trait ConfigGroup {
    type Err;

    fn new(key: bytes::Bytes) -> Self;
    fn parse_ast_group(&mut self, key: bytes::Bytes, group: AstGroup) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
}

pub trait ConfigOperation<T>
where
    T: ICval,
{
    fn assign<C: Into<Cval<T>>>(&mut self, value: C);
    fn assign_if_undefined<C: Into<Cval<T>>>(&mut self, value: C);
    fn add<C: Into<Cval<T>>>(&mut self, value: C);
    fn remove<C: Into<Cval<T>>>(&mut self, value: C);
    fn reset(&mut self);
    fn clear(&mut self);

    fn is_default(&self) -> bool;
    fn is_defined(&self) -> bool;
    fn history<'a>(&'a self) -> impl Iterator<Item = &'a Operation<T>>
    where
        T: 'a;
}

pub trait ConfigOperationExt<T>: ConfigOperation<T>
where
    T: ICval,
    Cval<T>: TryFrom<bytes::Bytes, Error = ConfigParseOperationError>,
{
    fn parse_ast_entry(
        &mut self,
        key: bytes::Bytes,
        operation: AstOperation,
    ) -> Result<(), ConfigParseOperationError> {
        let _ = key; // key is unused here, but required for the trait.

        match operation {
            AstOperation::Assign(value) => self.assign(Cval::try_from(value)?),
            AstOperation::AssignIfUndefined(value) => {
                self.assign_if_undefined(Cval::try_from(value)?)
            }
            AstOperation::Add(value) => self.add(Cval::try_from(value)?),
            AstOperation::Remove(value) => self.remove(Cval::try_from(value)?),
            AstOperation::Reset => self.reset(),
            AstOperation::Clear => self.clear(),
        }
        Ok(())
    }

    fn apply(&mut self, event: Operation<T>) {
        match event {
            Operation::Assign(value) => self.assign(value),
            Operation::AssignIfUndefined(value) => self.assign_if_undefined(value),
            Operation::Add(value) => self.add(value),
            Operation::Remove(value) => self.remove(value),
            Operation::Reset => self.reset(),
            Operation::Clear => self.clear(),
        }
    }

    fn replay(&mut self, other: &Self) {
        other.history().cloned().for_each(|event| self.apply(event));
    }
}
impl<C, T> ConfigOperationExt<T> for C
where
    C: ConfigOperation<T>,
    T: ICval,
    Cval<T>: TryFrom<bytes::Bytes, Error = ConfigParseOperationError>,
{
}

impl<C> ConfigGroup for HashMap<Bytes, C>
where
    C: ConfigGroup<Err = ConfigParseGroupError>,
{
    type Err = ConfigParseGroupError;

    fn new(_key: bytes::Bytes) -> Self {
        Self::default()
    }

    fn parse_ast_group(&mut self, key: bytes::Bytes, group: AstGroup) -> Result<(), Self::Err> {
        let parent_group = key;
        for entry in group.into_entries() {
            match entry {
                AstEntry::Group { key, group } => {
                    self.entry(key.clone())
                        .or_insert_with(|| ConfigGroup::new(key.clone()))
                        .parse_ast_group(key, group)?;
                }
                AstEntry::Operation { key, operation } => {
                    return Err(Self::Err::UnknownOperationKey {
                        group: parent_group,
                        entry: AstEntry::Operation { key, operation },
                    });
                }
            }
        }
        Ok(())
    }

    fn replay(&mut self, other: &Self) {
        for (key, group) in other.iter() {
            self.entry(key.clone())
                .or_insert_with(|| ConfigGroup::new(key.clone()))
                .replay(group);
        }
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::LazyLock};

    use rstest::rstest;

    static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        examples_path.push("benches");
        examples_path.push("examples");
        examples_path
    });

    #[rstest]
    #[case("cargo.lock.conf")]
    #[case("root_hints.conf")]
    fn test_parse_to_ast(#[case] file_name: &str) {
        use std::fs::read_to_string;

        use bytes::Bytes;

        let mut config_path = EXAMPLES_DIRECTORY.clone();
        config_path.push("config_name");

        let file_data = Bytes::from(
            read_to_string(config_path.with_file_name(file_name))
                .unwrap()
                .into_bytes(),
        );
        let ast = crate::ast::AstTree::parse_bytes(file_data);
        assert!(ast.is_ok(), "AST is not Ok: {ast:?}");
    }
}
