use macroquad::prelude::*;
use mue_core::batch;
use taffy::{AvailableSpace, Size};

use crate::{
    node::{IntoNode, Node},
    runtime::Runtime,
};

pub struct App {
    root_node: Node,
    layout_id: Option<taffy::NodeId>,
}

impl App {
    pub fn new(root_node: impl IntoNode) -> Self {
        let root_node = root_node.into_node();
        let layout_id = Runtime::with(|rt| rt.node(root_node.id).layout_id);

        Self {
            root_node,
            layout_id,
        }
    }

    pub fn frame(&self) {
        batch(|| {
            if let Some(layout_id) = self.layout_id {
                Runtime::with(|rt| {
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy
                        .compute_layout(
                            layout_id,
                            Size {
                                width: AvailableSpace::Definite(screen_width()),
                                height: AvailableSpace::Definite(screen_height()),
                            },
                        )
                        .unwrap();
                });
            }
            self.root_node.render();
        });

        crate::shader::consume_delete_queue();
    }
}
