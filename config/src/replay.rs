use std::{
    borrow::Borrow,
    ffi::{OsStr, OsString},
    fmt::{Debug, Display},
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{ConfigParseOperationError, ReprParseConfigOperationError};

pub trait Replayable {
    type Repr: Debug + Clone;
}

#[derive(Debug)]
pub struct Conf<T: Replayable>(T::Repr);

impl<T> Clone for Conf<T>
where
    T: Replayable,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Borrow<T> for Conf<T>
where
    Self: AsRef<T>,
    T: Replayable,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> PartialEq for Conf<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T> Eq for Conf<T>
where
    T: Replayable,
    T::Repr: Eq,
{
}

impl<T> PartialOrd for Conf<T>
where
    T: Replayable,
    T::Repr: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T> Ord for Conf<T>
where
    T: Replayable,
    T::Repr: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> Hash for Conf<T>
where
    T: Replayable,
    T::Repr: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<'a, 'b, A, B> From<&'b mut A> for Conf<B>
where
    Conf<B>: From<&'a A>,
    B: Replayable,
    'b: 'a,
{
    fn from(value: &'b mut A) -> Self {
        Self::from(&*value)
    }
}

impl Replayable for &str {
    type Repr = Bytes;
}

impl AsRef<str> for Conf<&str> {
    fn as_ref(&self) -> &str {
        // Safety:
        //
        // > The bytes passed in must be valid UTF-8.
        //
        // The bytes are validated as utf8 when a Conf<&str> is constructed and
        // although Bytes has multiple references, it is immutable so the
        // validity of the utf8 has not changed.
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

impl TryFrom<&[u8]> for Conf<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Validates that the underlying bytes are utf8 encoded. Required for
        // later safety guarantees.
        str::from_utf8(value)?;
        Ok(Self(value.to_vec().into()))
    }
}

impl TryFrom<Vec<u8>> for Conf<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        // try_from(Bytes) validates that the underlying bytes are utf8 encoded.
        // Required for later safety guarantees.
        Self::try_from(Bytes::from(value))
    }
}

impl TryFrom<Bytes> for Conf<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        // Validates that the underlying bytes are utf8 encoded. Required for
        // later safety guarantees.
        str::from_utf8(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&OsStr> for Conf<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        // try_from(&[u8]) validates that the underlying bytes are utf8 encoded.
        // Required for later safety guarantees.
        Self::try_from(value.as_bytes())
    }
}

impl TryFrom<OsString> for Conf<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        // try_from(Vec<u8>) validates that the underlying bytes are utf8
        // encoded. Required for later safety guarantees.
        Self::try_from(value.into_vec())
    }
}

impl From<&str> for Conf<&str> {
    fn from(value: &str) -> Self {
        // The input is already valid utf8. Safety guarantees for later
        // unchecked cast back into &str are fulfilled.
        value.to_string().into()
    }
}

impl From<String> for Conf<&str> {
    fn from(value: String) -> Self {
        // The input is already valid utf8. Safety guarantees for later
        // unchecked cast into &str are fulfilled.
        Self(value.into_bytes().into())
    }
}

impl Display for Conf<&str> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl Replayable for &OsStr {
    type Repr = Bytes;
}

impl AsRef<OsStr> for Conf<&OsStr> {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes(&self.0)
    }
}

impl TryFrom<&[u8]> for Conf<&OsStr> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(value.to_vec().into()))
    }
}

impl TryFrom<Vec<u8>> for Conf<&OsStr> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(Bytes::from(value))
    }
}

impl TryFrom<Bytes> for Conf<&OsStr> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<&OsStr> for Conf<&OsStr> {
    fn from(value: &OsStr) -> Self {
        value.to_os_string().into()
    }
}

impl From<OsString> for Conf<&OsStr> {
    fn from(value: OsString) -> Self {
        Self(value.into_vec().into())
    }
}

impl From<&str> for Conf<&OsStr> {
    fn from(value: &str) -> Self {
        Self::from(OsStr::new(value))
    }
}

impl From<String> for Conf<&OsStr> {
    fn from(value: String) -> Self {
        Self::from(OsString::from(value))
    }
}

impl Display for Conf<&OsStr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref().display())
    }
}

impl Replayable for &Path {
    type Repr = Bytes;
}

impl AsRef<Path> for Conf<&Path> {
    fn as_ref(&self) -> &Path {
        Path::new(OsStr::from_bytes(&self.0))
    }
}

impl TryFrom<Bytes> for Conf<&Path> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<&Path> for Conf<&Path> {
    fn from(value: &Path) -> Self {
        value.to_path_buf().into()
    }
}

impl From<PathBuf> for Conf<&Path> {
    fn from(value: PathBuf) -> Self {
        Self(value.into_os_string().into_vec().into())
    }
}

impl Display for Conf<&Path> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref().display())
    }
}

const BOOLEAN_TRUE: &[&[u8]] = &[b"true", b"enable", b"yes", b"t", b"y"];
const BOOLEAN_FALSE: &[&[u8]] = &[b"false", b"disable", b"no", b"f", b"n"];

impl Replayable for bool {
    type Repr = Self;
}

impl AsRef<bool> for Conf<bool> {
    fn as_ref(&self) -> &bool {
        &self.0
    }
}

impl TryFrom<Bytes> for Conf<bool> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if BOOLEAN_TRUE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(Self(true))
        } else if BOOLEAN_FALSE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(Self(false))
        } else {
            Err(ConfigParseOperationError(
                ReprParseConfigOperationError::ParseBoolean,
            ))
        }
    }
}

impl From<bool> for Conf<bool> {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<&bool> for Conf<bool> {
    fn from(value: &bool) -> Self {
        Self(*value)
    }
}

impl Display for Conf<bool> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Replayable for char {
    type Repr = Self;
}

impl AsRef<char> for Conf<char> {
    fn as_ref(&self) -> &char {
        &self.0
    }
}

impl TryFrom<Bytes> for Conf<char> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let value = str::from_utf8(&value)?;
        match (value.chars().next(), value.chars().next()) {
            (Some(character), None) => Ok(Self(character)),
            (_, _) => Err(ConfigParseOperationError(
                ReprParseConfigOperationError::ParseChar,
            )),
        }
    }
}

impl From<char> for Conf<char> {
    fn from(value: char) -> Self {
        Self(value)
    }
}

impl From<&char> for Conf<char> {
    fn from(value: &char) -> Self {
        Self(*value)
    }
}

impl Display for Conf<char> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! impl_replayable_option {
    ($({$lifetime:tt} for)? $ty:ty) => {
        impl$(<$lifetime>)? Replayable for Option<$ty> {
            type Repr = Option<Conf<$ty>>;
        }

        impl$(<$lifetime>)? Conf<Option<$ty>> {
            pub fn as_ref(&self) -> Option<&Conf<$ty>> {
                self.0.as_ref()
            }
        }

        impl$(<$lifetime>)? TryFrom<Bytes> for Conf<Option<$ty>> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(Some(<Conf<$ty>>::try_from(value)?)))
            }
        }

        impl$(<$lifetime>)? From<$ty> for Conf<Option<$ty>> {
            fn from(value: $ty) -> Self {
                Self(Some(Conf::from(value)))
            }
        }

        impl$(<$lifetime>)? From<Option<$ty>> for Conf<Option<$ty>> {
            fn from(value: Option<$ty>) -> Self {
                Self(value.map(Conf::from))
            }
        }

        impl$(<$lifetime>)? Display for Conf<Option<$ty>> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if let Some(value) = self.as_ref() {
                    write!(f, "{value}")
                } else {
                    Ok(())
                }
            }
        }
    };
}

impl_replayable_option!({'a} for &'a str);
impl_replayable_option!({'a} for &'a OsStr);
impl_replayable_option!({'a} for &'a Path);
impl_replayable_option!(bool);
impl_replayable_option!(char);

macro_rules! impl_replayable_integer {
    ($int:ty) => {
        impl Replayable for $int {
            type Repr = Self;
        }

        impl AsRef<$int> for Conf<$int> {
            fn as_ref(&self) -> &$int {
                &self.0
            }
        }

        impl TryFrom<Bytes> for Conf<$int> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(str::from_utf8(&value)?.parse()?))
            }
        }

        impl From<$int> for Conf<$int> {
            fn from(value: $int) -> Self {
                Self(value)
            }
        }

        impl From<&$int> for Conf<$int> {
            fn from(value: &$int) -> Self {
                Self(*value)
            }
        }

        impl Display for Conf<$int> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl_replayable_option!($int);
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
pub enum ReplayOperation<T: Replayable> {
    Assign(Conf<T>),
    AssignIfUndefined(Conf<T>),
    Add(Conf<T>),
    Remove(Conf<T>),
    Reset,
    Clear,
}
impl<T> Clone for ReplayOperation<T>
where
    T: Replayable,
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
