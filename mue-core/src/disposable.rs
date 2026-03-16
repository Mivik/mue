use std::ops::{Deref, DerefMut};

pub trait Disposable {
    fn dispose(&self);

    fn owned(self) -> Owned<Self>
    where
        Self: Sized,
    {
        Owned::new(self)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Owned<T: Disposable>(T);

impl<T: Disposable> Owned<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Disposable> From<T> for Owned<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Disposable> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Disposable> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Disposable> Drop for Owned<T> {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
