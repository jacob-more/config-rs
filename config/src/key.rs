use std::{
    borrow::Borrow,
    ffi::{OsStr, OsString},
    fmt::{Debug, Display},
    ops::Deref,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::Cval;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(Cval<&'static OsStr>);

impl Key {
    pub const fn from_static(string: &'static [u8]) -> Self {
        Self(Cval::from_static(string))
    }

    #[cfg(unix)]
    pub(crate) fn into_bytes(self) -> Bytes {
        self.0.into_inner()
    }
}

impl Deref for Key {
    type Target = OsStr;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl AsRef<OsStr> for Key {
    fn as_ref(&self) -> &OsStr {
        self.deref()
    }
}

impl<T> AsRef<T> for Key
where
    for<'a> OsStr: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl<T> Borrow<T> for Key
where
    Self: AsRef<T>,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl From<&[u8]> for Key {
    fn from(value: &[u8]) -> Self {
        Self(Cval::from(value))
    }
}

impl From<Vec<u8>> for Key {
    fn from(value: Vec<u8>) -> Self {
        Self(Cval::from(value))
    }
}

impl From<Bytes> for Key {
    fn from(value: Bytes) -> Self {
        Self(Cval::from(value))
    }
}

impl From<&OsStr> for Key {
    fn from(value: &OsStr) -> Self {
        Self(Cval::from(value))
    }
}

impl From<OsString> for Key {
    fn from(value: OsString) -> Self {
        Self(Cval::from(value))
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Self(Cval::from(value))
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Self(Cval::from(value))
    }
}

impl From<Cval<&str>> for Key {
    fn from(value: Cval<&str>) -> Self {
        Self(Cval::from(value))
    }
}

impl From<&Path> for Key {
    fn from(value: &Path) -> Self {
        Self(Cval::from(value))
    }
}

impl From<PathBuf> for Key {
    fn from(value: PathBuf) -> Self {
        Self(Cval::from(value))
    }
}

impl From<Cval<&'static Path>> for Key {
    fn from(value: Cval<&'static Path>) -> Self {
        Self(Cval::from(value))
    }
}

impl From<Key> for Bytes {
    fn from(value: Key) -> Self {
        value.into_bytes()
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref().display())
    }
}
