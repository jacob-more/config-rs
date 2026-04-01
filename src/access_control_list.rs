use std::fmt::Display;

use crate::{
    Conf, Config, ReplayOperation, Replayable,
    ast::{OPERATOR_ADD, OPERATOR_ASSIGN, OPERATOR_CLEAR, OPERATOR_REMOVE},
    header::ConfigHeader,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AclAction {
    Allow,
    Deny,
}

#[derive(Debug)]
pub struct ConfigAcl<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<(AclAction, Conf<T>)>,
    acl: Vec<(AclAction, Conf<T>)>,
}

impl<T> ConfigAcl<T>
where
    T: Replayable,
{
    pub const fn new(key: &'static str) -> Self {
        Self {
            header: ConfigHeader::new(key),
            acl: Vec::new(),
            default: Vec::new(),
        }
    }

    pub fn new_with_default<'x, X>(key: &'static str, default: &'x [(AclAction, X)]) -> Self
    where
        Conf<T>: From<&'x X>,
    {
        let default: Vec<_> = default
            .iter()
            .map(|(action, x)| (*action, Conf::from(x)))
            .collect();
        Self {
            header: ConfigHeader::new(key),
            acl: default.clone(),
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub const fn len(&self) -> usize {
        self.acl.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.acl.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<(&AclAction, &Conf<T>)> {
        self.acl.get(index).map(|(action, x)| (action, x))
    }

    pub fn values(&self) -> impl Iterator<Item = (&AclAction, &Conf<T>)> {
        self.acl.iter().map(|(action, x)| (action, x))
    }

    pub fn allowed(&self) -> impl Iterator<Item = &Conf<T>> {
        self.acl
            .iter()
            .filter(|(action, _)| matches!(action, AclAction::Allow))
            .map(|(_, x)| x)
    }

    pub fn denied(&self) -> impl Iterator<Item = &Conf<T>> {
        self.acl
            .iter()
            .filter(|(action, _)| matches!(action, AclAction::Deny))
            .map(|(_, x)| x)
    }
}

impl<T> Config<T> for ConfigAcl<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn assign(&mut self, value: Conf<T>) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.acl.clear();
        self.acl.push((AclAction::Allow, value));
    }

    fn assign_if_undefined(&mut self, value: Conf<T>) {
        if !self.is_defined() {
            self.header.set_modified();
            self.acl.push((AclAction::Allow, value.clone()));
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: Conf<T>) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        // The new action takes precedence over any exact duplicates.
        self.acl.retain(|(_, x)| x != &value);
        self.acl.push((AclAction::Allow, value));
    }

    fn remove(&mut self, value: Conf<T>) {
        self.header.history_mut().remove(value.clone());
        self.header.set_modified();
        // The new action takes precedence over any exact duplicates.
        self.acl.retain(|(_, x)| x != &value);
        self.acl.push((AclAction::Deny, value));
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.acl.clear();
        self.acl.extend(self.default.iter().cloned());
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_modified();
        self.acl.clear();
    }

    fn is_default(&self) -> bool {
        self.header.is_default()
    }

    fn is_defined(&self) -> bool {
        !self.acl.is_empty()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }
}

impl<T> Clone for ConfigAcl<T>
where
    T: Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            acl: self.acl.clone(),
        }
    }
}

impl<T> Display for ConfigAcl<T>
where
    T: Replayable,
    Conf<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut values = self.values();
        match values.next() {
            Some((first_action, first_value)) => {
                match first_action {
                    AclAction::Allow => {
                        write!(f, "{} {OPERATOR_ASSIGN} {first_value};", self.key())?
                    }
                    AclAction::Deny => {
                        write!(f, "{} {OPERATOR_REMOVE} {first_value};", self.key())?
                    }
                }
                for (action, value) in values {
                    match action {
                        AclAction::Allow => write!(f, "{} {OPERATOR_ADD} {value};", self.key())?,
                        AclAction::Deny => write!(f, "{} {OPERATOR_REMOVE} {value};", self.key())?,
                    }
                }
                Ok(())
            }
            None => write!(f, "{} {OPERATOR_CLEAR};", self.key()),
        }
    }
}
