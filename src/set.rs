use std::{collections::HashSet, hash::Hash};

use crate::{Config, ReplayOperation, Replayable, header::ConfigHeader};

pub struct ConfigSet<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<T::Repr>,
    set: HashSet<T::Repr>,
}

impl<T> ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    pub fn new(key: &'static str, default: &[T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| x.unparse_value()).collect();
        Self {
            header: ConfigHeader::new(key),
            set: HashSet::from_iter(default.iter().cloned()),
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.set.iter().map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.set.clear();
        self.set.insert(value);
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.set.insert(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.set.insert(value);
    }

    fn remove(&mut self, value: T::Repr) {
        if self.set.remove(&value) {
            self.header.set_modified();
        }
        self.header.history_mut().remove(value);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.set.clear();
        self.set.extend(self.default.iter().cloned());
    }

    fn is_default(&self) -> bool {
        self.header.is_default()
    }

    fn is_defined(&self) -> bool {
        !self.set.is_empty()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }
}
