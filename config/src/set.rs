use std::{collections::HashSet, fmt::Display, hash::Hash};

use crate::{
    ConfigFmt, ConfigOperation, Cval, ICval, Key, Operation,
    ast::{OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_CLEAR},
    header::ConfigHeader,
};

#[derive(Debug)]
pub struct ConfigSet<T: ICval> {
    header: ConfigHeader<T>,
    default: Vec<Cval<T>>,
    set: HashSet<Cval<T>>,
}

impl<T> ConfigSet<T>
where
    T: ICval,
    T::Repr: Hash + Eq,
{
    pub fn new(key: Key) -> Self {
        Self {
            header: ConfigHeader::new(key),
            set: HashSet::new(),
            default: Vec::new(),
        }
    }

    pub fn new_with_default<'x, X>(key: Key, default: &'x [X]) -> Self
    where
        Cval<T>: From<&'x X>,
    {
        let default: Vec<_> = default.iter().map(Cval::from).collect();
        Self {
            header: ConfigHeader::new(key),
            set: HashSet::from_iter(default.iter().cloned()),
            default,
        }
    }

    pub const fn key(&self) -> &Key {
        self.header.key()
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &Cval<T>> {
        self.set.iter()
    }
}

impl<T> ConfigOperation<T> for ConfigSet<T>
where
    T: ICval,
    T::Repr: Hash + Eq,
{
    fn assign<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.set.clear();
        self.set.insert(value);
    }

    fn assign_if_undefined<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        if !self.is_defined() {
            self.header.set_modified();
            self.set.insert(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.set.insert(value);
    }

    fn remove<C: Into<Cval<T>>>(&mut self, value: C) {
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

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a Operation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }

    fn display(&self, fmt: ConfigFmt) -> impl Display
    where
        Cval<T>: Display,
    {
        std::fmt::from_fn(move |f| {
            let indent = fmt.indent();
            let mut values = self.values();
            match values.next() {
                Some(first) => {
                    write!(f, "{indent}{} {OPERATOR_ASSIGN} {first};", self.key())?;
                    for value in values {
                        write!(f, "\n{indent}{} {OPERATOR_ADD} {value};", self.key())?;
                    }
                    Ok(())
                }
                None => write!(f, "{indent}{} {OPERATOR_CLEAR};", self.key()),
            }
        })
    }
}

impl<T> Clone for ConfigSet<T>
where
    T: ICval,
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
    T: ICval,
    T::Repr: Hash + Eq,
    Cval<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(ConfigFmt::new()))
    }
}
