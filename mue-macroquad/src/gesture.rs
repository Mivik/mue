mod long_press;
mod tap;

pub use long_press::LongPressGesture;
pub use tap::TapGesture;

use slotmap::new_key_type;

use crate::event::pointer::{ClaimToken, PointerEvent, PointerId};

new_key_type! {
    pub(crate) struct GestureId;
}

pub trait Gesture {
    fn on_event(&mut self, event: &PointerEvent, claim_token: &ClaimToken);

    fn on_rejected(&mut self, _pointer_id: PointerId) {}
}
