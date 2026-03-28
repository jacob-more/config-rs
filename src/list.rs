use crate::{Config, ReplayOperation, Replayable, header::ConfigHeader};

pub struct ConfigList<T: Replayable> {
    header: ConfigHeader<T>,
    default: Vec<T::Repr>,
    list: Vec<T::Repr>,
}

impl<T> ConfigList<T>
where
    T: Replayable,
{
    pub fn new(key: &'static str, default: &[T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| x.unparse_value()).collect();
        Self {
            header: ConfigHeader::new(key),
            list: default.clone(),
            default,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.header.key()
    }

    pub const fn len(&self) -> usize {
        self.list.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.list
            .get(index)
            .map(|x| <T as Replayable>::parse_value(x))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.list.iter().map(|x| <T as Replayable>::parse_value(x))
    }
}

impl<T> Config<T> for ConfigList<T>
where
    T: Replayable,
    T::Repr: PartialEq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.list.clear();
        self.list.push(value);
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.list.push(value.clone());
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.list.push(value);
    }

    fn remove(&mut self, value: T::Repr) {
        self.list.retain(|x| {
            let remove = x == &value;
            if remove {
                self.header.set_modified();
            }
            !remove
        });
        self.header.history_mut().remove(value);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.list.clear();
        self.list.extend(self.default.iter().cloned());
    }

    fn is_default(&self) -> bool {
        self.header.is_default()
    }

    fn is_defined(&self) -> bool {
        !self.list.is_empty()
    }

    fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.header.history().history()
    }
}
