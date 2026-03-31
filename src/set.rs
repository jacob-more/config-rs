use std::{collections::HashSet, hash::Hash};

use crate::{Conf, Config, ReplayOperation, Replayable, header::ConfigHeader};

#[derive(Debug)]
pub struct ConfigSet<T: ?Sized + Replayable> {
    header: ConfigHeader<T>,
    default: Vec<Conf<T>>,
    set: HashSet<Conf<T>>,
}

impl<T> ConfigSet<T>
where
    T: ?Sized + Replayable + Hash + Eq,
{
    pub fn new(key: &'static str) -> Self {
        Self {
            header: ConfigHeader::new(key),
            set: HashSet::new(),
            default: Vec::new(),
        }
    }

    pub fn new_with_default(key: &'static str, default: &[&T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| Conf::from(*x)).collect();
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

    pub fn values(&self) -> impl Iterator<Item = &Conf<T>> {
        self.set.iter()
    }
}

impl<T> Config<T> for ConfigSet<T>
where
    T: ?Sized + Replayable + Hash + Eq,
{
    fn assign(&mut self, value: T::Repr) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.set.clear();
        self.set.insert(Conf(value));
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.set.insert(Conf(value.clone()));
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.set.insert(Conf(value));
    }

    fn remove(&mut self, value: T::Repr) {
        let conf = Conf(value);
        if self.set.remove(&conf) {
            self.header.set_modified();
        }
        self.header.history_mut().remove(conf.0);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.set.clear();
        self.set.extend(self.default.iter().cloned());
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_modified();
        self.set.clear();
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

impl<T> Clone for ConfigSet<T>
where
    T: ?Sized + Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            set: self.set.clone(),
        }
    }
}
