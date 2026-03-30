use crate::{Config, ReplayOperation, Replayable, header::ConfigHeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AclAction {
    Allow,
    Deny,
}

pub struct ConfigAcl<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<(AclAction, T::Repr)>,
    acl: Vec<(AclAction, T::Repr)>,
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

    pub fn get(&self, index: usize) -> Option<(&AclAction, &T)> {
        self.acl
            .get(index)
            .map(|(action, x)| (action, <T as Replayable>::parse_value(x)))
    }

    pub fn values(&self) -> impl Iterator<Item = (&AclAction, &T)> {
        self.acl
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
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.acl.clear();
        self.acl.push((AclAction::Allow, value));
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.acl.push((AclAction::Allow, value.clone()));
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        // The new action takes precedence over any exact duplicates.
        self.acl.retain(|(_, x)| x != &value);
        self.acl.push((AclAction::Allow, value));
    }

    fn remove(&mut self, value: T::Repr) {
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
