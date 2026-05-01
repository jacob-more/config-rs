use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{ConfigParseOperationError, Cval, ICval};

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
