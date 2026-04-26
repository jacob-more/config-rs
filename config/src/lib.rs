use std::{
    collections::HashMap,
    convert::Infallible,
    ffi::OsStr,
    fmt::{Debug, Display},
    os::unix::ffi::OsStrExt,
};

use bytes::Bytes;
use thiserror::Error;

pub mod ext;
pub mod parse;

mod cval;
mod fmt;
pub(crate) mod header;
pub(crate) mod history;
mod key;

mod access_control_list;
mod list;
mod set;
mod value;

pub use cval::*;
pub use fmt::*;
pub use key::*;

pub use access_control_list::*;
pub use list::*;
pub use set::*;
pub use value::*;

pub mod derive {
    pub use bytes::Bytes;
    pub use config_derive::*;
}

use crate::{
    ext::IterJoin,
    parse::{AstOperation, ParseError, Parser, RawEntry, RawGroup, RawOperation},
};
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
impl From<Infallible> for ReprParseConfigOperationError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
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
impl From<Infallible> for ConfigParseOperationError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Error)]
pub enum ConfigParseError {
    #[error("{} is not a valid key", .0.display_key())]
    UnknownKey(RawEntry),
    #[error("{} is not a valid group key although it might be a valid operation key", .0.display_key())]
    UnknownGroupKey(RawEntry),
    #[error("{} is not a valid operation key although it might be a valid group key", .0.display_key())]
    UnknownOperationKey(RawEntry),
    #[error(transparent)]
    Group(#[from] ConfigParseGroupError),
    #[error(transparent)]
    Operation(#[from] ConfigParseOperationError),
}
impl From<Infallible> for ConfigParseError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Error)]
pub enum ConfigParseGroupError {
    #[error("{} is not a valid key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownKey { group: Bytes, entry: RawEntry },
    #[error("{} is not a valid group key although it might be a valid operation key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownGroupKey { group: Bytes, entry: RawEntry },
    #[error("{} is not a valid operation key although it might be a valid group key for group {}", .entry.display_key(), OsStr::from_bytes(.group).display())]
    UnknownOperationKey { group: Bytes, entry: RawEntry },
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
impl From<Infallible> for ConfigParseGroupError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Error)]
pub enum ConfigParseBytesError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Config(#[from] ConfigParseError),
}
impl From<ParseError> for Box<ConfigParseBytesError> {
    fn from(value: ParseError) -> Self {
        Box::new(ConfigParseBytesError::Parse(value))
    }
}
impl From<Box<ConfigParseError>> for Box<ConfigParseBytesError> {
    fn from(value: Box<ConfigParseError>) -> Self {
        Box::new(ConfigParseBytesError::Config(*value))
    }
}
impl From<Infallible> for ConfigParseBytesError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[derive(Debug, Error)]
pub enum ConfigParseIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Config(#[from] ConfigParseError),
}
impl From<std::io::Error> for Box<ConfigParseIoError> {
    fn from(value: std::io::Error) -> Self {
        Box::new(ConfigParseIoError::Io(value))
    }
}
impl From<ParseError> for Box<ConfigParseIoError> {
    fn from(value: ParseError) -> Self {
        Box::new(ConfigParseIoError::Parse(value))
    }
}
impl From<Box<ConfigParseError>> for Box<ConfigParseIoError> {
    fn from(value: Box<ConfigParseError>) -> Self {
        Box::new(ConfigParseIoError::Config(*value))
    }
}
impl From<Infallible> for ConfigParseIoError {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

pub trait Config {
    type Err;

    fn parse(&mut self, entries: impl Iterator<Item = RawEntry>) -> Result<(), Self::Err> {
        for entry in entries {
            self.parse_entry(entry)?;
        }
        Ok(())
    }
    fn parse_entry(&mut self, entry: RawEntry) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
    fn display(&self, fmt: ConfigFmt) -> impl Display;
}

pub trait ConfigExt<E>: Config<Err = E>
where
    ConfigParseBytesError: From<E>,
    ConfigParseIoError: From<E>,
{
    fn parse_bytes(&mut self, bytes: Bytes) -> Result<(), ConfigParseBytesError> {
        self.parse(Parser::new().parse_bytes(bytes)?)?;
        Ok(())
    }

    fn parse_reader<R>(&mut self, reader: R) -> Result<(), ConfigParseIoError>
    where
        R: std::io::Read,
    {
        self.parse(Parser::new().parse_reader(reader)??)?;
        Ok(())
    }

    fn parse_file<P>(&mut self, path: P) -> Result<(), ConfigParseIoError>
    where
        P: AsRef<std::path::Path>,
    {
        self.parse(Parser::new().parse_file(path)??)?;
        Ok(())
    }

    fn from_bytes(bytes: Bytes) -> Result<Self, ConfigParseBytesError>
    where
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_bytes(bytes)?;
        Ok(config)
    }

    fn from_reader<R>(reader: R) -> Result<Self, ConfigParseIoError>
    where
        R: std::io::Read,
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_reader(reader)?;
        Ok(config)
    }

    fn from_file<P>(path: P) -> Result<Self, ConfigParseIoError>
    where
        P: AsRef<std::path::Path>,
        Self: Default,
    {
        let mut config = Self::default();
        config.parse_file(path)?;
        Ok(config)
    }
}
impl<C, E> ConfigExt<E> for C
where
    C: Config<Err = E>,
    ConfigParseBytesError: From<E>,
    ConfigParseIoError: From<E>,
{
}

pub trait ConfigGroup {
    type Err;

    fn new(key: Key) -> Self;
    fn parse_group(&mut self, key: Key, body: RawGroup) -> Result<(), Self::Err> {
        for entry in body.0 {
            self.parse_entry(&key, RawEntry::new(entry))?;
        }
        Ok(())
    }
    fn parse_entry(&mut self, key: &Key, body: RawEntry) -> Result<(), Self::Err>;
    fn replay(&mut self, other: &Self);
    fn display(&self, fmt: ConfigFmt) -> impl Display;
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
    fn display(&self, fmt: ConfigFmt) -> impl Display
    where
        Cval<T>: Display;
}

pub trait ConfigOperationExt<T, E>: ConfigOperation<T>
where
    T: ICval,
    Cval<T>: TryFrom<bytes::Bytes, Error = E>,
    ConfigParseOperationError: From<E>,
{
    fn parse_entry(
        &mut self,
        key: Key,
        body: RawOperation,
    ) -> Result<(), ConfigParseOperationError> {
        let _ = key; // key is unused here, but required for the trait.

        match body.0 {
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
impl<C, T, E> ConfigOperationExt<T, E> for C
where
    C: ConfigOperation<T>,
    T: ICval,
    Cval<T>: TryFrom<bytes::Bytes, Error = E>,
    ConfigParseOperationError: From<E>,
{
}

impl<C> Config for HashMap<Key, C>
where
    C: ConfigGroup<Err = ConfigParseError>,
{
    type Err = ConfigParseError;

    fn parse_entry(&mut self, entry: RawEntry) -> Result<(), Self::Err> {
        match entry {
            RawEntry::Group { key, body } => {
                let key = Key::from(key);
                self.entry(key.clone())
                    .or_insert_with(|| ConfigGroup::new(key.clone()))
                    .parse_group(key, body)?;
            }
            RawEntry::Operation { key, body } => {
                return Err(ConfigParseError::UnknownOperationKey(RawEntry::Operation {
                    key,
                    body,
                }));
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

    fn display(&self, fmt: ConfigFmt) -> impl Display {
        std::fmt::from_fn(move |f| {
            write!(
                f,
                "{}",
                self.values()
                    .map(|group| group.display(fmt.clone()))
                    .join('\n')
            )
        })
    }
}

impl<C> ConfigGroup for HashMap<Key, C>
where
    C: ConfigGroup<Err = ConfigParseGroupError>,
{
    type Err = ConfigParseGroupError;

    fn new(_key: Key) -> Self {
        Self::default()
    }

    fn parse_group(&mut self, key: Key, body: RawGroup) -> Result<(), Self::Err> {
        self.parse_entry(
            &key,
            RawEntry::Group {
                key: key.clone().into_bytes(),
                body,
            },
        )
    }

    fn parse_entry(&mut self, key: &Key, entry: RawEntry) -> Result<(), Self::Err> {
        let parent_group = key;

        match entry {
            RawEntry::Group { key, body } => {
                let key = Key::from(key);
                self.entry(key.clone())
                    .or_insert_with(|| ConfigGroup::new(key.clone()))
                    .parse_group(key, body)?;
            }
            RawEntry::Operation { key, body } => {
                return Err(ConfigParseGroupError::UnknownOperationKey {
                    group: parent_group.clone().into_bytes(),
                    entry: RawEntry::Operation { key, body },
                });
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

    fn display(&self, fmt: ConfigFmt) -> impl Display {
        std::fmt::from_fn(move |f| {
            write!(
                f,
                "{}",
                self.values()
                    .map(|group| group.display(fmt.clone()))
                    .join('\n')
            )
        })
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::LazyLock};

    use rstest::rstest;

    use crate::parse::Parser;

    static EXAMPLES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        examples_path.push("benches");
        examples_path.push("examples");
        examples_path
    });

    #[rstest]
    #[case("cargo.lock.conf")]
    #[case("comments.conf")]
    #[case("empty.conf")]
    #[case("root_hints.conf")]
    #[case("short_cargo.lock.conf")]
    fn test_parse_to_entries(#[case] file_name: &str) {
        let mut config_path = EXAMPLES_DIRECTORY.clone();
        config_path.push("config_name");

        let ast = Parser::new()
            .parse_file(config_path.with_file_name(file_name))
            .unwrap();
        assert!(ast.is_ok(), "AST is not Ok: {ast:?}");
    }
}
