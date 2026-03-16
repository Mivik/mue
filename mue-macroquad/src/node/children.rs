use std::{collections::HashMap, hash::Hash, mem, rc::Rc};

use mue_core::{
    effect::computed,
    prelude::current_scope,
    signal::{Access, ReadSignal},
    Disposable, Prop,
};

use crate::{node::Node, NodeRef};

pub trait Children: Access<Value = Rc<[NodeRef]>> + Disposable {}

pub trait IntoChildren {
    fn into_children(self) -> Rc<dyn Children>;
}

impl<T: Children + 'static> IntoChildren for T {
    fn into_children(self) -> Rc<dyn Children> {
        Rc::new(self)
    }
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

impl Children for StaticChildren {}

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

pub struct KeyedChildren {
    node_refs: ReadSignal<Rc<[NodeRef]>>,
}

impl KeyedChildren {
    pub fn new<T, K>(
        list: impl Into<Prop<Rc<[T]>>>,
        key_fn: impl Fn(&T) -> K + 'static,
        node_fn: impl Fn(&T) -> Node + 'static,
    ) -> Self
    where
        T: 'static,
        K: Hash + Eq + 'static,
    {
        fn inner<T, K>(
            list: Prop<Rc<[T]>>,
            key_fn: Box<dyn Fn(&T) -> K + 'static>,
            node_fn: Box<dyn Fn(&T) -> Node + 'static>,
        ) -> KeyedChildren
        where
            T: 'static,
            K: Hash + Eq + 'static,
        {
            let owner_scope = current_scope();

            let mut node_cache: HashMap<K, Node> = HashMap::new();
            let node_refs = computed(move || {
                let list = list.get_clone();
                let mut new_nodes = Vec::with_capacity(list.len());
                let mut prev_node_cache = mem::take(&mut node_cache);
                for value in list.iter() {
                    let key = key_fn(value);
                    let node = prev_node_cache
                        .remove(&key)
                        .unwrap_or_else(|| owner_scope.run(|| node_fn(value)));
                    new_nodes.push(*node);
                    node_cache.insert(key, node);
                }
                new_nodes.into()
            });

            KeyedChildren { node_refs }
        }

        inner(list.into(), Box::new(key_fn), Box::new(node_fn))
    }
}

impl Access for KeyedChildren {
    type Value = Rc<[NodeRef]>;

    fn get_clone(&self) -> Self::Value {
        self.node_refs.get_clone()
    }

    fn get_clone_untracked(&self) -> Self::Value {
        self.node_refs.get_clone_untracked()
    }
}

impl Disposable for KeyedChildren {
    fn dispose(&self) {
        self.node_refs.dispose();
    }
}

impl Children for KeyedChildren {}
