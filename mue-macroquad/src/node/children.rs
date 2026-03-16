use std::rc::Rc;

use mue_core::{signal::Access, Disposable};

use crate::{node::Node, NodeRef};

pub trait Children: Access<Value = Rc<[NodeRef]>> + Disposable {}

impl<T> Children for T where T: Access<Value = Rc<[NodeRef]>> + Disposable {}

pub trait IntoChildren {
    fn into_children(self) -> Rc<dyn Children>;
}

pub struct StaticChildren {
    nodes: Box<[Node]>,
    node_refs: Rc<[NodeRef]>,
}

impl StaticChildren {
    pub fn new(nodes: Box<[Node]>) -> Self {
        let node_refs = nodes.iter().map(|node| **node).collect();
        Self { nodes, node_refs }
    }
}

impl Access for StaticChildren {
    type Value = Rc<[NodeRef]>;

    fn get_clone(&self) -> Self::Value {
        self.node_refs.clone()
    }
}

impl Disposable for StaticChildren {
    fn dispose(&self) {
        for child in self.nodes.iter() {
            child.dispose();
        }
    }
}

impl IntoChildren for Box<[Node]> {
    fn into_children(self) -> Rc<dyn Children> {
        Rc::new(StaticChildren::new(self))
    }
}

impl IntoChildren for Vec<Node> {
    fn into_children(self) -> Rc<dyn Children> {
        Rc::new(StaticChildren::new(self.into()))
    }
}

impl<const N: usize> IntoChildren for [Node; N] {
    fn into_children(self) -> Rc<dyn Children> {
        Rc::new(StaticChildren::new(self.into()))
    }
}
