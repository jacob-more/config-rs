use crate::{Config, ReplayOperation, Replayable, header::ConfigHeader};

pub struct ConfigValue<T: Replayable> {
    header: ConfigHeader<T>,
    default: Option<T::Repr>,
    value: Option<T::Repr>,
}

impl<T> ConfigValue<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: Option<T>) -> Self {
        let default = default.map(|x| x.unparse_value());
        Self {
            header: ConfigHeader::new(key),
            value: default.clone(),
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub fn value(&self) -> Option<&T> {
        self.value
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
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.value = Some(value);
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.value = Some(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.value = Some(value);
    }

    fn remove(&mut self, value: T::Repr) {
        if self.value.take_if(|x| x == &value).is_some() {
            self.header.set_modified();
        }
        self.header.history_mut().remove(value.clone());
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.value = self.default.clone();
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_modified();
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
