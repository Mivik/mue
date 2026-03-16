use crate::{
    layout::use_layout,
    node::{children::IntoChildren, Node, NodeInner},
};

pub fn flexbox(children: impl IntoChildren) -> Node {
    Node::build(move || {
        use_layout();
        NodeInner::set_children(children.into_children());
    })
}
