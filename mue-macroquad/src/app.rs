use macroquad::prelude::*;
use mue_core::batch;
use taffy::{AvailableSpace, Size};

use crate::{node::Node, runtime::Runtime, Layout};

pub struct App {
    root_node: Node,
    top: f32,

    layout: Option<Layout>,
}

impl App {
    pub fn new(root_node: Node) -> Self {
        let layout = Runtime::with(|rt| rt.node(root_node.id).layout);

        let sw = screen_width();
        let sh = screen_height();
        let top = sh / sw;
        set_camera(&Camera2D {
            zoom: vec2(2., -2. * sw / sh),
            viewport: Some((0, 0, sw as i32, sh as i32)),
            offset: vec2(-1., 1.),
            ..Default::default()
        });

        Self {
            root_node,
            top,

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
                                width: AvailableSpace::Definite(1.),
                                height: AvailableSpace::Definite(self.top),
                            },
                        )
                        .unwrap();
                });
            }
            self.root_node.render();
        });
    }
}
