use taffy::Display;

use crate::{
    layout::use_layout,
    node::{children::IntoChildren, Node, NodeInner},
    Style,
};

pub fn div(children: impl IntoChildren) -> Node {
    flexbox(children).styled(Style::new().display(Display::Block))
}

pub fn flexbox(children: impl IntoChildren) -> Node {
    Node::build(move || {
        use_layout();
        NodeInner::set_children(children.into_children());
    })
}
