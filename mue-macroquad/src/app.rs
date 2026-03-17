use macroquad::prelude::*;
use mue_core::batch;
use taffy::{AvailableSpace, Size};

use crate::{IntoNode, Layout, node::Node, runtime::Runtime};

pub struct App {
    root_node: Node,

    layout: Option<Layout>,
}

impl App {
    pub fn new(root_node: impl IntoNode) -> Self {
        let root_node = root_node.into_node();
        let layout = Runtime::with(|rt| rt.node(root_node.id).layout);

        Self {
            root_node,

            layout,
        }
    }

    pub fn frame(&self) {
        batch(|| {
            if let Some(layout) = self.layout {
                Runtime::with(|rt| {
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy
                        .compute_layout(
                            layout.id(),
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
    }
}
