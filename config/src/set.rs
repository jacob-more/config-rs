use std::{collections::HashSet, hash::Hash};

use crate::{Config, ReplayOperation, Replayable};

pub struct ConfigSet<T: Replayable> {
    key: &'static str,
    replay: Vec<ReplayOperation<T>>,
    default: Vec<T::Repr>,
    config: HashSet<T::Repr>,
    is_default: bool,
}

impl<T> ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    pub fn new(key: &'static str, default: &[T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| x.unparse_value()).collect();
        Self {
            key,
            replay: Vec::new(),
            config: HashSet::from_iter(default.iter().cloned()),
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

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.config
            .iter()
            .map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigSet<T>
where
    T: Replayable,
    T::Repr: Hash + Eq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.replay.clear();
        self.replay.push(ReplayOperation::Assign(value.clone()));
        self.config.clear();
        self.config.insert(value);
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.replay
                .push(ReplayOperation::AssignIfUndefined(value.clone()));
            self.config.insert(value);
            self.is_default = false;
        } else {
            self.replay.push(ReplayOperation::AssignIfUndefined(value));
        }
    }

    fn add(&mut self, value: T::Repr) {
        self.replay.push(ReplayOperation::Add(value.clone()));
        self.config.insert(value);
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        if self.config.remove(&value) {
            self.is_default = false;
        }
        self.replay.push(ReplayOperation::Remove(value));
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
