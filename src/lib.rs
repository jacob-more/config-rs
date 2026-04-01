use std::{
    borrow::Borrow, ffi::OsStr, fmt::{Debug, Display}, hash::Hash, net::{IpAddr, Ipv4Addr, Ipv6Addr}, ops::Deref, os::unix::ffi::OsStrExt, path::Path
};

use bytes::Bytes;
use thiserror::Error;

pub mod ast;
pub mod ext;

pub(crate) mod header;
pub(crate) mod history;

mod access_control_list;
mod list;
mod set;
mod value;

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

#[derive(Debug)]
pub struct Conf<T: ?Sized + Replayable>(T::Repr);

impl<T> Clone for Conf<T>
where
    T: ?Sized + Replayable,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AsRef<T> for Conf<T>
where
    T: ?Sized + Replayable,
{
    fn as_ref(&self) -> &T {
        T::parse_value(&self.0)
    }
}

impl<T> Borrow<T> for Conf<T> where T: ?Sized + Replayable {
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> From<&T> for Conf<T>
where
    T: ?Sized + Replayable,
{
    fn from(value: &T) -> Self {
        Self(T::unparse_value(value))
    }
}

impl<T> PartialEq for Conf<T>
where
    T: ?Sized + Replayable + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}
impl<T> Eq for Conf<T> where T: ?Sized + Replayable + Eq {}

impl<T> PartialOrd for Conf<T>
where
    T: ?Sized + Replayable + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}
impl<T> Ord for Conf<T>
where
    T: ?Sized + Replayable + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T> Hash for Conf<T>
where
    T: ?Sized + Replayable + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> Display for Conf<T>
where
    T: ?Sized + Replayable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", T::display_repr(&self.0))
    }
}

pub trait Replayable {
    type Repr: ?Sized + Debug + Clone;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError>;
    fn parse_value(value: &Self::Repr) -> &Self;
    fn unparse_value(&self) -> Self::Repr;
    fn display(&self) -> impl Display;
    fn display_repr(value: &Self::Repr) -> impl Display;
}

impl Replayable for str {
    type Repr = Bytes;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
        // Validates that the underlying bytes are utf8 encoded.
        str::from_utf8(&value)?;
        Ok(value)
    }

    fn parse_value(value: &Self::Repr) -> &Self {
        // TODO: use unchecked interface since we already verified utf8
        // keep until lifetime checks are confirmed.
        str::from_utf8(value).unwrap()
    }

    fn unparse_value(&self) -> Self::Repr {
        self.as_bytes().to_vec().into()
    }

    fn display(&self) -> impl Display {
        self
    }

    fn display_repr(value: &Self::Repr) -> impl Display {
        Self::parse_value(value).display()
    }
}
impl Replayable for Path {
    type Repr = Bytes;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
        Ok(value)
    }

    fn parse_value(value: &Self::Repr) -> &Self {
        Path::new(OsStr::from_bytes(value.deref()))
    }

    fn unparse_value(&self) -> Self::Repr {
        self.as_os_str().as_bytes().to_vec().into()
    }

    fn display(&self) -> impl Display {
        self.display()
    }

    fn display_repr(value: &Self::Repr) -> impl Display {
        Self::parse_value(value).display()
    }
}
impl Replayable for OsStr {
    type Repr = Bytes;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
        Ok(value)
    }

    fn parse_value(value: &Self::Repr) -> &Self {
        OsStr::from_bytes(value.deref())
    }

    fn unparse_value(&self) -> Self::Repr {
        self.as_bytes().to_vec().into()
    }

    fn display(&self) -> impl Display {
        self.display()
    }

    fn display_repr(value: &Self::Repr) -> impl Display {
        Self::parse_value(value).display()
    }
}

const BOOLEAN_TRUE: &[&[u8]] = &[b"true", b"enable", b"yes", b"t", b"y"];
const BOOLEAN_FALSE: &[&[u8]] = &[b"false", b"disable", b"no", b"f", b"n"];

impl Replayable for bool {
    type Repr = bool;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
        if BOOLEAN_TRUE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(true)
        } else if BOOLEAN_FALSE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(false)
        } else {
            Err(ConfigParseError(ReprConfigParseError::ParseBoolean))
        }
    }

    fn parse_value(value: &Self::Repr) -> &Self {
        value
    }

    fn unparse_value(&self) -> Self::Repr {
        *self
    }

    fn display(&self) -> impl Display {
        self
    }

    fn display_repr(value: &Self::Repr) -> impl Display {
        value
    }
}
impl Replayable for char {
    type Repr = char;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
        let value = str::from_utf8(&value)?;
        match (value.chars().next(), value.chars().next()) {
            (Some(character), None) => Ok(character),
            (_, _) => Err(ConfigParseError(ReprConfigParseError::ParseChar)),
        }
    }

    fn parse_value(value: &Self::Repr) -> &Self {
        value
    }

    fn unparse_value(&self) -> Self::Repr {
        *self
    }

    fn display(&self) -> impl Display {
        self
    }

    fn display_repr(value: &Self::Repr) -> impl Display {
        value
    }
}

macro_rules! impl_replayable_integer {
    ($int:ty) => {
        impl Replayable for $int {
            type Repr = $int;

            fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError> {
                Ok(str::from_utf8(&value)?.parse()?)
            }

            fn parse_value(value: &Self::Repr) -> &Self {
                value
            }

            fn unparse_value(&self) -> Self::Repr {
                *self
            }

            fn display(&self) -> impl Display {
                self
            }

            fn display_repr(value: &Self::Repr) -> impl Display {
                value
            }
        }
    };
}

impl_replayable_integer!(u8);
impl_replayable_integer!(u16);
impl_replayable_integer!(u32);
impl_replayable_integer!(u64);
impl_replayable_integer!(u128);
impl_replayable_integer!(usize);

impl_replayable_integer!(i8);
impl_replayable_integer!(i16);
impl_replayable_integer!(i32);
impl_replayable_integer!(i64);
impl_replayable_integer!(i128);
impl_replayable_integer!(isize);

impl_replayable_integer!(f32);
impl_replayable_integer!(f64);

impl_replayable_integer!(IpAddr);
impl_replayable_integer!(Ipv4Addr);
impl_replayable_integer!(Ipv6Addr);

#[derive(Debug)]
pub enum ReplayOperation<T: ?Sized + Replayable> {
    Assign(T::Repr),
    AssignIfUndefined(T::Repr),
    Add(T::Repr),
    Remove(T::Repr),
    Reset,
    Clear,
}
impl<T> Clone for ReplayOperation<T>
where
    T: ?Sized + Replayable,
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

pub trait Config<T>
where
    T: ?Sized + Replayable,
{
    fn assign(&mut self, value: T::Repr);
    fn assign_if_undefined(&mut self, value: T::Repr);
    fn add(&mut self, value: T::Repr);
    fn remove(&mut self, value: T::Repr);
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
    T: ?Sized + Replayable,
{
    fn parse_ast_entry(&mut self, operation: AstOperation) -> Result<(), ConfigParseError> {
        match operation {
            AstOperation::Assign(value) => self.assign(T::pre_parse_value(value)?),
            AstOperation::AssignIfUndefined(value) => {
                self.assign_if_undefined(T::pre_parse_value(value)?)
            }
            AstOperation::Add(value) => self.add(T::pre_parse_value(value)?),
            AstOperation::Remove(value) => self.remove(T::pre_parse_value(value)?),
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
    T: ?Sized + Replayable,
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
