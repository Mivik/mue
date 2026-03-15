use std::{any::Any, collections::HashSet, marker::PhantomData, ops::Deref};

use slotmap::new_key_type;

use crate::{effect::EffectId, runtime::Runtime, scope::CURRENT_SCOPE, Disposable};

new_key_type! {
    pub(crate) struct SignalId;
}

pub type Value = Box<dyn Any>;

pub type Comparator = fn(&dyn Any, &dyn Any) -> bool;

pub(crate) struct SignalInner {
    pub value: Option<Value>,
    pub effect: Option<EffectId>,
    pub subscribers: HashSet<EffectId>,
    pub comparator: Option<Comparator>,
}

impl SignalInner {
    pub fn new(value: Option<Value>, comparator: Option<Comparator>) -> Self {
        Self {
            value,
            effect: None,
            subscribers: HashSet::new(),
            comparator,
        }
    }

    pub fn register(self, rt: &Runtime) -> SignalId {
        rt.signals.borrow_mut().insert(self)
    }
}

pub struct ReadSignal<T> {
    pub(crate) id: SignalId,
    _marker: PhantomData<T>,
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ReadSignal<T> {}

pub trait Access {
    type Value;

    fn get(&self) -> Self::Value
    where
        Self::Value: Copy,
    {
        self.get_clone()
    }

    fn get_untracked(&self) -> Self::Value
    where
        Self::Value: Copy,
    {
        self.get_clone_untracked()
    }

    fn get_clone(&self) -> Self::Value
    where
        Self::Value: Clone;

    fn get_clone_untracked(&self) -> Self::Value
    where
        Self::Value: Clone;
}

impl<T> ReadSignal<T> {
    pub(crate) fn new(id: SignalId) -> Self {
        CURRENT_SCOPE.with_borrow_mut(|scope| {
            if let Some(scope) = scope {
                scope.signals.push(id);
            }
        });
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self {
            id: Runtime::with(|rt| rt.null_signal),
            _marker: PhantomData,
        }
    }

    pub fn is_null(&self) -> bool {
        Runtime::with(|rt| self.id == rt.null_signal)
    }

    fn with_inner_mut<R>(self, rt: &Runtime, f: impl FnOnce(&mut SignalInner) -> R) -> R {
        f(&mut rt.signal_mut(self.id))
    }

    pub fn map<U: PartialEq + 'static>(self, mut f: impl FnMut(T) -> U + 'static) -> ReadSignal<U>
    where
        T: Clone + 'static,
    {
        crate::effect::computed(move || f(self.get_clone()))
    }
}

impl<T> Disposable for ReadSignal<T> {
    fn dispose(&self) {
        Runtime::with(|rt| rt.dispose_signal(self.id));
    }
}

impl<T: 'static> Access for ReadSignal<T> {
    type Value = T;

    fn get_clone(&self) -> T
    where
        T: Clone,
    {
        Runtime::with(|rt| {
            rt.track(self.id);
            rt.update_if_necessary(self.id);
            self.with_inner_mut(rt, |inner| {
                inner
                    .value
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<T>()
                    .unwrap()
                    .clone()
            })
        })
    }

    fn get_clone_untracked(&self) -> T
    where
        T: Clone,
    {
        Runtime::with(|rt| {
            rt.update_if_necessary(self.id);
            self.with_inner_mut(rt, |inner| {
                inner
                    .value
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<T>()
                    .unwrap()
                    .clone()
            })
        })
    }
}

#[repr(transparent)]
pub struct Signal<T>(ReadSignal<T>);

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Signal<T> {}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self
    where
        T: PartialEq,
    {
        Self::new_by(value, |a, b| a.downcast_ref::<T>() == b.downcast_ref::<T>())
    }

    pub fn new_by(value: T, comparator: Comparator) -> Self {
        create_signal(SignalInner::new(Some(Box::new(value)), Some(comparator)))
    }

    pub fn shallow(value: T) -> Self {
        create_signal(SignalInner::new(Some(Box::new(value)), None))
    }

    pub fn force_trigger(self) {
        Runtime::with(|rt| {
            rt.on_update(self.id);
        });
    }

    pub fn set_with(self, f: impl FnOnce(&T) -> T) -> bool {
        let updated = self.set_with_untracked(f);
        if updated {
            Runtime::with(|rt| rt.on_update(self.id));
        }
        updated
    }

    pub fn set_with_untracked(self, f: impl FnOnce(&T) -> T) -> bool {
        Runtime::with(|rt| {
            let updated = self.with_inner_mut(rt, |inner| {
                let new_value = f(inner.value.as_ref().unwrap().downcast_ref::<T>().unwrap());
                if inner.comparator.map_or(true, |cmp| {
                    !cmp(inner.value.as_ref().unwrap().as_ref(), &new_value)
                }) {
                    inner.value = Some(Box::new(new_value));
                    true
                } else {
                    false
                }
            });
            updated
        })
    }

    pub fn set(self, value: T) {
        self.set_with(|_| value);
    }

    pub fn set_untracked(self, value: T) {
        self.set_with_untracked(|_| value);
    }

    pub fn update<R>(self, f: impl FnOnce(&mut T) -> R) -> R {
        let result = self.update_untracked(f);
        Runtime::with(|rt| rt.on_update(self.id));
        result
    }

    pub fn update_untracked<R>(self, f: impl FnOnce(&mut T) -> R) -> R {
        Runtime::with(|rt| {
            self.with_inner_mut(rt, |inner| {
                f(inner.value.as_mut().unwrap().downcast_mut::<T>().unwrap())
            })
        })
    }
}

impl<T> Deref for Signal<T> {
    type Target = ReadSignal<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn create_signal<T: 'static>(inner: SignalInner) -> Signal<T> {
    let id = Runtime::with(|rt| inner.register(rt));
    Signal(ReadSignal::new(id))
}

pub fn signal<T: PartialEq + 'static>(value: T) -> Signal<T> {
    Signal::new(value)
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::{prelude::*, runtime::Runtime};

    #[test]
    fn test_signal_basic() {
        let count = signal(0);
        assert_eq!(count.get(), 0);

        count.set(5);
        assert_eq!(count.get(), 5);

        count.set_with(|x| x + 1);
        assert_eq!(count.get(), 6);

        count.update(|x| *x += 1);
        assert_eq!(count.get(), 7);
    }

    #[test]
    fn test_read_signal() {
        let signal = signal(42);
        let read_only: ReadSignal<i32> = *signal;

        assert_eq!(read_only.get(), 42);

        // Can still write through original signal
        signal.set(100);
        assert_eq!(read_only.get(), 100);
    }

    #[test]
    fn test_get_untracked() {
        let count = signal(0);
        let tracked_reads = Rc::new(RefCell::new(0));
        let tracked_reads_clone = tracked_reads.clone();

        watch_effect(move || {
            count.get();
            *tracked_reads_clone.borrow_mut() += 1;
        });

        assert_eq!(*tracked_reads.borrow(), 1);

        // Untracked read should not trigger effect
        let _ = count.get_untracked();
        assert_eq!(*tracked_reads.borrow(), 1);

        // Tracked read inside effect should still work
        count.set(1);
        assert_eq!(*tracked_reads.borrow(), 2);
    }

    #[test]
    fn test_signal_dispose() {
        let count = signal(0);
        let runs = Rc::new(RefCell::new(0));
        let runs_clone = runs.clone();

        let effect = watch_effect(move || {
            count.get();
            *runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*runs.borrow(), 1);

        // Update should trigger effect
        count.set(1);
        assert_eq!(*runs.borrow(), 2);

        // Dispose the effect
        effect.dispose();

        // Update should not trigger effect anymore
        count.set(2);
        assert_eq!(*runs.borrow(), 2);
    }

    #[test]
    fn test_signal_dispose_removes_dependencies() {
        let count = signal(0);

        let effect = watch_effect(move || {
            count.get();
        });

        // Dispose the effect
        effect.dispose();

        // Signal should have no subscribers now
        Runtime::with(|rt| {
            let signals = rt.signals.borrow();
            let signal = signals.get(count.id).unwrap();
            assert!(signal.subscribers.is_empty());
        });
    }

    #[test]
    fn test_force_trigger() {
        let count = signal(0);
        let runs = Rc::new(RefCell::new(0));
        let runs_clone = runs.clone();

        watch_effect(move || {
            count.get();
            *runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*runs.borrow(), 1);

        // Force trigger should run effect even if value doesn't change
        count.force_trigger();
        assert_eq!(*runs.borrow(), 2);
    }
}
