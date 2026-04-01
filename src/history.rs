use crate::{Conf, ReplayOperation, Replayable};

#[derive(Debug)]
pub struct History<T: Replayable> {
    history: Vec<ReplayOperation<T>>,
}

impl<T> History<T>
where
    T: Replayable,
{
    pub const fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn assign(&mut self, value: Conf<T>) {
        self.history.clear();
        self.history.push(ReplayOperation::Assign(value));
    }

    pub fn assign_if_undefined(&mut self, value: Conf<T>) {
        self.history.push(ReplayOperation::AssignIfUndefined(value));
    }

    pub fn add(&mut self, value: Conf<T>) {
        self.history.push(ReplayOperation::Add(value.clone()));
    }

    pub fn remove(&mut self, value: Conf<T>) {
        self.history.push(ReplayOperation::Remove(value));
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.history.push(ReplayOperation::Reset);
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.history.push(ReplayOperation::Clear);
    }

    pub fn history<'a>(&'a self) -> impl Iterator<Item = &'a ReplayOperation<T>>
    where
        T: 'a,
    {
        self.history.iter()
    }
}

impl<T> Default for History<T>
where
    T: Replayable,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for History<T>
where
    T: Replayable,
{
    fn clone(&self) -> Self {
        Self {
            history: self.history.clone(),
        }
    }
}
