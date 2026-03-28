use crate::{Config, ReplayOperation, Replayable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AclAction {
    Allow,
    Deny,
}

pub struct ConfigAcl<T: Replayable> {
    key: &'static str,
    replay: Vec<ReplayOperation<T>>,
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
            replay: Vec::new(),
            config: default.clone(),
            default,
            is_default: true,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }

    pub fn len(&self) -> usize {
        self.config.len()
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
        self.replay.clear();
        self.replay.push(ReplayOperation::Assign(value.clone()));
        self.config.clear();
        self.config.push((AclAction::Allow, value));
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.replay
                .push(ReplayOperation::AssignIfUndefined(value.clone()));
            self.config.push((AclAction::Allow, value));
            self.is_default = false;
        } else {
            self.replay.push(ReplayOperation::AssignIfUndefined(value));
        }
    }

    fn add(&mut self, value: T::Repr) {
        self.replay.push(ReplayOperation::Add(value.clone()));
        // The new action takes precedence over any exact duplicates.
        self.config.retain(|(_, x)| x != &value);
        self.config.push((AclAction::Allow, value));
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        self.replay.push(ReplayOperation::Remove(value.clone()));
        // The new action takes precedence over any exact duplicates.
        self.config.retain(|(_, x)| x != &value);
        self.config.push((AclAction::Deny, value));
        self.is_default = false;
    }

    fn reset(&mut self) {
        self.replay.clear();
        self.replay.push(ReplayOperation::Reset);
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
        self.replay.iter()
    }
}
