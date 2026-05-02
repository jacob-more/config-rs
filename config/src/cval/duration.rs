use std::{
    ffi::OsStr,
    fmt::Display,
    num::ParseIntError,
    os::unix::ffi::OsStrExt,
    str::{FromStr, Utf8Error},
    sync::LazyLock,
    time::Duration,
};

use bytes::Bytes;
use thiserror::Error;

use crate::{ConfigParseOperationError, Cval, ICval};

#[derive(Debug, Error)]
pub(crate) enum ParseDurationError {
    #[error("duration must follow the format [[hh:]mm:]ss[.NN]")]
    Format,
    #[error(
        "duration must follow the format [[hh:]mm:]ss[.NN] but {} field ('{}') failed to parse: {}",
        .0,
        OsStr::from_bytes(&.1).display(),
        .2,
    )]
    ParseFieldUtf8(&'static str, Bytes, Utf8Error),
    #[error(
        "duration must follow the format [[hh:]mm:]ss[.NN] but {} field ('{}') failed to parse: {}",
        .0,
        OsStr::from_bytes(&.1).display(),
        .2,
    )]
    ParseFieldInt(&'static str, Bytes, ParseIntError),
    #[error(
        "duration '{}' exceeds the maximum duration of {}.{} seconds",
        OsStr::from_bytes(&.0).display(),
        Duration::MAX.as_secs(),
        Duration::MAX.subsec_nanos()
    )]
    FieldOverflows(Bytes),
}

const SECONDS_IN_MINUTE: u64 = 60;
const SECONDS_IN_HOUR: u64 = 60 * SECONDS_IN_MINUTE;
const SECONDS_IN_DAY: u64 = 24 * SECONDS_IN_HOUR;

impl ICval for Duration {
    type Repr = Self;
}

impl Default for Cval<Duration> {
    fn default() -> Self {
        Self(Duration::default())
    }
}

impl AsRef<Duration> for Cval<Duration> {
    fn as_ref(&self) -> &Duration {
        &self.0
    }
}

impl TryFrom<Bytes> for Cval<Duration> {
    type Error = ConfigParseOperationError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        fn parse_field<'h, R>(
            name: &'static str,
            source: &'h Bytes,
            matched: regex::bytes::Match<'h>,
        ) -> Result<R, ParseDurationError>
        where
            R: FromStr<Err = ParseIntError>,
        {
            let matched_str = match str::from_utf8(matched.as_bytes()) {
                Ok(string) => string,
                Err(error) => {
                    return Err(ParseDurationError::ParseFieldUtf8(
                        name,
                        source.slice_ref(matched.as_bytes()),
                        error,
                    ));
                }
            };
            match matched_str.parse() {
                Ok(value) => Ok(value),
                Err(error) => Err(ParseDurationError::ParseFieldInt(
                    name,
                    source.slice_ref(matched.as_bytes()),
                    error,
                )),
            }
        }

        fn checked_normalized_add(unit: u64, multiplier: u64, other_seconds: u64) -> Option<u64> {
            unit.checked_mul(multiplier)
                .and_then(|normalized_seconds| other_seconds.checked_add(normalized_seconds))
        }

        const CAPTURE_DAYS: &str = "days";
        const CAPTURE_HOURS: &str = "hours";
        const CAPTURE_MINUTES: &str = "minutes";
        const CAPTURE_SECONDS: &str = "seconds";
        const CAPTURE_NANOSECONDS: &str = "nanoseconds";
        static DURATION_REGEX: LazyLock<regex::bytes::Regex> = LazyLock::new(|| {
            regex::bytes::Regex::new(&format!(
                r"\A(:?(:?(:?(?<{CAPTURE_DAYS}>[0-9]+):)?(?<{CAPTURE_HOURS}>[0-9]+):)?(?<{CAPTURE_MINUTES}>[0-9]+):)?(?<{CAPTURE_SECONDS}>[0-9]+)(:?\.(?<{CAPTURE_NANOSECONDS}>[0-9]+))?\z"
            )).expect("static regexes must be valid")
        });
        let Some(matched) = DURATION_REGEX.captures(&value) else {
            return Err(ParseDurationError::Format)?;
        };

        let mut seconds: u64 = parse_field(
            CAPTURE_SECONDS,
            &value,
            matched
                .name(CAPTURE_SECONDS)
                .expect("the seconds field is always present in a match"),
        )?;
        let mut subsec_nanos = 0;

        // I have opted to re-implement the duration calculations provided by
        // the standard library so that there are no panics.

        const NANOSECONDS_MODULUS: u128 = 1_000_000_000;

        if let Some(nanoseconds) = matched.name(CAPTURE_NANOSECONDS) {
            let nanoseconds: u128 = parse_field(CAPTURE_NANOSECONDS, &value, nanoseconds)?;
            let add_seconds = nanoseconds / NANOSECONDS_MODULUS;
            let nanoseconds = nanoseconds % NANOSECONDS_MODULUS;
            let Ok(add_seconds) = u64::try_from(add_seconds) else {
                return Err(ParseDurationError::FieldOverflows(value))?;
            };
            let Some(summed_seconds) = add_seconds.checked_add(seconds) else {
                return Err(ParseDurationError::FieldOverflows(value))?;
            };
            seconds = summed_seconds;
            const _: () = assert!(
                NANOSECONDS_MODULUS < u32::MAX as u128,
                "casting to u32 without truncation requires modulus to be smaller than u32::MAX"
            );
            subsec_nanos = nanoseconds as u32;
        }
        if let Some(minutes) = matched.name(CAPTURE_MINUTES) {
            let minutes: u64 = parse_field(CAPTURE_MINUTES, &value, minutes)?;
            let Some(new_seconds) = checked_normalized_add(minutes, SECONDS_IN_MINUTE, seconds)
            else {
                return Err(ParseDurationError::FieldOverflows(value))?;
            };
            seconds = new_seconds;
        }
        if let Some(hours) = matched.name(CAPTURE_HOURS) {
            let hours: u64 = parse_field(CAPTURE_HOURS, &value, hours)?;
            let Some(new_seconds) = checked_normalized_add(hours, SECONDS_IN_HOUR, seconds) else {
                return Err(ParseDurationError::FieldOverflows(value))?;
            };
            seconds = new_seconds;
        }
        if let Some(days) = matched.name(CAPTURE_DAYS) {
            let days: u64 = parse_field(CAPTURE_DAYS, &value, days)?;
            let Some(new_seconds) = checked_normalized_add(days, SECONDS_IN_DAY, seconds) else {
                return Err(ParseDurationError::FieldOverflows(value))?;
            };
            seconds = new_seconds;
        }

        if (seconds > Duration::MAX.as_secs())
            || ((seconds == Duration::MAX.as_secs())
                && (subsec_nanos > Duration::MAX.subsec_nanos()))
        {
            return Err(ParseDurationError::FieldOverflows(value))?;
        }
        Ok(Self(Duration::new(seconds, subsec_nanos)))
    }
}

impl From<Duration> for Cval<Duration> {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<&Duration> for Cval<Duration> {
    fn from(value: &Duration) -> Self {
        Self(*value)
    }
}

impl Display for Cval<Duration> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nanos = self.0.subsec_nanos();
        let seconds = self.0.as_secs() % SECONDS_IN_MINUTE;
        let minutes = self.0.as_secs() % SECONDS_IN_HOUR;
        let hours = self.0.as_secs() % SECONDS_IN_DAY;
        let days = self.0.as_secs() / SECONDS_IN_DAY;
        match (nanos, seconds, minutes, hours, days) {
            (0, seconds, 0, 0, 0) => write!(f, "{seconds}"),
            (0, seconds, minutes, 0, 0) => write!(f, "{minutes}:{seconds}"),
            (0, seconds, minutes, hours, 0) => write!(f, "{hours}:{minutes}:{seconds}"),
            (nanos, seconds, 0, 0, 0) => write!(f, "{seconds}.{nanos}"),
            (nanos, seconds, minutes, 0, 0) => write!(f, "{minutes}:{seconds}.{nanos}"),
            (nanos, seconds, minutes, hours, 0) => write!(f, "{hours}:{minutes}:{seconds}.{nanos}"),
            (nanos, seconds, minutes, hours, days) => {
                write!(f, "{days}:{hours}:{minutes}:{seconds}.{nanos}")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use bytes::Bytes;
    use rstest::rstest;

    use crate::{Cval, ParseDurationError, ReprParseConfigOperationError};

    #[rstest]
    #[case("0", Duration::new(0, 0))]
    #[case("0:0", Duration::new(0, 0))]
    #[case("0:0:0", Duration::new(0, 0))]
    #[case("0:0:0:0", Duration::new(0, 0))]
    #[case("0.0", Duration::new(0, 0))]
    #[case("0:0.0", Duration::new(0, 0))]
    #[case("0:0:0.0", Duration::new(0, 0))]
    #[case("0:0:0:0.0", Duration::new(0, 0))]
    #[case("1", Duration::new(1, 0))]
    #[case("10", Duration::new(10, 0))]
    #[case("59", Duration::new(59, 0))]
    #[case("60", Duration::new(60, 0))]
    #[case("100", Duration::new(100, 0))]
    #[case("18446744073709551615", Duration::new(u64::MAX, 0))]
    #[case("1:2", Duration::new(62, 0))]
    #[case("1:2:3", Duration::new(3723, 0))]
    #[case("1:2:3:4", Duration::new(93784, 0))]
    #[case("1.2", Duration::new(1, 2))]
    #[case("1:2.3", Duration::new(62, 3))]
    #[case("1:2:3.4", Duration::new(3723, 4))]
    #[case("1:2:3:4.5", Duration::new(93784, 5))]
    #[case("1.1000000000", Duration::new(2, 0))]
    #[case("1.1000000001", Duration::new(2, 1))]
    #[case("1.9999999999", Duration::new(10, 999999999))]
    #[case("1:61", Duration::new(121, 0))]
    #[case("1:62:3", Duration::new(7323, 0))]
    #[case("1:25:3:4", Duration::new(176584, 0))]
    #[case("99:25:3:4", Duration::new(8643784, 0))]
    fn parse_success(#[case] duration_string: &'static str, #[case] expected_duration: Duration) {
        let bytes = Bytes::from_static(duration_string.as_bytes());
        let parse_result = <Cval<Duration>>::try_from(bytes);
        assert!(parse_result.is_ok());
        assert_eq!(parse_result.unwrap().as_ref(), &expected_duration);
    }

    #[rstest]
    #[case("a")]
    #[case("abcdefg")]
    #[case("a:b")]
    #[case("a:0")]
    #[case("a0")]
    #[case("0a")]
    #[case("0:a")]
    #[case("0:0.a")]
    #[case("0:0a")]
    #[case("a:0:0a")]
    #[case("a:0:0.a")]
    #[case("0:0:0.a")]
    #[case("a:0:0:0")]
    #[case("0:0:0:0.a")]
    #[case("0:0:0:0.0.a")]
    #[case("0:0:0:0.0a")]
    #[case("a0:0:0:0.0")]
    fn parse_format_error(#[case] duration_string: &'static str) {
        let bytes = Bytes::from_static(duration_string.as_bytes());
        let parse_result = <Cval<Duration>>::try_from(bytes);
        assert!(parse_result.is_err());
        assert!(matches!(
            *parse_result.unwrap_err().0,
            ReprParseConfigOperationError::ParseDuration(ParseDurationError::Format)
        ));
    }

    #[rstest]
    #[case("18446744073709551616")]
    #[case("18446744073709551616:0")]
    #[case("18446744073709551616:0:0")]
    #[case("18446744073709551616:0:0:0")]
    #[case("18446744073709551616.0")]
    #[case("18446744073709551616:0.0")]
    #[case("18446744073709551616:0:0.0")]
    #[case("18446744073709551616:0:0:0.0")]
    fn parse_integer_error(#[case] duration_string: &'static str) {
        let bytes = Bytes::from_static(duration_string.as_bytes());
        let parse_result = <Cval<Duration>>::try_from(bytes);
        assert!(parse_result.is_err());
        assert!(matches!(
            *parse_result.unwrap_err().0,
            ReprParseConfigOperationError::ParseDuration(ParseDurationError::ParseFieldInt(..))
        ));
    }

    #[rstest]
    #[case("1:18446744073709551615")]
    #[case("1:0:18446744073709551615")]
    #[case("1:0:0:18446744073709551615")]
    #[case("18446744073709551615.1000000000")]
    #[case("0.340282366920938463463374607431768211455")]
    #[case("307445734561825861:0")]
    #[case("5124095576030433:0:0")]
    #[case("213503982334603:0:0:0")]
    fn parse_overflow_error(#[case] duration_string: &'static str) {
        let bytes = Bytes::from_static(duration_string.as_bytes());
        let parse_result = <Cval<Duration>>::try_from(bytes);
        assert!(parse_result.is_err());
        assert!(matches!(
            *parse_result.unwrap_err().0,
            ReprParseConfigOperationError::ParseDuration(ParseDurationError::FieldOverflows(..))
        ));
    }
}

#[cfg(test)]
mod property_test {
    use std::time::Duration;

    use bytes::Bytes;
    use proptest::proptest;

    use crate::Cval;

    proptest! {
        #[test]
        fn no_panics(input: Vec<u8>) {
            let _ = <Cval<Duration>>::try_from(Bytes::from(input));
        }
    }
}
