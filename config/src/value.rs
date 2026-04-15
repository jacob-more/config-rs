use std::fmt::Display;

use crate::{
    ConfigFmt, ConfigOperation, Cval, ICval, Operation, ast::OPERATOR_ASSIGN, header::ConfigHeader,
};

#[derive(Debug)]
pub struct ConfigValue<T: ICval> {
    header: ConfigHeader<T>,
    default: Cval<T>,
    value: Option<Cval<T>>,
}

impl<T> ConfigValue<T>
where
    T: ICval,
{
    pub fn new(key: &'static str) -> Self
    where
        T: Default,
        Cval<T>: From<T>,
    {
        Self {
            header: ConfigHeader::new(key),
            value: None,
            default: Cval::from(T::default()),
        }
    }

    pub fn new_with_default<X>(key: &'static str, default: X) -> Self
    where
        Cval<T>: From<X>,
    {
        let default = Cval::from(default);
        Self {
            header: ConfigHeader::new(key),
            value: None,
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub fn value(&self) -> &Cval<T> {
        self.value.as_ref().unwrap_or(&self.default)
    }
}

impl<T> ConfigOperation<T> for ConfigValue<T>
where
    T: ICval,
    T::Repr: PartialEq,
{
    fn assign<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.value = Some(value);
    }

    fn assign_if_undefined<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        if !self.is_defined() {
            self.header.set_modified();
            self.value = Some(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.value = Some(value);
    }

    fn remove<C: Into<Cval<T>>>(&mut self, value: C) {
        let value = value.into();
        if self.value.as_ref().is_some_and(|x| x == &value) {
            self.value = None;
            self.header.set_default();
        }
        self.header.history_mut().remove(value);
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
            write!(
                f,
                "{indent}{} {OPERATOR_ASSIGN} {};",
                self.key(),
                self.value()
            )
        })
    }
}

impl<T> Clone for ConfigValue<T>
where
    T: ICval,
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
    T: ICval,
    Cval<T>: Display,
    T::Repr: PartialEq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(ConfigFmt::new()))
    }
}
