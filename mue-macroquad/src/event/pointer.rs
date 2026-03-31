use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};

use glam::vec2;
use indexmap::IndexSet;
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use miniquad::{EventHandler, MouseButton};
use mue_core::signal::Access;

use slotmap::Key;

use crate::{
    gesture::GestureId,
    math::Vector,
    node::{NodeInner, NodeRef},
    runtime::{get_time, Runtime},
};

pub(crate) type HitTestFn = Rc<dyn Fn(NodeRef, Vector, &mut IndexSet<NodeRef>)>;

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
                    MouseButton::Left => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Right => 3,
                    MouseButton::Unknown => 4,
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
    time: f32,
}

impl PointerEvent {
    pub fn new(pointer_id: PointerId, pointer_type: PointerType, start_pos: Vector) -> Self {
        Self {
            pointer_id,
            pointer_type,

            action: PointerAction::Down,
            start_position: start_pos,
            position: start_pos,
            delta: vec2(0., 0.),
            time: get_time(),
        }
    }

    fn update(&mut self, action: PointerAction, current_pos: Vector) {
        self.delta = current_pos - self.position;
        self.position = current_pos;
        self.action = action;
        self.time = get_time();
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

    pub fn time(&self) -> f32 {
        self.time
    }
}

#[derive(Clone)]
pub struct ClaimToken {
    pointer_id: PointerId,
    gesture_id: GestureId,
    state: Rc<PointerGestureState>,
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
        self.state.claimed_by.borrow().is_null()
            && self.state.gestures.borrow().contains(&self.gesture_id)
            && !self.state.dismissed.borrow().contains(&self.gesture_id)
    }

    #[must_use]
    pub fn claim(&self) -> bool {
        if self.can_claim() {
            *self.state.claimed_by.borrow_mut() = self.gesture_id;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn claim_all(tokens: &[&ClaimToken]) -> bool {
        if !tokens.iter().all(|token| token.can_claim()) {
            false
        } else {
            for token in tokens {
                let _ = token.claim();
            }
            true
        }
    }

    pub fn dismiss(&self) {
        self.state.dismissed.borrow_mut().insert(self.gesture_id);
    }
}

struct PointerGestureState {
    gestures: RefCell<IndexSet<GestureId>>,
    dismissed: RefCell<HashSet<GestureId>>,
    claimed_by: RefCell<GestureId>,
}

struct PointerTrack {
    nodes: IndexSet<NodeRef>,
    state: Rc<PointerGestureState>,
    event: PointerEvent,
    claim_token: ClaimToken,
}

impl PointerTrack {
    fn new(nodes: IndexSet<NodeRef>, event: PointerEvent) -> Self {
        let mut gestures = IndexSet::new();
        Runtime::with(|rt| {
            for node in nodes.iter() {
                gestures.extend(rt.node(node.id).gestures.iter().copied());
            }
        });
        let state = Rc::new(PointerGestureState {
            gestures: RefCell::new(gestures),
            dismissed: RefCell::default(),
            claimed_by: RefCell::new(GestureId::null()),
        });
        let claim_token = ClaimToken {
            pointer_id: event.pointer_id(),
            gesture_id: GestureId::null(),
            state: Rc::clone(&state),
        };
        Self {
            nodes,
            state,
            event,
            claim_token,
        }
    }

    fn flush_updates(&mut self) {
        let claimed_by = *self.state.claimed_by.borrow();
        if !claimed_by.is_null() && self.state.gestures.borrow().len() > 1 {
            Runtime::with(|rt| {
                let mut gesture_map = rt.gestures.borrow_mut();
                self.state.gestures.borrow_mut().retain(|&id| {
                    if id == claimed_by {
                        true
                    } else {
                        gesture_map[id].on_rejected(self.event.pointer_id());
                        false
                    }
                });
            })
        }

        let mut dismissed = self.state.dismissed.borrow_mut();
        if !dismissed.is_empty() {
            self.state
                .gestures
                .borrow_mut()
                .retain(|id| !dismissed.contains(id));
            dismissed.clear();
        }
    }

    fn dispatch(&mut self) {
        Runtime::with(|rt| {
            for &node in self.nodes.iter() {
                rt.invoke_hook(node.id, |hooks| &mut hooks.pointer_event, &self.event);
            }

            let mut gesture_map = rt.gestures.borrow_mut();
            for gesture_id in self.state.gestures.borrow().iter() {
                let gesture = &mut gesture_map[*gesture_id];
                self.claim_token.gesture_id = *gesture_id;
                gesture.on_event(&self.event, &self.claim_token);
            }
        });
    }

    pub fn update(&mut self, action: PointerAction, pos: Vector) {
        self.event.update(action, pos);
        self.dispatch();
    }
}

pub struct PointerManager {
    root_node: NodeRef,
    subscriber_id: usize,
    tracks: HashMap<PointerId, PointerTrack>,
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
            mouse_pos: Vector::ZERO,
        }
    }

    pub fn process(&mut self, root_node: NodeRef) {
        self.root_node = root_node;
        for track in self.tracks.values_mut() {
            track.flush_updates();
        }
        repeat_all_miniquad_input(self, self.subscriber_id);
    }

    fn hit_test(&self, pos: Vector) -> IndexSet<NodeRef> {
        let mut result = IndexSet::new();
        do_hit_test(self.root_node, pos, &mut result);
        result
    }

    fn on_pointer_start(&mut self, id: PointerId, pointer_type: PointerType, pos: Vector) {
        let nodes = self.hit_test(pos);
        let event = PointerEvent::new(id, pointer_type, pos);
        let mut track = PointerTrack::new(nodes, event);

        use std::collections::hash_map::Entry;
        match self.tracks.entry(id) {
            Entry::Occupied(mut occupied) => {
                occupied.get_mut().update(PointerAction::Cancel, pos);
                track.dispatch();
                occupied.insert(track);
            }
            Entry::Vacant(vacant) => {
                track.dispatch();
                vacant.insert(track);
            }
        }
    }

    fn on_pointer_move(&mut self, id: PointerId, pos: Vector) {
        if let Some(track) = self.tracks.get_mut(&id) {
            track.update(PointerAction::Move, pos);
        }
    }

    fn on_pointer_end(&mut self, id: PointerId) {
        if let Some(mut track) = self.tracks.remove(&id) {
            track.update(PointerAction::Up, track.event.position());
        }
    }

    fn on_pointer_cancel(&mut self, id: PointerId) {
        if let Some(mut track) = self.tracks.remove(&id) {
            track.update(PointerAction::Cancel, track.event.position());
        }
    }
}

fn do_hit_test(node: NodeRef, pos: Vector, result: &mut IndexSet<NodeRef>) {
    if node.id.is_null() {
        return;
    }
    Runtime::with(|rt| {
        let inner = rt.node(node.id).hit_test.clone();
        let inner = inner.as_deref().unwrap_or(&default_hit_test);
        inner(node, pos, result);
    });
}

fn default_hit_test(node: NodeRef, pos: Vector, result: &mut IndexSet<NodeRef>) {
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

    result.insert(node);
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
