use std::fmt::Display;

pub struct Join<I, S> {
    values: I,
    sep: S,
}

impl<I, S> Join<I, S> {
    pub fn new(values: I, seperator: S) -> Self {
        Self {
            values,
            sep: seperator,
        }
    }
}

impl<I, V, S> Display for Join<I, S>
where
    I: Clone + Iterator<Item = V>,
    V: Display,
    S: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.values.clone();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
            for value in iter {
                write!(f, "{}{value}", self.sep)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use rstest::rstest;

    use crate::ext::Join;

    #[rstest]
    #[case(Join::new(std::iter::empty::<&str>(), ""), "")]
    #[case(Join::new(std::iter::empty::<&str>(), "A"), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 'A'), "")]
    #[case(Join::new(std::iter::empty::<&str>(), true), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_u8), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_u16), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_u32), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_u64), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_u128), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_usize), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_i8), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_i16), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_i32), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_i64), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_i128), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 9_isize), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 3.14_f32), "")]
    #[case(Join::new(std::iter::empty::<&str>(), 3.14_f64), "")]
    fn empty_iter_to_string<T>(#[case] input: T, #[case] output: &str) where T: Display {
        assert_eq!(&input.to_string(), output);
    }

    #[rstest]
    #[case(Join::new(std::iter::once(""), ""), "")]
    #[case(Join::new(std::iter::once(""), "A"), "")]
    #[case(Join::new(std::iter::once("hello"), ""), "hello")]
    #[case(Join::new(std::iter::once(true), "A"), "true")]
    #[case(Join::new(std::iter::once(9_u64), "A"), "9")]
    #[case(Join::new(std::iter::once(3.14_f32), 'A'), "3.14")]
    fn once_iter_to_string<T>(#[case] input: T, #[case] output: &str) where T: Display {
        assert_eq!(&input.to_string(), output);
    }

    #[rstest]
    #[case(Join::new(["hello", "world"].iter(), ""), "helloworld")]
    #[case(Join::new(["hello", "world"].iter(), " "), "hello world")]
    #[case(Join::new(["hello", "world"].iter(), ' '), "hello world")]
    #[case(Join::new(["hello", "world"].iter(), " foo bar "), "hello foo bar world")]
    #[case(Join::new(["hello", "world"].iter(), " foo bar "), "hello foo bar world")]
    #[case(Join::new([3.14_f32, 2.7_f32].iter(), 'A'), "3.14A2.7")]
    fn twice_iter_to_string<T>(#[case] input: T, #[case] output: &str) where T: Display {
        assert_eq!(&input.to_string(), output);
    }

    #[rstest]
    #[case(Join::new(["hello", "again", "world"].iter(), '-'), "hello-again-world")]
    #[case(Join::new([0, 1, 2].iter(), true), "0true1true2")]
    fn thrice_iter_to_string<T>(#[case] input: T, #[case] output: &str) where T: Display {
        assert_eq!(&input.to_string(), output);
    }
}
