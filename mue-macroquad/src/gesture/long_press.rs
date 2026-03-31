use std::{cell::RefCell, rc::Rc};

use crate::{
    event::pointer::{ClaimToken, PointerAction, PointerEvent, PointerId},
    gesture::Gesture,
    hook::HookFn,
    runtime::set_timeout,
};

pub struct LongPressGesture {
    threshold: f32,
    duration: f32,

    active: Option<PointerId>,

    pub(crate) on_long_press: Rc<RefCell<HookFn<()>>>,
}

impl Default for LongPressGesture {
    fn default() -> Self {
        Self::new(15.0, 0.5)
    }
}

impl LongPressGesture {
    pub fn new(threshold: f32, duration: f32) -> Self {
        Self {
            threshold,
            duration,

            active: None,

            on_long_press: Rc::new(RefCell::new(HookFn::default())),
        }
    }
}

impl Gesture for LongPressGesture {
    fn on_event(&mut self, event: &PointerEvent, claim_token: &ClaimToken) {
        if let Some(active) = &self.active {
            if *active != event.pointer_id() {
                return;
            }

            return match event.action() {
                PointerAction::Down => {}
                PointerAction::Move => {
                    if (event.start_position() - event.position()).length() > self.threshold {
                        self.active = None;
                        claim_token.dismiss();
                    }
                }
                PointerAction::Up | PointerAction::Cancel => {
                    self.active = None;
                    claim_token.dismiss();
                }
            };
        }

        // No active pointer, try to claim this one
        if event.action() == PointerAction::Down {
            self.active = Some(event.pointer_id());
            set_timeout(self.duration, {
                let claim_token = claim_token.clone();
                let on_long_press = self.on_long_press.clone();
                move || {
                    if claim_token.claim() {
                        on_long_press.borrow_mut().invoke(&());
                    }
                }
            });
        } else {
            claim_token.dismiss();
        }
    }

    fn on_rejected(&mut self, pointer_id: PointerId) {
        assert!(self.active.as_ref().is_some_and(|id| id == &pointer_id));
        self.active = None;
    }
}
