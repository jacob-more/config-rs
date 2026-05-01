use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{Cval, ICval};

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
