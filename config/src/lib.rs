use std::{ffi::OsStr, fmt::Debug, os::unix::ffi::OsStrExt};

use bytes::Bytes;
use thiserror::Error;

pub mod ast;
pub mod ext;

pub(crate) mod header;
pub(crate) mod history;
pub mod replay;

mod access_control_list;
mod list;
mod set;
mod value;

pub use replay::*;

pub use access_control_list::*;
pub use list::*;
pub use set::*;
pub use value::*;

use crate::ast::{AstEntry, AstGroup, AstOperation, AstTree};

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
    #[error("{} is not a valid operation key although it might be a valid group key", .0.display_key())]
    UnknownOperationKey(AstEntry),
    #[error("{} is not a valid group key although it might be a valid operation key", .0.display_key())]
    UnknownGroupKey(AstEntry),
    #[error("{} is not a valid key", .0.display_key())]
    UnknownKey(AstEntry),
}

#[derive(Debug, Error)]
pub enum ConfigParseGroupError {
    #[error("{} is not a valid operation key although it might be a valid group key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownOperationKey { group: Bytes, entry: AstEntry },
    #[error("{} is not a valid group key although it might be a valid operation key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownGroupKey { group: Bytes, entry: AstEntry },
    #[error("{} is not a valid key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownKey { group: Bytes, entry: AstEntry },
}

pub trait Config {
    type Err;

    fn parse_ast(&mut self, ast: AstTree) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
}

pub trait ConfigGroup {
    type Err;

    fn parse_ast_group(&mut self, key: bytes::Bytes, group: AstGroup) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
}

pub trait ConfigOperation<T>
where
    T: Replayable,
{
    fn assign<C: Into<Conf<T>>>(&mut self, value: C);
    fn assign_if_undefined<C: Into<Conf<T>>>(&mut self, value: C);
    fn add<C: Into<Conf<T>>>(&mut self, value: C);
    fn remove<C: Into<Conf<T>>>(&mut self, value: C);
    fn reset(&mut self);
    fn clear(&mut self);

    fn is_default(&self) -> bool;
    fn is_defined(&self) -> bool;
    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a;
}

pub trait ConfigOperationExt<T>: ConfigOperation<T>
where
    T: Replayable,
    Conf<T>: TryFrom<bytes::Bytes, Error = ConfigParseOperationError>,
{
    fn parse_ast_entry(
        &mut self,
        key: bytes::Bytes,
        operation: AstOperation,
    ) -> Result<(), ConfigParseOperationError> {
        let _ = key; // key is unused here, but required for the trait.

        match operation {
            AstOperation::Assign(value) => self.assign(Conf::try_from(value)?),
            AstOperation::AssignIfUndefined(value) => {
                self.assign_if_undefined(Conf::try_from(value)?)
            }
            AstOperation::Add(value) => self.add(Conf::try_from(value)?),
            AstOperation::Remove(value) => self.remove(Conf::try_from(value)?),
            AstOperation::Reset => self.reset(),
            AstOperation::Clear => self.clear(),
        }
        Ok(())
    }

    fn apply(&mut self, event: ReplayOperation<T>) {
        match event {
            ReplayOperation::Assign(value) => self.assign(value),
            ReplayOperation::AssignIfUndefined(value) => self.assign_if_undefined(value),
            ReplayOperation::Add(value) => self.add(value),
            ReplayOperation::Remove(value) => self.remove(value),
            ReplayOperation::Reset => self.reset(),
            ReplayOperation::Clear => self.clear(),
        }
    }

    fn replay(&mut self, other: &Self) {
        other.history().cloned().for_each(|event| self.apply(event));
    }
}
impl<C, T> ConfigOperationExt<T> for C
where
    C: ConfigOperation<T>,
    T: Replayable,
    Conf<T>: TryFrom<bytes::Bytes, Error = ConfigParseOperationError>,
{
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
