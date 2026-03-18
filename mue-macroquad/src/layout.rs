use macroquad::math::Rect;
use mue_core::prelude::*;

use crate::{node::NodeInner, runtime::Runtime, style::Style};

#[non_exhaustive]
pub struct Layout {
    pub rect: ReadSignal<Rect>,
}

pub(crate) struct OwnedLayout {
    pub rect: Signal<Rect>,
}

impl Default for OwnedLayout {
    fn default() -> Self {
        Self {
            rect: Signal::null(),
        }
    }
}

pub fn use_layout(style: &mut Style) -> Layout {
    let style = style.build_taffy();
    NodeInner::with_mut(|node| {
        if node.layout_id.is_some() {
            panic!("use_layout can only be called once per node");
        }

        let layout_id =
            Runtime::with(|rt| rt.taffy.borrow_mut().new_leaf(Default::default()).unwrap());
        watch_effect(move || {
            Runtime::with(|rt| {
                rt.taffy
                    .borrow_mut()
                    .set_style(layout_id, style.get_clone())
                    .unwrap();
            });
        });

        node.layout_id = Some(layout_id);
        node.layout = OwnedLayout {
            rect: signal(Rect::default()),
        };

        let children = *node.children;
        node.scope.run(|| {
            watch_effect(move || {
                Runtime::with(|rt| {
                    let children = children.get_clone();
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy.remove_children_range(layout_id, ..).unwrap();
                    for child_layout_id in children
                        .iter()
                        .filter_map(|node| rt.node(node.id).layout_id)
                    {
                        taffy.add_child(layout_id, child_layout_id).unwrap();
                    }
                });
            });
        });

        Layout {
            rect: *node.layout.rect,
        }
    })
}
