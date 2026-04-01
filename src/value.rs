use std::fmt::Display;

use crate::{
    Conf, Config, ReplayOperation, Replayable, ast::OPERATOR_ASSIGN, header::ConfigHeader,
};

#[derive(Debug)]
pub struct ConfigValue<T: ?Sized + Replayable> {
    header: ConfigHeader<T>,
    default: Conf<T>,
    value: Option<Conf<T>>,
}

impl<T> ConfigValue<T>
where
    T: ?Sized + Replayable,
{
    pub fn new(key: &'static str) -> Self
    where
        T: Default,
    {
        Self {
            header: ConfigHeader::new(key),
            value: None,
            default: Conf::from(&T::default()),
        }
    }

    pub fn new_with_default(key: &'static str, default: &T) -> Self {
        let default = Conf::from(default);
        Self {
            header: ConfigHeader::new(key),
            value: None,
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub fn value(&self) -> &Conf<T> {
        self.value.as_ref().unwrap_or(&self.default)
    }
}

impl<T> Config<T> for ConfigValue<T>
where
    T: ?Sized + Replayable + PartialEq,
{
    fn assign(&mut self, value: T::Repr) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.value = Some(Conf(value));
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.value = Some(Conf(value.clone()));
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.value = Some(Conf(value));
    }

    fn remove(&mut self, value: T::Repr) {
        let conf = Conf(value);
        if self.value.as_ref().is_some_and(|x| x == &conf) {
            self.value = None;
            self.header.set_default();
        }
        self.header.history_mut().remove(conf.0);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.value = None;
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_default();
        self.value = None;
    }

    fn is_default(&self) -> bool {
        self.header.is_default()
    }

    fn is_defined(&self) -> bool {
        self.value.is_some()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }
}

impl<T> Clone for ConfigValue<T>
where
    T: ?Sized + Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T> Display for ConfigValue<T>
where
    T: ?Sized + Replayable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {OPERATOR_ASSIGN} {};", self.key(), self.value())
    }
}
