use crate::{Config, ReplayEntry, Replayable};

pub struct ConfigList<T: Replayable> {
    key: &'static str,
    replay: Vec<ReplayEntry<T>>,
    default: Vec<T::Repr>,
    config: Vec<T::Repr>,
    is_default: bool,
}

impl<T> ConfigList<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: &[T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| x.unparse_value()).collect();
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

    pub fn get(&self, index: usize) -> Option<&T> {
        self.config
            .get(index)
            .map(|x| <T as Replayable>::parse_value(x))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.config
            .iter()
            .map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigList<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn replay(&mut self, other: &Self) {
        other
            .replay
            .iter()
            .cloned()
            .for_each(|event| self.apply(event));
    }

    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.replay.clear();
        self.replay.push(ReplayEntry::Assign(value.clone()));
        self.config.clear();
        self.config.push(value);
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.replay
                .push(ReplayEntry::AssignIfUndefined(value.clone()));
            self.config.push(value);
            self.is_default = false;
        } else {
            self.replay.push(ReplayEntry::AssignIfUndefined(value));
        }
    }

    fn add(&mut self, value: T::Repr) {
        self.replay.push(ReplayEntry::Add(value.clone()));
        self.config.push(value);
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        self.config.retain(|x| {
            let remove = x == &value;
            if remove {
                self.is_default = false;
            }
            !remove
        });
        self.replay.push(ReplayEntry::Remove(value));
    }

    fn reset(&mut self) {
        self.replay.clear();
        self.replay.push(ReplayEntry::Reset);
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
}
