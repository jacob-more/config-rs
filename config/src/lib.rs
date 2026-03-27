use std::{
    ffi::OsStr,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Deref,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use bytes::Bytes;
use config_ir::IrEntry;
use thiserror::Error;

mod access_control_list;
mod list;
mod set;
mod value;

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

pub trait Replayable {
    type Repr: Clone + Sized;

    fn pre_parse_value(value: Bytes) -> Result<Self::Repr, ConfigParseError>;
    fn parse_value(value: &Self::Repr) -> &Self;
    fn unparse_value(&self) -> Self::Repr;
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
pub enum ReplayEntry<T: Replayable> {
    Assign(T::Repr),
    AssignIfUndefined(T::Repr),
    Add(T::Repr),
    Remove(T::Repr),
    Reset,
}
impl<T> Clone for ReplayEntry<T>
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
        }
    }
}

pub trait Config<T>
where
    T: Replayable,
{
    fn parse_ir_entry(&mut self, entry: IrEntry) -> Result<(), ConfigParseError> {
        match entry {
            IrEntry::Group { .. } => panic!("groups must be handled before this parser"),
            IrEntry::Assign { name: _, value } => {
                self.assign(T::pre_parse_value(value)?);
            }
            IrEntry::AssignIfUndefined { name: _, value } => {
                self.assign_if_undefined(T::pre_parse_value(value)?)
            }
            IrEntry::Add { name: _, value } => self.add(T::pre_parse_value(value)?),
            IrEntry::Remove { name: _, value } => {
                self.remove(T::pre_parse_value(value)?);
            }
            IrEntry::Reset { name: _ } => self.reset(),
        }
        Ok(())
    }
    fn apply(&mut self, event: ReplayEntry<T>) {
        match event {
            ReplayEntry::Assign(value) => self.assign(value),
            ReplayEntry::AssignIfUndefined(value) => self.assign_if_undefined(value),
            ReplayEntry::Add(value) => self.add(value),
            ReplayEntry::Remove(value) => self.remove(value),
            ReplayEntry::Reset => self.reset(),
        }
    }
    fn replay(&mut self, other: &Self);

    fn assign(&mut self, value: T::Repr);
    fn assign_if_undefined(&mut self, value: T::Repr);
    fn add(&mut self, value: T::Repr);
    fn remove(&mut self, value: T::Repr);
    fn reset(&mut self);

    fn is_default(&self) -> bool;
    fn is_defined(&self) -> bool;
}
