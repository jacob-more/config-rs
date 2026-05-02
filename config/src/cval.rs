use std::{
    borrow::Borrow,
    ffi::OsStr,
    fmt::{Debug, Display},
    hash::Hash,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
    },
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
pub struct Cval<T: ?Sized + ICval>(T::Repr);

impl<T> Debug for Cval<T>
where
    T: ?Sized + ICval,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Cval").field(&self.0).finish()
    }
}

impl<T> Cval<T>
where
    T: ?Sized + ICval,
{
    pub(crate) fn into_inner(self) -> T::Repr {
        self.0
    }
}

impl<T> Clone for Cval<T>
where
    T: ?Sized + ICval,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Borrow<T> for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> PartialEq for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}
impl<T> Eq for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval + Eq,
{
}

impl<T> PartialOrd for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}
impl<T> Ord for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T> Hash for Cval<T>
where
    Self: AsRef<T>,
    T: ?Sized + ICval + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<'a, 'b, A, B> From<&'b mut A> for Cval<B>
where
    Cval<B>: From<&'a A>,
    B: ?Sized + ICval,
    'b: 'a,
{
    fn from(value: &'b mut A) -> Self {
        Self::from(&*value)
    }
}

macro_rules! impl_icval_option {
    (Cval<$ty:ty>) => {
        impl ICval for Option<Cval<$ty>> {
            type Repr = Option<Cval<$ty>>;
        }

        impl Default for Cval<Option<Cval<$ty>>> {
            fn default() -> Self {
                Self(None)
            }
        }

        impl Cval<Option<Cval<$ty>>> {
            pub fn as_ref(&self) -> Option<&Cval<$ty>> {
                self.0.as_ref()
            }
        }

        impl AsRef<Option<Cval<$ty>>> for Cval<Option<Cval<$ty>>> {
            fn as_ref(&self) -> &Option<Cval<$ty>> {
                &self.0
            }
        }

        impl TryFrom<Bytes> for Cval<Option<Cval<$ty>>> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(Some(<Cval<$ty>>::try_from(value)?)))
            }
        }

        impl From<&$ty> for Cval<Option<Cval<$ty>>> {
            fn from(value: &$ty) -> Self {
                Self(Some(Cval::from(value)))
            }
        }

        impl From<Option<&$ty>> for Cval<Option<Cval<$ty>>> {
            fn from(value: Option<&$ty>) -> Self {
                Self(value.map(Cval::from))
            }
        }

        impl From<Cval<$ty>> for Cval<Option<Cval<$ty>>> {
            fn from(value: Cval<$ty>) -> Self {
                Self(Some(Cval::from(value)))
            }
        }

        impl From<Option<Cval<$ty>>> for Cval<Option<Cval<$ty>>> {
            fn from(value: Option<Cval<$ty>>) -> Self {
                Self(value.map(Cval::from))
            }
        }

        impl Display for Cval<Option<Cval<$ty>>> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if let Some(value) = self.as_ref() {
                    write!(f, "{value}")
                } else {
                    Ok(())
                }
            }
        }
    };
    ($ty:ty) => {
        impl ICval for Option<$ty> {
            type Repr = Option<Cval<$ty>>;
        }

        impl Default for Cval<Option<$ty>> {
            fn default() -> Self {
                Self(None)
            }
        }

        impl Cval<Option<$ty>> {
            pub fn as_ref(&self) -> Option<&Cval<$ty>> {
                self.0.as_ref()
            }
        }

        impl TryFrom<Bytes> for Cval<Option<$ty>> {
            type Error = ConfigParseOperationError;

            fn try_from(value: Bytes) -> Result<Self, Self::Error> {
                Ok(Self(Some(<Cval<$ty>>::try_from(value)?)))
            }
        }

        impl From<$ty> for Cval<Option<$ty>> {
            fn from(value: $ty) -> Self {
                Self(Some(Cval::from(value)))
            }
        }

        impl From<Option<$ty>> for Cval<Option<$ty>> {
            fn from(value: Option<$ty>) -> Self {
                Self(value.map(Cval::from))
            }
        }

        impl Display for Cval<Option<$ty>> {
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

impl_icval_option!(Cval<str>);
impl_icval_option!(Cval<OsStr>);
impl_icval_option!(Cval<Path>);
impl_icval_option!(bool);
impl_icval_option!(char);
impl_icval_option!(Duration);

macro_rules! impl_icval_from_str {
    ($int:ty $(, $default:expr)?) => {
        impl ICval for $int {
            type Repr = Self;
        }

        $(
            impl Default for Cval<$int> {
                fn default() -> Self {
                    Self($default)
                }
            }
        )?

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

impl_icval_from_str!(u8, u8::default());
impl_icval_from_str!(u16, u16::default());
impl_icval_from_str!(u32, u32::default());
impl_icval_from_str!(u64, u64::default());
impl_icval_from_str!(u128, u128::default());
impl_icval_from_str!(usize, usize::default());

impl_icval_from_str!(i8, i8::default());
impl_icval_from_str!(i16, i16::default());
impl_icval_from_str!(i32, i32::default());
impl_icval_from_str!(i64, i64::default());
impl_icval_from_str!(i128, i128::default());
impl_icval_from_str!(isize, isize::default());

impl_icval_from_str!(NonZeroU8);
impl_icval_from_str!(NonZeroU16);
impl_icval_from_str!(NonZeroU32);
impl_icval_from_str!(NonZeroU64);
impl_icval_from_str!(NonZeroU128);
impl_icval_from_str!(NonZeroUsize);

impl_icval_from_str!(NonZeroI8);
impl_icval_from_str!(NonZeroI16);
impl_icval_from_str!(NonZeroI32);
impl_icval_from_str!(NonZeroI64);
impl_icval_from_str!(NonZeroI128);
impl_icval_from_str!(NonZeroIsize);

impl_icval_from_str!(f32, f32::default());
impl_icval_from_str!(f64, f64::default());

impl_icval_from_str!(IpAddr);
impl_icval_from_str!(Ipv4Addr);
impl_icval_from_str!(Ipv6Addr);
impl_icval_from_str!(SocketAddr);
impl_icval_from_str!(SocketAddrV4);
impl_icval_from_str!(SocketAddrV6);
