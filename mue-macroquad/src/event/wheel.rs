use crate::{event::hit_test::hit_test, math::Vector, node::NodeRef, runtime::Runtime};

#[derive(Debug, Clone)]
pub struct WheelEvent {
    pub position: Vector,
    pub delta: Vector,
}

pub fn check_wheel(root_node: NodeRef) {
    let pos = macroquad::input::mouse_position();
    let delta = macroquad::input::mouse_wheel();
    if delta.0 == 0. && delta.1 == 0. {
        return;
    }

    let event = WheelEvent {
        position: Vector::new(pos.0, pos.1),
        delta: Vector::new(delta.0, delta.1),
    };
    let nodes = hit_test(root_node, event.position);
    Runtime::with(|rt| {
        for node in nodes.iter().rev() {
            rt.invoke_hook(node.id, |hooks| &mut hooks.wheel_event, &event);
        }
    });
}
