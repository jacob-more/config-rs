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
    pub use bytes::Bytes;
    pub use config_derive::*;
}

use crate::ast::{Ast, AstEntry, AstGroup, AstOperation, AstParseError};
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
pub struct ConfigParseOperationError(#[from] Box<ReprParseConfigOperationError>);
macro_rules! impl_from_config_parse_error {
    ($ty:ty) => {
        impl From<$ty> for ConfigParseOperationError {
            fn from(value: $ty) -> Self {
                ConfigParseOperationError(Box::new(ReprParseConfigOperationError::from(value)))
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

#[derive(Debug, Error)]
pub enum ConfigParseBytesError {
    #[error(transparent)]
    Ast(#[from] AstParseError),
    #[error(transparent)]
    Config(#[from] ConfigParseError),
}
impl From<AstParseError> for Box<ConfigParseBytesError> {
    fn from(value: AstParseError) -> Self {
        Box::new(ConfigParseBytesError::Ast(value))
    }
}
impl From<Box<ConfigParseError>> for Box<ConfigParseBytesError> {
    fn from(value: Box<ConfigParseError>) -> Self {
        Box::new(ConfigParseBytesError::Config(*value))
    }
}

#[derive(Debug, Error)]
pub enum ConfigParseIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ast(#[from] AstParseError),
    #[error(transparent)]
    Config(#[from] ConfigParseError),
}
impl From<std::io::Error> for Box<ConfigParseIoError> {
    fn from(value: std::io::Error) -> Self {
        Box::new(ConfigParseIoError::Io(value))
    }
}
impl From<AstParseError> for Box<ConfigParseIoError> {
    fn from(value: AstParseError) -> Self {
        Box::new(ConfigParseIoError::Ast(value))
    }
}
impl From<Box<ConfigParseError>> for Box<ConfigParseIoError> {
    fn from(value: Box<ConfigParseError>) -> Self {
        Box::new(ConfigParseIoError::Config(*value))
    }
}

pub trait Config {
    type Err;

    fn parse_ast(&mut self, ast: Ast) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
}

pub trait ConfigExt: Config<Err = Box<ConfigParseError>> {
    fn parse_bytes(&mut self, bytes: Bytes) -> Result<(), Box<ConfigParseBytesError>> {
        let ast = Ast::from_bytes(bytes)?;
        self.parse_ast(ast)?;
        Ok(())
    }

    fn parse_reader<R>(&mut self, reader: R) -> Result<(), Box<ConfigParseIoError>>
    where
        R: std::io::Read,
    {
        let ast = Ast::from_reader(reader)??;
        self.parse_ast(ast)?;
        Ok(())
    }

    fn parse_file<P>(&mut self, path: P) -> Result<(), Box<ConfigParseIoError>>
    where
        P: AsRef<std::path::Path>,
    {
        let file = std::fs::File::open(path)?;
        let ast = Ast::from_reader(file)??;
        self.parse_ast(ast)?;
        Ok(())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, Box<ConfigParseBytesError>>
    where
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_bytes(bytes)?;
        Ok(config)
    }

    fn from_reader<R>(reader: R) -> Result<Self, Box<ConfigParseIoError>>
    where
        R: std::io::Read,
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_reader(reader)?;
        Ok(config)
    }

    fn from_file<P>(path: P) -> Result<Self, Box<ConfigParseIoError>>
    where
        P: AsRef<std::path::Path>,
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_file(path)?;
        Ok(config)
    }
}
impl<C> ConfigExt for C where C: Config<Err = Box<ConfigParseError>> {}

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

impl<C> Config for HashMap<Bytes, C>
where
    C: ConfigGroup<Err = Box<ConfigParseError>>,
{
    type Err = Box<ConfigParseError>;

    fn parse_ast(&mut self, ast: Ast) -> Result<(), Self::Err> {
        for entry in ast.into_entries() {
            match entry {
                AstEntry::Group { key, group } => {
                    self.entry(key.clone())
                        .or_insert_with(|| ConfigGroup::new(key.clone()))
                        .parse_ast_group(key, group)?;
                }
                AstEntry::Operation { key, operation } => {
                    return Err(Box::new(ConfigParseError::UnknownOperationKey(
                        AstEntry::Operation { key, operation },
                    )));
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

impl<C> ConfigGroup for HashMap<Bytes, C>
where
    C: ConfigGroup<Err = Box<ConfigParseGroupError>>,
{
    type Err = Box<ConfigParseGroupError>;

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
                    return Err(Box::new(ConfigParseGroupError::UnknownOperationKey {
                        group: parent_group,
                        entry: AstEntry::Operation { key, operation },
                    }));
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
        let ast = crate::ast::Ast::from_bytes(file_data);
        assert!(ast.is_ok(), "AST is not Ok: {ast:?}");
    }
}
