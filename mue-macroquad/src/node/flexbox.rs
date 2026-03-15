use std::rc::Rc;

use mue_core::{effect::watch_effect, signal::Access, Prop};

use crate::{
    layout::use_layout,
    node::{children::Children, NodeInner},
    runtime::Runtime,
    Node,
};

pub fn flexbox(children: impl Children) -> Node {
    fn inner(children: Prop<Rc<[Node]>>) -> Node {
        Node::build(move || {
            let layout = use_layout();
            NodeInner::with_mut(|node| {
                node.children = children.clone();
            });
            watch_effect(move || {
                Runtime::with(|rt| {
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy.remove_children_range(layout.id(), ..).unwrap();
                    for child_layout in children
                        .get_clone()
                        .iter()
                        .filter_map(|node| rt.node(node.id).layout)
                    {
                        taffy.add_child(layout.id(), child_layout.id()).unwrap();
                    }
                });
            });
        })
    }

    inner(children.into_nodes())
}
