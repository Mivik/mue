mod tap;

pub use tap::TapGesture;

use slotmap::new_key_type;
use smallvec::SmallVec;

use crate::event::pointer::{ClaimToken, ClaimedPointer, PointerEvent, PointerId};

new_key_type! {
    pub(crate) struct GestureId;
}

#[derive(Debug)]
pub enum GestureUpdate {
    Pending,
    Accept(SmallVec<[ClaimedPointer; 1]>),
    Reject,
}

impl GestureUpdate {
    pub fn claim(token: &ClaimToken) -> Option<Self> {
        token
            .claim()
            .map(|claimed| Self::Accept(SmallVec::from_buf([claimed])))
    }

    pub fn claim_all(tokens: &[&ClaimToken]) -> Option<Self> {
        if !tokens.iter().all(|token| token.can_claim()) {
            None
        } else {
            Some(Self::Accept(
                tokens.iter().map(|it| it.claim().unwrap()).collect(),
            ))
        }
    }
}

pub trait Gesture {
    fn on_event(&mut self, event: &PointerEvent) -> GestureUpdate;

    fn on_rejected(&mut self, _pointer_id: PointerId) {}
}
