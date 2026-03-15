use std::rc::Rc;

use mue_core::Prop;

use crate::Node;

pub trait Children {
    fn into_nodes(self) -> Prop<Rc<[Node]>>;
}

impl<const N: usize> Children for [Node; N] {
    fn into_nodes(self) -> Prop<Rc<[Node]>> {
        Prop::Static(Rc::from(self))
    }
}

impl Children for Vec<Node> {
    fn into_nodes(self) -> Prop<Rc<[Node]>> {
        Prop::Static(Rc::from(self))
    }
}
