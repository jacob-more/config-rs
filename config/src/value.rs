use crate::{Config, ReplayOperation, Replayable, history::History};

pub struct ConfigValue<T: Replayable> {
    key: &'static str,
    history: History<T>,
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
            history: History::new(),
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
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.history.assign(value.clone());
        self.config = Some(value);
        self.is_default = false;
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.config = Some(value.clone());
            self.is_default = false;
        }
        self.history.assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.history.add(value.clone());
        self.config = Some(value);
        self.is_default = false;
    }

    fn remove(&mut self, value: T::Repr) {
        if self.config.take_if(|x| x == &value).is_some() {
            self.is_default = false;
        }
        self.history.remove(value.clone());
    }

    fn reset(&mut self) {
        self.history.reset();
        self.config = self.default.clone();
        self.is_default = true;
    }

    fn is_default(&self) -> bool {
        self.is_default
    }

    fn is_defined(&self) -> bool {
        self.config.is_some()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.history.history()
    }
}
