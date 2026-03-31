use crate::{Conf, Config, ReplayOperation, Replayable, header::ConfigHeader};

#[derive(Debug)]
pub struct ConfigList<T: ?Sized + Replayable> {
    header: ConfigHeader<T>,
    default: Vec<Conf<T>>,
    list: Vec<Conf<T>>,
}

impl<T> ConfigList<T>
where
    T: ?Sized + Replayable,
{
    pub const fn new(key: &'static str) -> Self {
        Self {
            header: ConfigHeader::new(key),
            list: Vec::new(),
            default: Vec::new(),
        }
    }

    pub fn new_with_default(key: &'static str, default: &[&T]) -> Self {
        let default: Vec<_> = default.iter().map(|x| Conf::from(*x)).collect();
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

    pub fn get(&self, index: usize) -> Option<&Conf<T>> {
        self.list.get(index)
    }

    pub fn values(&self) -> impl Iterator<Item = &Conf<T>> {
        self.list.iter()
    }
}

impl<T> Config<T> for ConfigList<T>
where
    T: ?Sized + Replayable + PartialEq,
{
    fn assign(&mut self, value: <T as Replayable>::Repr) {
        self.header.history_mut().assign(value.clone());
        self.header.set_modified();
        self.list.clear();
        self.list.push(Conf(value));
    }

    fn assign_if_undefined(&mut self, value: T::Repr) {
        if !self.is_defined() {
            self.header.set_modified();
            self.list.push(Conf(value.clone()));
        }
        self.header.history_mut().assign_if_undefined(value);
    }

    fn add(&mut self, value: T::Repr) {
        self.header.history_mut().add(value.clone());
        self.header.set_modified();
        self.list.push(Conf(value));
    }

    fn remove(&mut self, value: T::Repr) {
        let conf = Conf(value);
        self.list.retain(|x| {
            let remove = x == &conf;
            if remove {
                self.header.set_modified();
            }
            !remove
        });
        self.header.history_mut().remove(conf.0);
    }

    fn reset(&mut self) {
        self.header.history_mut().reset();
        self.header.set_default();
        self.list.clear();
        self.list.extend(self.default.iter().cloned());
    }

    fn clear(&mut self) {
        self.header.history_mut().clear();
        self.header.set_modified();
        self.list.clear();
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

impl<T> Clone for ConfigList<T>
where
    T: ?Sized + Replayable,
{
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            default: self.default.clone(),
            list: self.list.clone(),
        }
    }
}
