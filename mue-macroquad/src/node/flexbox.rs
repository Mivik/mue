use mue_core::{
    effect::{computed, watch_immediate},
    signal::{Access, ReadSignal},
    Prop,
};

use crate::{layout::use_layout, node::NodeContext, runtime::Runtime, Node, Style};

fn collect_layout_ids(nodes: &[Node]) -> ReadSignal<Vec<taffy::NodeId>> {
    let array = Runtime::with(|rt| {
        nodes
            .iter()
            .map(|node| rt.node(node.id).layout_ids.clone())
            .collect::<Vec<_>>()
    });
    computed(move || array.iter().flat_map(|ids| ids.get_clone()).collect())
}

pub fn flexbox(children: Vec<Node>) -> Node {
    Node::build(move || {
        let layout = use_layout(Style::new());
        let layout_ids = collect_layout_ids(&children);
        watch_immediate(Prop::Dynamic(layout_ids), move |_| {
            Runtime::with(|rt| {
                let mut taffy = rt.taffy.borrow_mut();
                let layout_id = layout.id();
                for child_layout_id in layout_ids.get_clone() {
                    taffy.add_child(layout_id, child_layout_id).unwrap();
                }
            });
        });
        NodeContext::set_children(children);
    })
}
