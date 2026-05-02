use std::fmt::Display;

use bytes::Bytes;

use crate::{ConfigParseOperationError, Cval, ICval, ReprParseConfigOperationError};

impl ICval for char {
    type Repr = Self;
}

impl Default for Cval<char> {
    fn default() -> Self {
        Self(char::default())
    }
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
