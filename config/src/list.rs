use std::fmt::Display;

use crate::{
    ConfigFmt, ConfigOperation, Cval, ICval, Key, Operation,
    header::ConfigHeader,
    parse::{OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_CLEAR},
};

#[derive(Debug)]
pub struct ConfigList<T: ICval> {
    header: ConfigHeader<T>,
    default: Vec<Cval<T>>,
    list: Vec<Cval<T>>,
}

impl<T> ConfigList<T>
where
    T: ICval,
{
    pub const fn new(key: Key) -> Self {
        Self {
            header: ConfigHeader::new(key),
            list: Vec::new(),
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
            list: default.clone(),
            default,
        }
    }

    pub const fn key(&self) -> &Key {
        self.header.key()
    }

    pub const fn len(&self) -> usize {
        self.list.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Cval<T>> {
        self.list.get(index)
    }

    pub fn values(&self) -> impl Iterator<Item = &Cval<T>> {
        self.list.iter()
    }
}

impl<T> ConfigOperation<T> for ConfigList<T>
where
    T: ICval,
    T::Repr: PartialEq,
{
    fn assign<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.list.clear();
        self.list.push(value);
    }

    fn assign_if_undefined<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        if !self.is_defined() {
            self.header.set_modified();
            self.list.push(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.list.push(value);
    }

    fn remove<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
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

impl<T> Clone for ConfigList<T>
where
    T: ICval,
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
    T: ICval,
    Cval<T>: Display,
    T::Repr: PartialEq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(ConfigFmt::new()))
    }
}
