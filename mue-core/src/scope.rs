use std::{cell::RefCell, mem};

use crate::{effect::EffectId, runtime::Runtime, signal::SignalId};

thread_local! {
    pub(crate) static CURRENT_SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct Scope {
    pub(crate) effects: Vec<EffectId>,
    pub(crate) signals: Vec<SignalId>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let prev_scope = CURRENT_SCOPE.replace(Some(mem::take(self)));
        let result = f();
        *self = CURRENT_SCOPE.replace(prev_scope).unwrap();
        result
    }

    pub fn dispose(self) {
        let Scope { effects, signals } = self;
        Runtime::with(|rt| {
            for effect_id in effects {
                rt.dispose_effect(effect_id);
            }
            for signal_id in signals {
                rt.dispose_signal(signal_id);
            }
        });
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::prelude::*;

    #[test]
    fn test_scope_disposes_on_drop() {
        let count = signal(0);
        let runs = Rc::new(RefCell::new(0));
        let runs_clone = runs.clone();

        let mut scope = Scope::new();
        scope.run(|| {
            watch_effect(move || {
                count.get();
                *runs_clone.borrow_mut() += 1;
            });

            // Effect runs once inside scope
            assert_eq!(*runs.borrow(), 1);

            // Update triggers effect
            count.set(1);
            assert_eq!(*runs.borrow(), 2);
        });
        scope.dispose();

        // Update should not trigger effect anymore
        count.set(2);
        assert_eq!(*runs.borrow(), 2);
    }

    #[test]
    fn test_nested_scopes() {
        let outer_runs = Rc::new(RefCell::new(0));
        let inner_runs = Rc::new(RefCell::new(0));
        let outer_clone = outer_runs.clone();
        let inner_clone = inner_runs.clone();

        let count = signal(0);

        let mut scope_outer = Scope::new();
        scope_outer.run(|| {
            watch_effect(move || {
                count.get();
                *outer_clone.borrow_mut() += 1;
            });

            let mut scope_inner = Scope::new();
            scope_inner.run(|| {
                watch_effect(move || {
                    count.get();
                    *inner_clone.borrow_mut() += 1;
                });
            });
            // Inner effect runs
            assert_eq!(*inner_runs.borrow(), 1);
            scope_inner.dispose();

            // Update triggers only outer effect
            count.set(1);
            assert_eq!(*outer_runs.borrow(), 2);
            assert_eq!(*inner_runs.borrow(), 1);
        });
        scope_outer.dispose();

        count.set(2);
        // Neither effect should run
        assert_eq!(*outer_runs.borrow(), 2);
        assert_eq!(*inner_runs.borrow(), 1);
    }

    #[test]
    fn test_scope_with_computed() {
        let mut scope = Scope::new();
        let result = scope.run(|| {
            let a = signal(2);
            let b = signal(3);
            let sum = computed(move || a.get() + b.get());
            sum.get()
        });
        assert_eq!(result, 5);
        // Computed is disposed when scope drops
    }
}
