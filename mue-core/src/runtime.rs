use std::{
    cell::{Cell, RefCell, RefMut},
    mem,
};

use slotmap::SlotMap;

use crate::{
    effect::{Dependencies, EffectId, EffectInner, EffectState},
    signal::{SignalId, SignalInner},
};

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

/// The reactive runtime that manages all signals, memos, and effects.
pub(crate) struct Runtime {
    pub signals: RefCell<SlotMap<SignalId, SignalInner>>,
    pub effects: RefCell<SlotMap<EffectId, EffectInner>>,

    pub null_signal: SignalId,

    tracker: RefCell<Option<Vec<SignalId>>>,

    batch_depth: Cell<usize>,
    pending_updates: RefCell<Vec<EffectId>>,
}

impl Runtime {
    fn new() -> Self {
        let signals = RefCell::new(SlotMap::with_key());
        let null_signal = signals.borrow_mut().insert(SignalInner::new(None, None));

        Self {
            signals,
            effects: RefCell::new(SlotMap::with_key()),

            null_signal,

            tracker: RefCell::default(),

            batch_depth: Cell::new(0),
            pending_updates: RefCell::default(),
        }
    }

    pub fn with<R>(f: impl FnOnce(&Runtime) -> R) -> R {
        RUNTIME.with(|rt| f(rt))
    }

    pub fn signal_mut(&self, signal_id: SignalId) -> RefMut<'_, SignalInner> {
        RefMut::map(self.signals.borrow_mut(), |arena| &mut arena[signal_id])
    }

    pub fn effect_mut(&self, effect_id: EffectId) -> RefMut<'_, EffectInner> {
        RefMut::map(self.effects.borrow_mut(), |arena| &mut arena[effect_id])
    }

    pub fn track(&self, signal_id: SignalId) {
        if let Some(tracker) = self.tracker.borrow_mut().as_mut() {
            tracker.push(signal_id);
        }
    }

    pub fn on_update(&self, signal_id: SignalId) {
        let subscribers = mem::take(&mut self.signal_mut(signal_id).subscribers);
        if self.batch_depth.get() > 0 {
            for effect_id in &subscribers {
                self.mark_stale(*effect_id, EffectState::Dirty);
            }
        } else {
            for effect_id in &subscribers {
                self.update(*effect_id);
            }
        }
        self.signal_mut(signal_id).subscribers = subscribers;
    }

    pub fn update(&self, effect_id: EffectId) {
        let mut effect = self.effect_mut(effect_id);
        let is_dynamic = matches!(effect.dependencies, Dependencies::Dynamic(_));
        let prev_tracker = self.tracker.replace(is_dynamic.then(Vec::new));

        let signal_id = effect.signal;
        let mut value = self.signal_mut(signal_id).value.take();
        let mut callback = effect.callback.take().unwrap();
        drop(effect);

        let updated = callback(&mut value);
        self.signal_mut(signal_id).value = value;

        // TODO: optimize, avoid unnecessary re-tracking
        let mut effect = self.effect_mut(effect_id);
        effect.callback = Some(callback);
        if let Dependencies::Dynamic(dependencies) = &mut effect.dependencies {
            for dep in dependencies.drain(..) {
                self.signal_mut(dep).subscribers.remove(&effect_id);
            }
            *dependencies = self.tracker.borrow_mut().take().unwrap();
            for dep in dependencies {
                self.signal_mut(*dep).subscribers.insert(effect_id);
            }

            effect.state = EffectState::Clean;
            drop(effect);
        }

        if updated {
            self.on_update(signal_id);
        }

        *self.tracker.borrow_mut() = prev_tracker;
    }

    pub fn update_if_necessary(&self, signal_id: SignalId) {
        let Some(effect_id) = self.signal_mut(signal_id).effect else {
            return;
        };
        let mut effect = self.effect_mut(effect_id);
        if effect.state == EffectState::Check {
            let dependencies = mem::take(&mut effect.dependencies);
            for dependency in &*dependencies {
                self.update_if_necessary(*dependency);
                if self.effect_mut(effect_id).state == EffectState::Dirty {
                    break;
                }
            }
            self.effect_mut(effect_id).dependencies = dependencies;
            effect = self.effect_mut(effect_id);
        }
        if effect.state == EffectState::Dirty {
            drop(effect);
            self.update(effect_id);
            effect = self.effect_mut(effect_id);
        }

        effect.state = EffectState::Clean;
    }

    fn mark_stale(&self, effect_id: EffectId, state: EffectState) {
        let mut effect = self.effect_mut(effect_id);
        if effect.state >= state {
            return;
        }

        let old_state = mem::replace(&mut effect.state, state);
        if old_state == EffectState::Clean {
            self.pending_updates.borrow_mut().push(effect_id);
        } else {
            return;
        }

        let signal = effect.signal;
        drop(effect);

        let subscribers = mem::take(&mut self.signal_mut(signal).subscribers);
        for subscriber in &subscribers {
            self.mark_stale(*subscriber, EffectState::Check);
        }
        self.signal_mut(signal).subscribers = subscribers;
    }

    pub fn batch(&self, f: impl FnOnce()) {
        let depth = self.batch_depth.get();
        self.batch_depth.set(depth + 1);
        f();
        self.batch_depth.set(depth);

        if depth == 0 {
            let pending_updates = mem::take(&mut *self.pending_updates.borrow_mut());
            for effect_id in pending_updates {
                self.update(effect_id);
            }
        }
    }

    /// Dispose a signal, removing it and cleaning up all dependencies.
    pub fn dispose_signal(&self, signal_id: SignalId) {
        // Don't dispose the null signal
        if signal_id == self.null_signal {
            return;
        }

        // Remove this signal from all effects that depend on it
        let subscribers = mem::take(&mut self.signal_mut(signal_id).subscribers);
        for effect_id in subscribers {
            if let Some(effect) = self.effects.borrow_mut().get_mut(effect_id) {
                // Remove signal from effect's dependencies
                if let Dependencies::Dynamic(deps) = &mut effect.dependencies {
                    deps.retain(|&id| id != signal_id);
                }
            }
        }

        // If there's an effect associated with this signal (computed), dispose it too
        if let Some(effect_id) = self.signal_mut(signal_id).effect {
            self.dispose_effect(effect_id);
        }

        // Remove the signal from the runtime
        self.signals.borrow_mut().remove(signal_id);
    }

    /// Dispose an effect, removing it and cleaning up all dependencies.
    pub fn dispose_effect(&self, effect_id: EffectId) {
        let Some(effect) = self.effects.borrow_mut().remove(effect_id) else {
            return;
        };

        // Remove this effect from all signals it depends on
        for signal_id in &*effect.dependencies {
            self.signal_mut(*signal_id).subscribers.remove(&effect_id);
        }

        self.dispose_signal(effect.signal);
    }
}

pub fn batch(f: impl FnOnce()) {
    Runtime::with(|rt| rt.batch(f))
}
