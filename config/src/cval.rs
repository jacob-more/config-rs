use std::{
    borrow::Borrow,
    ffi::OsStr,
    fmt::{Debug, Display},
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::Path,
    time::Duration,
};

use bytes::Bytes;

use crate::ConfigParseOperationError;

mod bool;
mod char;
mod duration;
mod osstring;
mod path;
mod string;

pub(crate) use duration::ParseDurationError;

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
impl_icval_option!(Duration);

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
impl_icval_integer!(SocketAddr);
impl_icval_integer!(SocketAddrV4);
impl_icval_integer!(SocketAddrV6);
