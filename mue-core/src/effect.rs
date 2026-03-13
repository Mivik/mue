use std::{ops::Deref, slice};

use slotmap::{new_key_type, Key};

use crate::{
    runtime::Runtime,
    signal::{ReadSignal, SignalId, SignalInner, Value},
    Prop,
};

new_key_type! {
    /// Unique identifier for an effect in the reactive system.
    /// Can be used to dispose effects created by `watch` or `watch_effect`.
    pub struct EffectId;
}

pub(crate) type EffectCallback = Box<dyn FnMut(&mut Option<Value>) -> bool>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EffectState {
    Clean,
    Check,
    Dirty,
}

pub(crate) enum Dependencies {
    Dynamic(Vec<SignalId>),
    Static(SignalId),
}

impl Default for Dependencies {
    fn default() -> Self {
        Self::Dynamic(Vec::new())
    }
}

impl Deref for Dependencies {
    type Target = [SignalId];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Dynamic(signal_ids) => signal_ids,
            Self::Static(signal_id) => slice::from_ref(signal_id),
        }
    }
}

pub(crate) struct EffectInner {
    pub callback: Option<EffectCallback>,
    pub signal: SignalId,
    pub dependencies: Dependencies,
    pub state: EffectState,
}

impl EffectInner {
    pub fn new(
        callback: EffectCallback,
        signal: SignalId,
        dependencies: Dependencies,
        state: EffectState,
    ) -> Self {
        Self {
            callback: Some(callback),
            signal,
            dependencies,
            state,
        }
    }

    pub fn register(self, rt: &Runtime) -> EffectId {
        rt.effects.borrow_mut().insert(self)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Effect {
    id: EffectId,
}

impl Effect {
    pub(crate) fn new(id: EffectId) -> Self {
        Self { id }
    }

    pub fn force_trigger(&self) {
        Runtime::with(|rt| rt.update(self.id));
    }

    pub fn dispose(self) {
        Runtime::with(|rt| rt.dispose_effect(self.id));
    }
}

pub fn watch<T: 'static>(prop: Prop<T>, mut f: impl FnMut(ReadSignal<T>) + 'static) -> Effect {
    let signal = match prop {
        Prop::Static(_) => {
            // Watch with a static prop doesn't make much sense
            return Effect::new(EffectId::null());
        }
        Prop::Dynamic(signal) => signal,
    };
    Runtime::with(|rt| {
        let effect_id = EffectInner::new(
            Box::new(move |_value| {
                f(signal);
                false
            }),
            rt.null_signal,
            Dependencies::Static(signal.id),
            EffectState::Clean,
        )
        .register(rt);
        rt.signal_mut(signal.id).subscribers.insert(effect_id);
        Effect::new(effect_id)
    })
}

pub fn watch_immediate<T: 'static>(prop: Prop<T>, mut f: impl FnMut(Prop<T>) + 'static) -> Effect {
    let signal = match prop {
        Prop::Static(_) => {
            f(prop);
            return Effect::new(EffectId::null());
        }
        Prop::Dynamic(signal) => signal,
    };
    Runtime::with(|rt| {
        let effect_id = EffectInner::new(
            Box::new(move |_value| {
                f(Prop::Dynamic(signal));
                false
            }),
            rt.null_signal,
            Dependencies::Static(signal.id),
            EffectState::Dirty,
        )
        .register(rt);
        rt.signal_mut(signal.id).subscribers.insert(effect_id);
        let effect = Effect::new(effect_id);
        effect.force_trigger();
        effect
    })
}

pub fn watch_effect(mut f: impl FnMut() + 'static) -> Effect {
    Runtime::with(|rt| {
        let effect_id = EffectInner::new(
            Box::new(move |_value| {
                f();
                false
            }),
            rt.null_signal,
            Dependencies::default(),
            EffectState::Dirty,
        )
        .register(rt);
        rt.update(effect_id);
        Effect::new(effect_id)
    })
}

fn create_computed<T: 'static>(callback: EffectCallback) -> ReadSignal<T> {
    Runtime::with(|rt| {
        let signal_id = SignalInner::new(None, None).register(rt);
        let effect_id = EffectInner::new(
            callback,
            signal_id,
            Dependencies::default(),
            EffectState::Dirty,
        )
        .register(rt);
        rt.signal_mut(signal_id).effect = Some(effect_id);
        ReadSignal::new(signal_id)
    })
}

pub fn computed<T: PartialEq + 'static>(mut f: impl FnMut() -> T + 'static) -> ReadSignal<T> {
    computed_with_previous(move |_| f())
}
pub fn computed_with_previous<T: PartialEq + 'static>(
    mut f: impl FnMut(Option<&T>) -> T + 'static,
) -> ReadSignal<T> {
    create_computed(Box::new(move |value| {
        let new_value = f(value.as_mut().and_then(|v| v.downcast_ref::<T>()));
        if value
            .as_ref()
            .is_some_and(|value| value.downcast_ref::<T>() == Some(&new_value))
        {
            false
        } else {
            *value = Some(Box::new(new_value));
            true
        }
    }))
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::prelude::*;

    #[test]
    fn test_computed_basic() {
        let count = signal(2);
        let doubled = computed(move || count.get() * 2);

        assert_eq!(doubled.get(), 4);

        count.set(5);
        assert_eq!(doubled.get(), 10);
    }

    #[test]
    fn test_effect_tracking() {
        let count = signal(0);
        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        watch_effect(move || {
            count.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*effect_runs.borrow(), 1);

        count.set(1);
        assert_eq!(*effect_runs.borrow(), 2);

        count.set(2);
        assert_eq!(*effect_runs.borrow(), 3);
    }

    #[test]
    fn test_watch() {
        let a = signal(1);
        let b = signal(10);

        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        watch(Prop::Dynamic(*a), move |_a| {
            b.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*effect_runs.borrow(), 0);
        a.set(2);
        assert_eq!(*effect_runs.borrow(), 1);
        b.set(20);
        assert_eq!(*effect_runs.borrow(), 1);
    }

    #[test]
    fn test_watch_prop() {
        let a = signal(1);
        let b = signal(10);

        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        watch_immediate(Prop::Dynamic(*a), move |_a| {
            b.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*effect_runs.borrow(), 1);
        a.set(2);
        assert_eq!(*effect_runs.borrow(), 2);
        b.set(20);
        assert_eq!(*effect_runs.borrow(), 2);

        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();
        let effect = watch_immediate(Prop::Static(1), move |_a| {
            *effect_runs_clone.borrow_mut() += 1;
        });

        assert_eq!(*effect_runs.borrow(), 1);
        effect.dispose();
    }

    #[test]
    fn test_computed_dependency_chain() {
        let a = signal(1);
        let b = signal(2);

        let sum = computed(move || a.get() + b.get());
        let product = computed(move || sum.get() * 2);

        assert_eq!(sum.get(), 3);
        assert_eq!(product.get(), 6);

        a.set(5);
        assert_eq!(sum.get(), 7);
        assert_eq!(product.get(), 14);

        b.set(3);
        assert_eq!(sum.get(), 8);
        assert_eq!(product.get(), 16);
    }

    #[test]
    fn test_batch_defers_effects() {
        let a = signal(1);
        let b = signal(2);
        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        watch_effect(move || {
            a.get();
            b.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        // Initial run
        assert_eq!(*effect_runs.borrow(), 1);

        // Multiple updates in batch should only trigger effect once
        batch(|| {
            a.set(10);
            b.set(20);
        });
        assert_eq!(*effect_runs.borrow(), 2);
    }

    #[test]
    fn test_batch_nested() {
        let a = signal(1);
        let b = signal(2);
        let c = signal(3);
        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        watch_effect(move || {
            a.get();
            b.get();
            c.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        // Initial run
        assert_eq!(*effect_runs.borrow(), 1);

        // Nested batches should still only trigger effect once at the end
        batch(|| {
            a.set(10);
            batch(|| {
                b.set(20);
                c.set(30);
            });
        });
        assert_eq!(*effect_runs.borrow(), 2);
    }

    #[test]
    fn test_batch_with_memo() {
        let a = signal(1);
        let b = signal(2);

        let sum = computed(move || a.get() + b.get());

        // Initial value
        assert_eq!(sum.get(), 3);

        // Update in batch
        batch(|| {
            a.set(10);
            b.set(20);
            // Memo should not be recalculated yet during batch
        });

        // After batch, memo should reflect new values
        assert_eq!(sum.get(), 30);
    }

    #[test]
    fn test_force_trigger() {
        let count = signal(0);
        let effect_runs = Rc::new(RefCell::new(0));
        let effect_runs_clone = effect_runs.clone();

        let effect = watch_effect(move || {
            count.get();
            *effect_runs_clone.borrow_mut() += 1;
        });

        // Initial run
        assert_eq!(*effect_runs.borrow(), 1);

        // Force trigger should run effect even if dependencies haven't changed
        effect.force_trigger();
        assert_eq!(*effect_runs.borrow(), 2);

        // Update should also trigger effect
        count.set(1);
        assert_eq!(*effect_runs.borrow(), 3);
    }
}
