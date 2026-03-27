use crate::{Config, ReplayEntry, Replayable};

pub struct ConfigValue<T: Replayable> {
    key: &'static str,
    replay: Vec<ReplayEntry<T>>,
    default: Option<T::Repr>,
    config: Option<T::Repr>,
    is_default: bool,
}

impl<T> ConfigValue<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: Option<T>) -> Self {
        let default = default.map(|x| x.unparse_value());
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

    pub fn value(&self) -> Option<&T> {
        self.config
            .as_ref()
            .map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigValue<T>
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
        self.config = Some(value);
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.replay
                .push(ReplayEntry::AssignIfUndefined(value.clone()));
            self.config = Some(value);
            self.is_default = false;
        } else {
            self.replay.push(ReplayEntry::AssignIfUndefined(value));
        }
    }

    fn add(&mut self, value: T::Repr) {
        self.replay.push(ReplayEntry::Add(value.clone()));
        self.config = Some(value);
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        if self.config.take_if(|x| x == &value).is_some() {
            self.is_default = false;
        }
        self.replay.push(ReplayEntry::Remove(value));
    }

    fn reset(&mut self) {
        self.replay.clear();
        self.replay.push(ReplayEntry::Reset);
        self.config = self.default.clone();
        self.is_default = true;
    }

    fn is_default(&self) -> bool {
        self.is_default
    }

    fn is_defined(&self) -> bool {
        self.config.is_some()
    }
}
