use std::fmt::Display;

use bytes::Bytes;

use crate::{ConfigParseOperationError, Cval, ICval, ReprParseConfigOperationError};

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
