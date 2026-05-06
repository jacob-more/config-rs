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
    ops::Deref,
    path::Path,
    time::Duration,
};

use bytes::Bytes;

use crate::ConfigParseEntryError;

mod bool;
mod char;
mod duration;
mod f32;
mod f64;
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
pub struct Cval<T: ?Sized + ICval>(T::Repr)
where
    Self: Sized;

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

macro_rules! impl_icval_from_str {
    ($int:ty $(, @default:$default:expr)? $(,)?) => {
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
            type Error = ConfigParseEntryError;

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
    };
}

impl_icval_from_str!(u8,    @default:u8::default());
impl_icval_from_str!(u16,   @default:u16::default());
impl_icval_from_str!(u32,   @default:u32::default());
impl_icval_from_str!(u64,   @default:u64::default());
impl_icval_from_str!(u128,  @default:u128::default());
impl_icval_from_str!(usize, @default:usize::default());

impl_icval_from_str!(i8,    @default:i8::default());
impl_icval_from_str!(i16,   @default:i16::default());
impl_icval_from_str!(i32,   @default:i32::default());
impl_icval_from_str!(i64,   @default:i64::default());
impl_icval_from_str!(i128,  @default:i128::default());
impl_icval_from_str!(isize, @default:isize::default());

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

impl_icval_from_str!(f32, @default:f32::default());
impl_icval_from_str!(f64, @default:f64::default());

impl_icval_from_str!(IpAddr);
impl_icval_from_str!(Ipv4Addr);
impl_icval_from_str!(Ipv6Addr);
impl_icval_from_str!(SocketAddr);
impl_icval_from_str!(SocketAddrV4);
impl_icval_from_str!(SocketAddrV6);

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

        impl TryFrom<Bytes> for Cval<Option<Cval<$ty>>> {
            type Error = ConfigParseEntryError;

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
            type Error = ConfigParseEntryError;

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

impl_icval_option!(u8);
impl_icval_option!(u16);
impl_icval_option!(u32);
impl_icval_option!(u64);
impl_icval_option!(u128);
impl_icval_option!(usize);

impl_icval_option!(i8);
impl_icval_option!(i16);
impl_icval_option!(i32);
impl_icval_option!(i64);
impl_icval_option!(i128);
impl_icval_option!(isize);

impl_icval_option!(NonZeroU8);
impl_icval_option!(NonZeroU16);
impl_icval_option!(NonZeroU32);
impl_icval_option!(NonZeroU64);
impl_icval_option!(NonZeroU128);
impl_icval_option!(NonZeroUsize);

impl_icval_option!(NonZeroI8);
impl_icval_option!(NonZeroI16);
impl_icval_option!(NonZeroI32);
impl_icval_option!(NonZeroI64);
impl_icval_option!(NonZeroI128);
impl_icval_option!(NonZeroIsize);

impl_icval_option!(f32);
impl_icval_option!(f64);

impl_icval_option!(IpAddr);
impl_icval_option!(Ipv4Addr);
impl_icval_option!(Ipv6Addr);
impl_icval_option!(SocketAddr);
impl_icval_option!(SocketAddrV4);
impl_icval_option!(SocketAddrV6);

macro_rules! impl_icval_traits {
    ($ty:ty) => {
        impl_icval_traits!($ty {}{.0} {&}{.0});
    };
    ($ty:ty {$($pre:tt)*} {$($post:tt)*}) => {
        impl_icval_traits!($ty {$($pre)*}{$($post)*} {$($pre)*}{$($post)*});
    };
    (
        $ty:ty
        {$($pre_self:tt)*} {$($post_self:tt)*}
        {$($pre_other:tt)*} {$($post_other:tt)*}
    ) => {
        impl PartialEq for Cval<$ty> {
            fn eq(&self, other: &Self) -> bool {
                $($pre_self)*self$($post_self)*.eq($($pre_other)*other$($post_other)*)
            }
        }
        impl Eq for Cval<$ty> {}

        impl PartialOrd for Cval<$ty> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Cval<$ty> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                $($pre_self)*self$($post_self)*.cmp($($pre_other)*other$($post_other)*)
            }
        }

        impl Hash for Cval<$ty> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                $($pre_self)*self$($post_self)*.hash(state);
            }
        }
    };
}

impl_icval_traits!(str {}{.deref()});
impl_icval_traits!(OsStr {}{.deref()});
impl_icval_traits!(Path {}{.deref()});

impl_icval_traits!(bool);
impl_icval_traits!(char);
impl_icval_traits!(Duration);

impl_icval_traits!(u8);
impl_icval_traits!(u16);
impl_icval_traits!(u32);
impl_icval_traits!(u64);
impl_icval_traits!(u128);
impl_icval_traits!(usize);

impl_icval_traits!(i8);
impl_icval_traits!(i16);
impl_icval_traits!(i32);
impl_icval_traits!(i64);
impl_icval_traits!(i128);
impl_icval_traits!(isize);

impl_icval_traits!(NonZeroU8);
impl_icval_traits!(NonZeroU16);
impl_icval_traits!(NonZeroU32);
impl_icval_traits!(NonZeroU64);
impl_icval_traits!(NonZeroU128);
impl_icval_traits!(NonZeroUsize);

impl_icval_traits!(NonZeroI8);
impl_icval_traits!(NonZeroI16);
impl_icval_traits!(NonZeroI32);
impl_icval_traits!(NonZeroI64);
impl_icval_traits!(NonZeroI128);
impl_icval_traits!(NonZeroIsize);

impl_icval_traits!(IpAddr);
impl_icval_traits!(Ipv4Addr);
impl_icval_traits!(Ipv6Addr);
impl_icval_traits!(SocketAddr);
impl_icval_traits!(SocketAddrV4);
impl_icval_traits!(SocketAddrV6);

impl_icval_traits!(Option<Cval<str>> {}{.0.as_ref()} {&}{.0.as_ref()});
impl_icval_traits!(Option<Cval<OsStr>> {}{.0.as_ref()} {&}{.0.as_ref()});
impl_icval_traits!(Option<Cval<Path>> {}{.0.as_ref()} {&}{.0.as_ref()});

impl_icval_traits!(Option<bool>);
impl_icval_traits!(Option<char>);
impl_icval_traits!(Option<Duration>);

impl_icval_traits!(Option<u8>);
impl_icval_traits!(Option<u16>);
impl_icval_traits!(Option<u32>);
impl_icval_traits!(Option<u64>);
impl_icval_traits!(Option<u128>);
impl_icval_traits!(Option<usize>);

impl_icval_traits!(Option<i8>);
impl_icval_traits!(Option<i16>);
impl_icval_traits!(Option<i32>);
impl_icval_traits!(Option<i64>);
impl_icval_traits!(Option<i128>);
impl_icval_traits!(Option<isize>);

impl_icval_traits!(Option<NonZeroU8>);
impl_icval_traits!(Option<NonZeroU16>);
impl_icval_traits!(Option<NonZeroU32>);
impl_icval_traits!(Option<NonZeroU64>);
impl_icval_traits!(Option<NonZeroU128>);
impl_icval_traits!(Option<NonZeroUsize>);

impl_icval_traits!(Option<NonZeroI8>);
impl_icval_traits!(Option<NonZeroI16>);
impl_icval_traits!(Option<NonZeroI32>);
impl_icval_traits!(Option<NonZeroI64>);
impl_icval_traits!(Option<NonZeroI128>);
impl_icval_traits!(Option<NonZeroIsize>);

impl_icval_traits!(Option<IpAddr>);
impl_icval_traits!(Option<Ipv4Addr>);
impl_icval_traits!(Option<Ipv6Addr>);
impl_icval_traits!(Option<SocketAddr>);
impl_icval_traits!(Option<SocketAddrV4>);
impl_icval_traits!(Option<SocketAddrV6>);
