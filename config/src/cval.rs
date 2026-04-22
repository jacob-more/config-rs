use std::{
    borrow::Borrow,
    ffi::{OsStr, OsString},
    fmt::{Debug, Display},
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Deref,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{ConfigParseOperationError, ReprParseConfigOperationError};

/// This trait is implemented on types that can be represented by `Cval` and
/// defines what the internal representation used for that type.
pub trait ICval {
    type Repr: Debug + Clone;
}

/// A type used to represent a single value in configurations. It should be
/// cheaply cloneable and might be reference counted if it requires heap
/// allocation.
#[derive(Debug)]
pub struct Cval<T: ICval>(T::Repr);

impl<T> Cval<T>
where
    T: ICval,
{
    pub(crate) fn into_inner(self) -> T::Repr {
        self.0
    }
}

impl<T> Clone for Cval<T>
where
    T: ICval,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Borrow<T> for Cval<T>
where
    Self: AsRef<T>,
    T: ICval,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> PartialEq for Cval<T>
where
    T: ICval,
    T::Repr: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T> Eq for Cval<T>
where
    T: ICval,
    T::Repr: Eq,
{
}

impl<T> PartialOrd for Cval<T>
where
    T: ICval,
    T::Repr: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T> Ord for Cval<T>
where
    T: ICval,
    T::Repr: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> Hash for Cval<T>
where
    T: ICval,
    T::Repr: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<'a, 'b, A, B> From<&'b mut A> for Cval<B>
where
    Cval<B>: From<&'a A>,
    B: ICval,
    'b: 'a,
{
    fn from(value: &'b mut A) -> Self {
        Self::from(&*value)
    }
}

impl ICval for &str {
    type Repr = Bytes;
}

impl Deref for Cval<&str> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // Safety:
        //
        // > The bytes passed in must be valid UTF-8.
        //
        // The bytes are validated as utf8 when a Cval<&str> is constructed and
        // although Bytes has multiple references, it is immutable so the
        // validity of the utf8 has not changed.
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

impl AsRef<str> for Cval<&str> {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl<T> AsRef<T> for Cval<&str>
where
    for<'a> str: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl TryFrom<&[u8]> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Validates that the underlying bytes are utf8 encoded. Required for
        // later safety guarantees.
        str::from_utf8(value)?;
        Ok(Self(value.to_vec().into()))
    }
}

impl TryFrom<Vec<u8>> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        // try_from(Bytes) validates that the underlying bytes are utf8 encoded.
        // Required for later safety guarantees.
        Self::try_from(Bytes::from(value))
    }
}

impl TryFrom<Bytes> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        // Validates that the underlying bytes are utf8 encoded. Required for
        // later safety guarantees.
        str::from_utf8(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&OsStr> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        // try_from(&[u8]) validates that the underlying bytes are utf8 encoded.
        // Required for later safety guarantees.
        Self::try_from(value.as_encoded_bytes())
    }
}

impl TryFrom<OsString> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        // try_from(Vec<u8>) validates that the underlying bytes are utf8
        // encoded. Required for later safety guarantees.
        Self::try_from(value.into_encoded_bytes())
    }
}

impl TryFrom<Cval<&OsStr>> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Cval<&OsStr>) -> Result<Self, Self::Error> {
        // Validates that the underlying bytes are utf8 encoded. Required for
        // later safety guarantees.
        str::from_utf8(value.deref().as_encoded_bytes())?;
        Ok(Self(value.into_inner()))
    }
}

impl TryFrom<&Path> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        // try_from(&OsStr) validates that the underlying bytes are utf8
        // encoded. Required for later safety guarantees.
        Self::try_from(OsStr::new(value))
    }
}

impl TryFrom<PathBuf> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        // try_from(&OsString) validates that the underlying bytes are utf8
        // encoded. Required for later safety guarantees.
        Self::try_from(OsString::from(value))
    }
}

impl TryFrom<Cval<&Path>> for Cval<&str> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Cval<&Path>) -> Result<Self, Self::Error> {
        // try_from(Cval<&OsStr>) validates that the underlying bytes are utf8
        // encoded. Required for later safety guarantees.
        Self::try_from(<Cval<&OsStr>>::from(value))
    }
}

impl From<&str> for Cval<&str> {
    fn from(value: &str) -> Self {
        // The input is already valid utf8. Safety guarantees for later
        // unchecked cast back into &str are fulfilled.
        value.to_string().into()
    }
}

impl From<String> for Cval<&str> {
    fn from(value: String) -> Self {
        // The input is already valid utf8. Safety guarantees for later
        // unchecked cast into &str are fulfilled.
        Self(value.into_bytes().into())
    }
}

impl Display for Cval<&str> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl ICval for &OsStr {
    type Repr = Bytes;
}

#[cfg(unix)]
impl Cval<&OsStr> {
    pub(crate) const fn from_static(bytes: &'static [u8]) -> Self {
        // On Unix-like systems, this upholds the safety guarantees for the
        // underlying encoding
        Self(Bytes::from_static(bytes))
    }
}

impl Deref for Cval<&OsStr> {
    type Target = OsStr;

    fn deref(&self) -> &Self::Target {
        // Safety:
        //
        // > As the encoding is unspecified, callers must pass in bytes that
        // > originated as a mixture of validated UTF-8 and bytes from
        // > OsStr::as_encoded_bytes from within the same Rust version built for
        // > the same target platform. For example, reconstructing an OsStr from
        // > bytes sent over the network or stored in a file will likely violate
        // > these safety rules.
        // >
        // > Due to the encoding being self-synchronizing, the bytes from
        // > OsStr::as_encoded_bytes can be split either immediately before or
        // > immediately after any valid non-empty UTF-8 substring.
        //
        // Cval does not expose the underlying encoding and all methods that
        // convert into this function first parse from the system-specific
        // encoding.
        unsafe { OsStr::from_encoded_bytes_unchecked(&self.0) }
    }
}

impl AsRef<OsStr> for Cval<&OsStr> {
    fn as_ref(&self) -> &OsStr {
        self.deref()
    }
}

impl<T> AsRef<T> for Cval<&OsStr>
where
    for<'a> OsStr: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl From<&[u8]> for Cval<&OsStr> {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

#[cfg(unix)]
impl From<Vec<u8>> for Cval<&OsStr> {
    fn from(value: Vec<u8>) -> Self {
        use std::os::unix::ffi::OsStringExt;

        // On Unix-like systems, this upholds the safety guarantees for the
        // underlying encoding
        Self::from(OsString::from_vec(value))
    }
}

#[cfg(unix)]
impl From<Bytes> for Cval<&OsStr> {
    fn from(value: Bytes) -> Self {
        // On Unix-like systems, this upholds the safety guarantees for the
        // underlying encoding
        Self(value)
    }
}

impl From<&OsStr> for Cval<&OsStr> {
    fn from(value: &OsStr) -> Self {
        Self::from(value.to_os_string())
    }
}

impl From<OsString> for Cval<&OsStr> {
    fn from(value: OsString) -> Self {
        // This upholds the safety guarantees for the underlying encoding on all
        // platforms
        Self(Bytes::from(value.into_encoded_bytes()))
    }
}

impl From<&str> for Cval<&OsStr> {
    fn from(value: &str) -> Self {
        Self::from(OsStr::new(value))
    }
}

impl From<String> for Cval<&OsStr> {
    fn from(value: String) -> Self {
        Self::from(OsString::from(value))
    }
}

impl From<Cval<&str>> for Cval<&OsStr> {
    fn from(value: Cval<&str>) -> Self {
        // This upholds the safety guarantees for the underlying encoding on all
        // platforms because OsStr encoding is a superset of UTF-8.
        Self(value.into_inner())
    }
}

impl From<&Path> for Cval<&OsStr> {
    fn from(value: &Path) -> Self {
        Self::from(value.as_os_str())
    }
}

impl From<PathBuf> for Cval<&OsStr> {
    fn from(value: PathBuf) -> Self {
        Self::from(OsString::from(value))
    }
}

impl<'a> From<Cval<&'a Path>> for Cval<&'a OsStr> {
    fn from(value: Cval<&'a Path>) -> Self {
        value.into_inner()
    }
}

impl Display for Cval<&OsStr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref().display())
    }
}

impl<'a> ICval for &'a Path {
    type Repr = Cval<&'a OsStr>;
}

impl Deref for Cval<&Path> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        Path::new(self.0.deref())
    }
}

impl AsRef<Path> for Cval<&Path> {
    fn as_ref(&self) -> &Path {
        self.deref()
    }
}

impl<T> AsRef<T> for Cval<&Path>
where
    for<'a> Path: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl From<Bytes> for Cval<&Path> {
    fn from(value: Bytes) -> Self {
        Self(Cval::from(value))
    }
}

impl From<&OsStr> for Cval<&Path> {
    fn from(value: &OsStr) -> Self {
        Self(Cval::from(value))
    }
}

impl From<OsString> for Cval<&Path> {
    fn from(value: OsString) -> Self {
        Self(Cval::from(value))
    }
}

impl<'a> From<Cval<&'a OsStr>> for Cval<&'a Path> {
    fn from(value: Cval<&'a OsStr>) -> Self {
        Self(value)
    }
}

impl From<&Path> for Cval<&Path> {
    fn from(value: &Path) -> Self {
        Self(Cval::from(value))
    }
}

impl From<PathBuf> for Cval<&Path> {
    fn from(value: PathBuf) -> Self {
        Self(Cval::from(value))
    }
}

impl Display for Cval<&Path> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref().display())
    }
}

const BOOLEAN_TRUE: &[&[u8]] = &[b"true", b"enable", b"yes", b"t", b"y"];
const BOOLEAN_FALSE: &[&[u8]] = &[b"false", b"disable", b"no", b"f", b"n"];

impl ICval for bool {
    type Repr = Self;
}

impl AsRef<bool> for Cval<bool> {
    fn as_ref(&self) -> &bool {
        &self.0
    }
}

impl TryFrom<Bytes> for Cval<bool> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if BOOLEAN_TRUE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(Self(true))
        } else if BOOLEAN_FALSE.iter().any(|x| x.eq_ignore_ascii_case(&value)) {
            Ok(Self(false))
        } else {
            Err(ConfigParseOperationError(Box::new(
                ReprParseConfigOperationError::ParseBoolean,
            )))
        }
    }
}

impl From<bool> for Cval<bool> {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<&bool> for Cval<bool> {
    fn from(value: &bool) -> Self {
        Self(*value)
    }
}

impl Display for Cval<bool> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ICval for char {
    type Repr = Self;
}

impl AsRef<char> for Cval<char> {
    fn as_ref(&self) -> &char {
        &self.0
    }
}

impl TryFrom<Bytes> for Cval<char> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let value = str::from_utf8(&value)?;
        match (value.chars().next(), value.chars().next()) {
            (Some(character), None) => Ok(Self(character)),
            (_, _) => Err(ConfigParseOperationError(Box::new(
                ReprParseConfigOperationError::ParseChar,
            ))),
        }
    }
}

impl From<char> for Cval<char> {
    fn from(value: char) -> Self {
        Self(value)
    }
}

impl From<&char> for Cval<char> {
    fn from(value: &char) -> Self {
        Self(*value)
    }
}

impl Display for Cval<char> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! impl_icval_option {
    ($({$lifetime:tt} for)? $ty:ty) => {
        impl$(<$lifetime>)? ICval for Option<$ty> {
            type Repr = Option<Cval<$ty>>;
        }

        impl$(<$lifetime>)? Cval<Option<$ty>> {
            pub fn as_ref(&self) -> Option<&Cval<$ty>> {
                self.0.as_ref()
            }
        }

        impl$(<$lifetime>)? TryFrom<Bytes> for Cval<Option<$ty>> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(Some(<Cval<$ty>>::try_from(value)?)))
            }
        }

        impl$(<$lifetime>)? From<$ty> for Cval<Option<$ty>> {
            fn from(value: $ty) -> Self {
                Self(Some(Cval::from(value)))
            }
        }

        impl$(<$lifetime>)? From<Option<$ty>> for Cval<Option<$ty>> {
            fn from(value: Option<$ty>) -> Self {
                Self(value.map(Cval::from))
            }
        }

        impl$(<$lifetime>)? Display for Cval<Option<$ty>> {
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

impl_icval_option!({'a} for &'a str);
impl_icval_option!({'a} for &'a OsStr);
impl_icval_option!({'a} for &'a Path);
impl_icval_option!(bool);
impl_icval_option!(char);

macro_rules! impl_icval_integer {
    ($int:ty) => {
        impl ICval for $int {
            type Repr = Self;
        }

        impl AsRef<$int> for Cval<$int> {
            fn as_ref(&self) -> &$int {
                &self.0
            }
        }

        impl TryFrom<Bytes> for Cval<$int> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(str::from_utf8(&value)?.parse()?))
            }
        }

        impl From<$int> for Cval<$int> {
            fn from(value: $int) -> Self {
                Self(value)
            }
        }

        impl From<&$int> for Cval<$int> {
            fn from(value: &$int) -> Self {
                Self(*value)
            }
        }

        impl Display for Cval<$int> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl_icval_option!($int);
    };
}

impl_icval_integer!(u8);
impl_icval_integer!(u16);
impl_icval_integer!(u32);
impl_icval_integer!(u64);
impl_icval_integer!(u128);
impl_icval_integer!(usize);

impl_icval_integer!(i8);
impl_icval_integer!(i16);
impl_icval_integer!(i32);
impl_icval_integer!(i64);
impl_icval_integer!(i128);
impl_icval_integer!(isize);

impl_icval_integer!(f32);
impl_icval_integer!(f64);

impl_icval_integer!(IpAddr);
impl_icval_integer!(Ipv4Addr);
impl_icval_integer!(Ipv6Addr);
