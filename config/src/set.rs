use std::{collections::HashSet, fmt::Display, hash::Hash};

use crate::{
    Conf, ConfigOperation, ReplayOperation, Replayable,
    ast::{OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_CLEAR},
    header::ConfigHeader,
};

#[derive(Debug)]
pub struct ConfigSet<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<Conf<T>>,
    set: HashSet<Conf<T>>,
}

impl<T> ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    pub fn new(key: &'static str) -> Self {
        Self {
            header: ConfigHeader::new(key),
            set: HashSet::new(),
            default: Vec::new(),
        }
    }

    pub fn new_with_default<'x, X>(key: &'static str, default: &'x [X]) -> Self
    where
        Conf<T>: From<&'x X>,
    {
        let default: Vec<_> = default.iter().map(Conf::from).collect();
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

impl<T> ConfigOperation<T> for ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    fn assign<C: Into<Conf<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.set.clear();
        self.set.insert(value);
    }

    fn assign_if_undefined<C: Into<Conf<T>>>(&mut self, value: C) {
        let value = value.into();
        if !self.is_defined() {
            self.header.set_modified();
            self.set.insert(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add<C: Into<Conf<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.set.insert(value);
    }

    fn remove<C: Into<Conf<T>>>(&mut self, value: C) {
        let value = value.into();
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
    T: Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            set: self.set.clone(),
        }
    }
}

impl<T> Display for ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
    Conf<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut values = self.values();
        match values.next() {
            Some(first) => {
                write!(f, "{} {OPERATOR_ASSIGN} {first};", self.key())?;
                for value in values {
                    write!(f, " {} {OPERATOR_ADD} {value};", self.key())?;
                }
                Ok(())
            }
            None => write!(f, "{} {OPERATOR_CLEAR};", self.key()),
        }
    }
}
