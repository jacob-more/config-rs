use std::fmt::Display;

use crate::{
    Conf, Config, ReplayOperation, Replayable,
    ast::{OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_CLEAR},
    header::ConfigHeader,
};

#[derive(Debug)]
pub struct ConfigList<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<Conf<T>>,
    list: Vec<Conf<T>>,
}

impl<T> ConfigList<T>
where
    T: Replayable,
{
    pub const fn new(key: &'static str) -> Self {
        Self {
            header: ConfigHeader::new(key),
            list: Vec::new(),
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
            list: default.clone(),
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub const fn len(&self) -> usize {
        self.list.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Conf<T>> {
        self.list.get(index)
    }

    pub fn values(&self) -> impl Iterator<Item = &Conf<T>> {
        self.list.iter()
    }
}

impl<T> Config<T> for ConfigList<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn assign(&mut self, value: Conf<T>) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.list.clear();
        self.list.push(value);
    }

    fn assign_if_undefined(&mut self, value: Conf<T>) {
        if !self.is_defined() {
            self.header.set_modified();
            self.list.push(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: Conf<T>) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.list.push(value);
    }

    fn remove(&mut self, value: Conf<T>) {
        self.list.retain(|x| {
            let remove = x == &value;
            if remove {
                self.header.set_modified();
            }
            !remove
        });
        self.header.history_mut().remove(value);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.list.clear();
        self.list.extend(self.default.iter().cloned());
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_modified();
        self.list.clear();
    }

    fn is_default(&self) -> bool {
        self.header.is_default()
    }

    fn is_defined(&self) -> bool {
        !self.list.is_empty()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }
}

impl<T> Clone for ConfigList<T>
where
    T: Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            list: self.list.clone(),
        }
    }
}

impl<T> Display for ConfigList<T>
where
    T: Replayable,
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
