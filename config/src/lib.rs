use std::fmt::Debug;

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

use crate::ast::AstOperation;

#[derive(Debug, Error)]
enum ReprConfigParseError {
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
pub struct ConfigParseError(#[from] ReprConfigParseError);
macro_rules! impl_from_config_parse_error {
    ($ty:ty) => {
        impl From<$ty> for ConfigParseError {
            fn from(value: $ty) -> Self {
                ConfigParseError(ReprConfigParseError::from(value))
            }
        }
    };
}
impl_from_config_parse_error!(std::num::ParseIntError);
impl_from_config_parse_error!(std::num::ParseFloatError);
impl_from_config_parse_error!(std::str::Utf8Error);
impl_from_config_parse_error!(std::net::AddrParseError);

pub trait Config<T>
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

pub trait ConfigExt<T>: Config<T>
where
    T: Replayable,
    Conf<T>: TryFrom<bytes::Bytes, Error = ConfigParseError>,
{
    fn parse_ast_entry(&mut self, operation: AstOperation) -> Result<(), ConfigParseError> {
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
impl<C, T> ConfigExt<T> for C
where
    C: Config<T>,
    T: Replayable,
    Conf<T>: TryFrom<bytes::Bytes, Error = ConfigParseError>,
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
