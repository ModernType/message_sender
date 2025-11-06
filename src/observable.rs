use std::cell::UnsafeCell;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct ObservableInner<T> {
    value: T,
    changed: bool,
}

impl<T> ObservableInner<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            changed: true,
        }
    }

    fn is_changed(&self) -> bool {
        self.changed
    }
}

impl<T> AsRef<T> for ObservableInner<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

#[derive(Debug)]
pub struct Observable<T>(UnsafeCell<ObservableInner<T>>);

impl<T: Serialize> Serialize for Observable<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let inner = unsafe { &*self.0.get() };
        inner.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Observable<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = ObservableInner::deserialize(deserializer)?;
        Ok(Self(UnsafeCell::new(inner)))
    }
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Observable(UnsafeCell::new(ObservableInner::new(value)))
    }

    pub fn is_changed(&self) -> bool {
        // SAFETY: as self is borrowed, there can't be any mutable references to inner
        let inner = unsafe { &*self.0.get() };
        inner.is_changed()
    }

    pub fn get_changed(&self) -> Option<&T> {
        // SAFETY: we are mutating only changed state and user won't get mutable access
        let inner = unsafe { &mut *self.0.get() };
        if inner.changed {
            inner.changed = false;
            Some(&inner.value)
        } else {
            None
        }
    }

    pub fn change(&mut self, value: T) {
        let inner = self.0.get_mut();
        inner.value = value;
        inner.changed = true;
    }
}

impl<T> AsRef<T> for Observable<T> {
    fn as_ref(&self) -> &T {
        // SAFETY: as self is borrowed, there can't be any mutable references to inner
        let inner = unsafe { &*self.0.get() };
        &inner.value
    }
}

impl<T: Default> Default for Observable<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}
