use crate::{Config, ReplayOperation, Replayable, history::History};

pub struct ConfigList<T: Replayable> {
    key: &'static str,
    history: History<T>,
    default: Vec<T::Repr>,
    config: Vec<T::Repr>,
    is_default: bool,
}

impl<T> ConfigList<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: &[T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| x.unparse_value()).collect();
        Self {
            key,
            history: History::new(),
            config: default.clone(),
            default,
            is_default: true,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }

    pub const fn len(&self) -> usize {
        self.config.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.config.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.config
            .get(index)
            .map(|x| <T as Replayable>::parse_value(x))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.config
            .iter()
            .map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigList<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.history.assign(value.clone());
        self.config.clear();
        self.config.push(value);
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.config.push(value.clone());
            self.is_default = false;
        }
        self.history.assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.history.add(value.clone());
        self.config.push(value);
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        self.config.retain(|x| {
            let remove = x == &value;
            if remove {
                self.is_default = false;
            }
            !remove
        });
        self.history.remove(value);
    }

    fn reset(&mut self) {
        self.history.reset();
        self.config.clear();
        self.config.extend(self.default.iter().cloned());
        self.is_default = true;
    }

    fn is_default(&self) -> bool {
        self.is_default
    }

    fn is_defined(&self) -> bool {
        !self.config.is_empty()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.history.history()
    }
}
