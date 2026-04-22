use crate::{ICval, Key, history::History};

#[derive(Debug)]
pub struct ConfigHeader<T: ICval> {
    key: Key,
    history: History<T>,
    is_default: bool,
}

impl<T> ConfigHeader<T>
where
    T: ICval,
{
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            history: History::new(),
            is_default: true,
        }
    }

    pub const fn key(&self) -> &Key {
        &self.key
    }

    pub const fn history(&self) -> &History<T> {
        &self.history
    }

    pub const fn history_mut(&mut self) -> &mut History<T> {
        &mut self.history
    }

    pub const fn set_default(&mut self) {
        self.is_default = true;
    }

    pub const fn set_modified(&mut self) {
        self.is_default = false;
    }

    pub const fn is_default(&self) -> bool {
        self.is_default
    }
}

impl<T> Clone for ConfigHeader<T>
where
    T: ICval,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            history: self.history.clone(),
            is_default: self.is_default,
        }
    }
}
