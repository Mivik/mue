use crate::{
    event::pointer::{ClaimToken, PointerAction, PointerEvent, PointerId},
    gesture::{Gesture, GestureUpdate},
    hook::HookFn,
};

pub struct TapGesture {
    threshold: f32,

    active: Option<ClaimToken>,

    pub(crate) on_click: HookFn<()>,
    pub(crate) on_tap_down: HookFn<()>,
    pub(crate) on_tap_up: HookFn<()>,
    pub(crate) on_tap_cancel: HookFn<()>,
}

impl Default for TapGesture {
    fn default() -> Self {
        Self::new(10.0)
    }
}

impl TapGesture {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,

            active: None,

            on_click: HookFn::default(),
            on_tap_down: HookFn::default(),
            on_tap_up: HookFn::default(),
            on_tap_cancel: HookFn::default(),
        }
    }
}

impl Gesture for TapGesture {
    fn on_event(&mut self, event: &PointerEvent) -> super::GestureUpdate {
        if let Some(active) = &self.active {
            if active.pointer_id() != event.pointer_id() {
                return GestureUpdate::Reject;
            }

            return match event.action() {
                PointerAction::Down => GestureUpdate::Reject,
                PointerAction::Move => {
                    if (event.start_position() - event.position()).length() > self.threshold {
                        self.active = None;
                        self.on_tap_cancel.invoke(&());
                        GestureUpdate::Reject
                    } else {
                        GestureUpdate::Pending
                    }
                }
                PointerAction::Cancel => {
                    self.on_tap_cancel.invoke(&());
                    self.active = None;
                    GestureUpdate::Reject
                }
                PointerAction::Up => {
                    let update = if let Some(update) = GestureUpdate::claim(active) {
                        self.on_tap_up.invoke(&());
                        self.on_click.invoke(&());
                        update
                    } else {
                        self.on_tap_cancel.invoke(&());
                        GestureUpdate::Reject
                    };
                    self.active = None;
                    update
                }
            };
        }

        // No active pointer, try to claim this one
        if event.action() == PointerAction::Down {
            self.active = Some(event.claim_token().clone());
            self.on_tap_down.invoke(&());
            GestureUpdate::Pending
        } else {
            GestureUpdate::Reject
        }
    }

    fn on_rejected(&mut self, pointer_id: PointerId) {
        assert!(self
            .active
            .as_ref()
            .is_some_and(|token| token.pointer_id() == pointer_id));
        self.active = None;
    }
}
