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
