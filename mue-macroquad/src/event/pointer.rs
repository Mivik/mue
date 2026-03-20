use std::{
    collections::HashMap,
    fmt,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use glam::vec2;
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use miniquad::{EventHandler, MouseButton};
use mue_core::signal::Access;

use slotmap::Key;

use crate::{
    gesture::{GestureId, GestureUpdate},
    math::Vector,
    node::{NodeInner, NodeRef},
    runtime::Runtime,
};

pub(crate) type HitTestFn = Rc<dyn Fn(NodeRef, Vector, &mut Vec<GestureId>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerAction {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerType {
    Touch,
    Mouse,
}

/// Identifies a pointer (touch finger or mouse button).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PointerId(u64);

impl PointerId {
    pub fn from_touch_id(id: u64) -> Self {
        Self(id)
    }

    pub fn from_mouse_button(button: MouseButton) -> Self {
        Self(
            u64::MAX
                - match button {
                    MouseButton::Left => 0,
                    MouseButton::Middle => 1,
                    MouseButton::Right => 2,
                    MouseButton::Unknown => 3,
                },
        )
    }
}

#[derive(Debug)]
pub struct PointerEvent {
    pointer_id: PointerId,
    pointer_type: PointerType,

    action: PointerAction,
    start_position: Vector,
    position: Vector,
    delta: Vector,

    claim_token: ClaimToken,
}

impl PointerEvent {
    pub fn new(pointer_id: PointerId, pointer_type: PointerType, start_pos: Vector) -> Self {
        let claim_token = ClaimToken {
            pointer_id,
            token: Rc::new(AtomicBool::new(false)),
        };
        Self {
            pointer_id,
            pointer_type,

            action: PointerAction::Down,
            start_position: start_pos,
            position: start_pos,
            delta: vec2(0., 0.),

            claim_token,
        }
    }

    fn update(&mut self, action: PointerAction, current_pos: Vector) {
        self.delta = current_pos - self.position;
        self.position = current_pos;
        self.action = action;
    }

    pub fn claim_token(&self) -> &ClaimToken {
        &self.claim_token
    }

    pub fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }

    pub fn pointer_type(&self) -> PointerType {
        self.pointer_type
    }

    pub fn action(&self) -> PointerAction {
        self.action
    }

    pub fn start_position(&self) -> Vector {
        self.start_position
    }

    pub fn position(&self) -> Vector {
        self.position
    }

    pub fn delta(&self) -> Vector {
        self.delta
    }
}

#[derive(Debug)]
pub struct ClaimedPointer(PointerId);

#[derive(Clone)]
pub struct ClaimToken {
    pointer_id: PointerId,
    token: Rc<AtomicBool>,
}

impl fmt::Debug for ClaimToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClaimToken")
            .field("pointer_id", &self.pointer_id)
            .finish()
    }
}

impl ClaimToken {
    pub fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }

    pub fn can_claim(&self) -> bool {
        !self.token.load(Ordering::Relaxed)
    }

    pub fn claim(&self) -> Option<ClaimedPointer> {
        if self
            .token
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            Some(ClaimedPointer(self.pointer_id))
        } else {
            None
        }
    }
}

struct PointerTrack {
    gestures: Vec<GestureId>,
    event: PointerEvent,
}

impl PointerTrack {
    fn new(gestures: Vec<GestureId>, event: PointerEvent) -> Self {
        Self { gestures, event }
    }

    fn dispatch(&mut self, claims: &mut Vec<(ClaimedPointer, GestureId)>) {
        Runtime::with(|rt| {
            let mut gesture_map = rt.gestures.borrow_mut();
            self.gestures.retain(|gesture_id| {
                let gesture = &mut gesture_map[*gesture_id];
                let update = gesture.on_event(&self.event);
                match update {
                    GestureUpdate::Pending => true,
                    GestureUpdate::Accept(pointers) => {
                        claims.extend(pointers.into_iter().map(|p| (p, *gesture_id)));
                        false
                    }
                    GestureUpdate::Reject => false,
                }
            });
        });
    }

    pub fn update(
        &mut self,
        action: PointerAction,
        pos: Vector,
        claims: &mut Vec<(ClaimedPointer, GestureId)>,
    ) {
        self.event.update(action, pos);
        self.dispatch(claims);
    }
}

pub struct PointerManager {
    root_node: NodeRef,
    subscriber_id: usize,
    tracks: HashMap<PointerId, PointerTrack>,
    claims: Vec<(ClaimedPointer, GestureId)>,
    mouse_pos: Vector,
}

impl Default for PointerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerManager {
    pub fn new() -> Self {
        Self {
            root_node: NodeRef::null(),
            subscriber_id: register_input_subscriber(),
            tracks: HashMap::new(),
            claims: Vec::new(),
            mouse_pos: Vector::ZERO,
        }
    }

    pub fn process(&mut self, root_node: NodeRef) {
        self.root_node = root_node;
        repeat_all_miniquad_input(self, self.subscriber_id);
    }

    fn hit_test(&self, pos: Vector) -> Vec<GestureId> {
        let mut result = Vec::new();
        do_hit_test(self.root_node, pos, &mut result);
        result
    }

    fn flush_claims(&mut self) {
        Runtime::with(|rt| {
            let mut gesture_map = rt.gestures.borrow_mut();
            for (pointer, gesture_id) in self.claims.drain(..) {
                let Some(track) = self.tracks.get_mut(&pointer.0) else {
                    continue;
                };
                track.gestures.retain(|&id| {
                    if gesture_id == id {
                        true
                    } else {
                        gesture_map[id].on_rejected(pointer.0);
                        true
                    }
                });
            }
        })
    }

    fn on_pointer_start(&mut self, id: PointerId, pointer_type: PointerType, pos: Vector) {
        let gestures = self.hit_test(pos);
        let event = PointerEvent::new(id, pointer_type, pos);
        let mut track = PointerTrack::new(gestures, event);

        use std::collections::hash_map::Entry;
        match self.tracks.entry(id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .update(PointerAction::Cancel, pos, &mut self.claims);
                track.dispatch(&mut self.claims);
                occupied.insert(track);
            }
            Entry::Vacant(vacant) => {
                track.dispatch(&mut self.claims);
                vacant.insert(track);
            }
        }
        self.flush_claims();
    }

    fn on_pointer_move(&mut self, id: PointerId, pos: Vector) {
        if let Some(track) = self.tracks.get_mut(&id) {
            track.update(PointerAction::Move, pos, &mut self.claims);
            self.flush_claims();
        }
    }

    fn on_pointer_end(&mut self, id: PointerId) {
        if let Some(mut track) = self.tracks.remove(&id) {
            track.update(PointerAction::Up, track.event.position(), &mut self.claims);
            self.flush_claims();
        }
    }

    fn on_pointer_cancel(&mut self, id: PointerId) {
        if let Some(mut track) = self.tracks.remove(&id) {
            track.update(
                PointerAction::Cancel,
                track.event.position(),
                &mut self.claims,
            );
            self.flush_claims();
        }
    }
}

fn do_hit_test(node: NodeRef, pos: Vector, result: &mut Vec<GestureId>) {
    if node.id.is_null() {
        return;
    }
    Runtime::with(|rt| {
        let inner = rt.node(node.id).hit_test.clone();
        let inner = inner.as_deref().unwrap_or(&default_hit_test);
        inner(node, pos, result);
    });
}

fn default_hit_test(node: NodeRef, pos: Vector, result: &mut Vec<GestureId>) {
    let (layout_rect, children) = Runtime::with(|rt| {
        let n = rt.node(node.id);
        let children = if n.children.is_null() {
            Default::default()
        } else {
            n.children.get_clone_untracked()
        };
        (n.layout.rect, children)
    });

    if !layout_rect.is_null() && !layout_rect.get().contains(&pos) {
        return;
    }

    for &child in children.iter().rev() {
        do_hit_test(child, pos, result);
    }

    Runtime::with(|rt| {
        result.extend(rt.node(node.id).gestures.iter().cloned());
    });
}

impl EventHandler for PointerManager {
    fn update(&mut self, _ctx: &mut miniquad::Context) {}
    fn draw(&mut self, _ctx: &mut miniquad::Context) {}

    fn mouse_motion_event(&mut self, _ctx: &mut miniquad::Context, x: f32, y: f32) {
        let pos = Vector::new(x, y);
        let old_pos = self.mouse_pos;
        self.mouse_pos = pos;

        // Propagate move to any active mouse-button pointers.
        let ids: Vec<PointerId> = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
            .iter()
            .map(|&b| PointerId::from_mouse_button(b))
            .filter(|id| self.tracks.contains_key(id))
            .collect();

        if ids.is_empty() {
            let _ = old_pos; // suppress unused warning
        }

        for id in ids {
            self.on_pointer_move(id, pos);
        }
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut miniquad::Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) {
        let id = PointerId::from_mouse_button(button);
        let pos = Vector::new(x, y);
        self.mouse_pos = pos;
        self.on_pointer_start(id, PointerType::Mouse, pos);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut miniquad::Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.on_pointer_end(PointerId::from_mouse_button(button));
    }

    fn touch_event(
        &mut self,
        _ctx: &mut miniquad::Context,
        phase: miniquad::TouchPhase,
        id: u64,
        x: f32,
        y: f32,
        _time: f64,
    ) {
        let pointer_id = PointerId::from_touch_id(id);
        let pos = Vector::new(x, y);
        match phase {
            miniquad::TouchPhase::Started => {
                self.on_pointer_start(pointer_id, PointerType::Touch, pos)
            }
            miniquad::TouchPhase::Moved => self.on_pointer_move(pointer_id, pos),
            miniquad::TouchPhase::Ended => self.on_pointer_end(pointer_id),
            miniquad::TouchPhase::Cancelled => self.on_pointer_cancel(pointer_id),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn set_hit_test_fn(hit_test: HitTestFn) {
    NodeInner::with_mut(|node| {
        node.hit_test = Some(hit_test);
    })
}
