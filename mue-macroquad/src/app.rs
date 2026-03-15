use macroquad::prelude::*;
use mue_core::{
    batch,
    effect::computed,
    signal::{Access, ReadSignal},
};
use taffy::{AvailableSpace, Size};

use crate::{runtime::Runtime, Node};

pub struct App {
    root_node: Node,
    top: f32,

    layout_id: ReadSignal<Option<taffy::NodeId>>,
}

impl App {
    pub fn new(root_node: Node) -> Self {
        let layout_ids = Runtime::with(|rt| rt.node(root_node.id).layout_ids.clone());
        let layout_id = computed(move || {
            let layout_ids = layout_ids.get_clone();
            match layout_ids.len() {
                0 => None,
                1 => Some(layout_ids[0]),
                _ => panic!("Multiple layout ids found for root node"),
            }
        });

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

            layout_id,
        }
    }

    pub fn frame(&self) {
        batch(|| {
            if let Some(layout_id) = self.layout_id.get() {
                Runtime::with(|rt| {
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy
                        .compute_layout(
                            layout_id,
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
