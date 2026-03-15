use std::cell::RefCell;

use slotmap::{new_key_type, Key};

use crate::{
    effect::{Effect, EffectId},
    runtime::Runtime,
    signal::{ReadSignal, SignalId},
    Disposable,
};

new_key_type! {
    pub(crate) struct ScopeId;
}

thread_local! {
    pub(crate) static CURRENT_SCOPE: RefCell<Option<ScopeInner>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub(crate) struct ScopeInner {
    pub(crate) effects: Vec<EffectId>,
    pub(crate) signals: Vec<SignalId>,
    pub(crate) subscopes: Vec<ScopeId>,
}

impl Disposable for ScopeInner {
    fn dispose(&self) {
        let ScopeInner {
            effects,
            signals,
            subscopes,
        } = self;
        Runtime::with(|rt| {
            for effect_id in effects {
                rt.dispose_effect(*effect_id);
            }
            for signal_id in signals {
                rt.dispose_signal(*signal_id);
            }
            for subscope_id in subscopes {
                let scope = rt.scopes.borrow_mut().remove(*subscope_id);
                if let Some(scope) = scope {
                    scope.dispose();
                }
            }
        });
    }
}

#[derive(Clone, Copy)]
pub struct Scope {
    id: ScopeId,
}

impl Scope {
    pub(crate) fn new(id: ScopeId) -> Self {
        Self { id }
    }

    pub fn null() -> Self {
        Self {
            id: ScopeId::null(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.id.is_null()
    }

    pub fn push_signal<T>(&self, signal: ReadSignal<T>) {
        Runtime::with(|rt| {
            rt.scopes.borrow_mut()[self.id].signals.push(signal.id);
        });
    }

    pub fn push_effect(&self, effect: Effect) {
        Runtime::with(|rt| {
            rt.scopes.borrow_mut()[self.id].effects.push(effect.id);
        });
    }
}

impl Disposable for Scope {
    fn dispose(&self) {
        Runtime::with(|rt| {
            let scope = rt.scopes.borrow_mut().remove(self.id);
            if let Some(scope) = scope {
                scope.dispose();
            }
        });
    }
}

pub fn create_scope(f: impl FnOnce()) -> Scope {
    let prev_scope = CURRENT_SCOPE.replace(Some(ScopeInner::default()));
    f();
    let scope = CURRENT_SCOPE.replace(prev_scope).unwrap();
    let id = Runtime::with(|rt| rt.scopes.borrow_mut().insert(scope));
    CURRENT_SCOPE.with_borrow_mut(|parent| {
        if let Some(parent) = parent {
            parent.subscopes.push(id);
        }
    });
    Scope::new(id)
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

        let scope = create_scope(|| {
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

        let scope_outer = create_scope(|| {
            watch_effect(move || {
                count.get();
                *outer_clone.borrow_mut() += 1;
            });

            let scope_inner = create_scope(|| {
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
    fn test_scope_cascade_dispose() {
        let runs = Rc::new(RefCell::new(0));
        let runs_clone = runs.clone();

        let count = signal(0);

        let scope_outer = create_scope(|| {
            watch_effect(move || {
                count.get();
                *runs_clone.borrow_mut() += 1;
            });

            let scope_inner = create_scope({
                let runs_clone = runs.clone();
                || {
                    watch_effect(move || {
                        count.get();
                        *runs_clone.borrow_mut() += 1;
                    });
                }
            });
            // Both effects run
            assert_eq!(*runs.borrow(), 2);
            scope_inner.dispose();
        });
        scope_outer.dispose();

        count.set(1);
        // No effects should run
        assert_eq!(*runs.borrow(), 2);
    }
}
