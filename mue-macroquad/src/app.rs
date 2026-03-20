use glam::vec2;
use mue_core::batch;
use taffy::{AvailableSpace, Size};

use crate::{
    event::pointer::PointerManager,
    node::{IntoNode, Node},
    runtime::Runtime,
};

pub struct App {
    root_node: Node,
    layout_id: Option<taffy::NodeId>,

    motion_manager: PointerManager,
}

impl App {
    pub fn new(root_node: impl IntoNode) -> Self {
        let root_node = root_node.into_node();
        let layout_id = Runtime::with(|rt| rt.node(root_node.id).layout_id);

        Self {
            root_node,
            layout_id,

            motion_manager: PointerManager::new(),
        }
    }

    pub fn frame(&mut self) {
        let root_node = *self.root_node;
        batch(|| {
            if let Some(layout_id) = self.layout_id {
                Runtime::with_taffy_mut(|taffy| {
                    taffy
                        .compute_layout_with_measure(
                            layout_id,
                            Size {
                                width: AvailableSpace::Definite(macroquad::window::screen_width()),
                                height: AvailableSpace::Definite(macroquad::window::screen_height()),
                            },
                            |known_dimensions, available_space, _node_id, node_context, _style| {
                                node_context.map_or(Size::ZERO, |f| {
                                    f.measure(known_dimensions, available_space)
                                })
                            },
                        )
                        .unwrap();
                });
            }
            self.root_node.render(vec2(0., 0.));
            self.motion_manager.process(root_node);
        });

        crate::shader::consume_delete_queue();
    }
}
