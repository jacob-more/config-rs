use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{Cval, ICval};

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
