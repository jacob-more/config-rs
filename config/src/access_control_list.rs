use crate::{Config, ReplayOperation, Replayable, history::History};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AclAction {
    Allow,
    Deny,
}

pub struct ConfigAcl<T: Replayable> {
    key: &'static str,
    history: History<T>,
    default: Vec<(AclAction, T::Repr)>,
    config: Vec<(AclAction, T::Repr)>,
    is_default: bool,
}

impl<T> ConfigAcl<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: &[(AclAction, T)]) -> Self {
        let default: Vec<_> = default
            .iter()
            .map(|(action, x)| (*action, x.unparse_value()))
            .collect();
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

    pub fn get(&self, index: usize) -> Option<(&AclAction, &T)> {
        self.config
            .get(index)
            .map(|(action, x)| (action, <T as Replayable>::parse_value(x)))
    }

    pub fn values(&self) -> impl Iterator<Item = (&AclAction, &T)> {
        self.config
            .iter()
            .map(|(action, x)| (action, <T as Replayable>::parse_value(x)))
    }
}

impl<T> Config<T> for ConfigAcl<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.history.assign(value.clone());
        self.config.clear();
        self.config.push((AclAction::Allow, value));
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.config.push((AclAction::Allow, value.clone()));
            self.is_default = false;
        }
        self.history.assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.history.add(value.clone());
        // The new action takes precedence over any exact duplicates.
        self.config.retain(|(_, x)| x != &value);
        self.config.push((AclAction::Allow, value));
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        self.history.remove(value.clone());
        // The new action takes precedence over any exact duplicates.
        self.config.retain(|(_, x)| x != &value);
        self.config.push((AclAction::Deny, value));
        self.is_default = false;
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
