use crate::{Cval, ICval, Operation};

#[derive(Debug)]
pub struct History<T: ICval> {
    history: Vec<Operation<T>>,
}

impl<T> History<T>
where
    T: ICval,
{
    pub const fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn assign(&mut self, value: Cval<T>) {
        self.history.clear();
        self.history.push(Operation::Assign(value));
    }

    pub fn assign_if_undefined(&mut self, value: Cval<T>) {
        self.history.push(Operation::AssignIfUndefined(value));
    }

    pub fn add(&mut self, value: Cval<T>) {
        self.history.push(Operation::Add(value.clone()));
    }

    pub fn remove(&mut self, value: Cval<T>) {
        self.history.push(Operation::Remove(value));
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.history.push(Operation::Reset);
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.history.push(Operation::Clear);
    }

    pub fn history<'a>(&'a self) -> impl Iterator<Item = &'a Operation<T>>
    where
        T: 'a,
    {
        self.history.iter()
    }
}

impl<T> Default for History<T>
where
    T: ICval,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for History<T>
where
    T: ICval,
{
    fn clone(&self) -> Self {
        Self {
            history: self.history.clone(),
        }
    }
}
