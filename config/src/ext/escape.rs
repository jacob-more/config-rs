pub trait IterEscaped<I> {
    fn unescaped(self) -> UnescapeBytes<I>
    where
        Self: Sized;
}

impl<'a, I> IterEscaped<I::IntoIter> for I
where
    I: IntoIterator<Item = &'a u8>,
{
    fn unescaped(self) -> UnescapeBytes<I::IntoIter>
    where
        Self: Sized,
    {
        UnescapeBytes::new(self.into_iter())
    }
}

pub struct UnescapeBytes<I>(I);
impl<'a, I> UnescapeBytes<I>
where
    I: Iterator<Item = &'a u8>,
{
    pub fn new(escaped_string: I) -> Self {
        Self(escaped_string)
    }
}
impl<'a, I> Iterator for UnescapeBytes<I>
where
    I: Iterator<Item = &'a u8>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let byte = self.0.next()?;
        if *byte == b'\\' {
            Some(self.0.next().unwrap_or(byte))
        } else {
            Some(byte)
        }
    }
}
