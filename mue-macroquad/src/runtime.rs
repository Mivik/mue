use std::cell::{Ref, RefCell, RefMut};

use slotmap::SlotMap;
use taffy::TaffyTree;

use crate::node::{NodeId, NodeInner};

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

pub(crate) struct Runtime {
    pub taffy: RefCell<TaffyTree<()>>,
    pub nodes: RefCell<SlotMap<NodeId, NodeInner>>,
}

impl Runtime {
    fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.disable_rounding();

        Self {
            taffy: RefCell::new(taffy),
            nodes: RefCell::new(SlotMap::with_key()),
        }
    }

    pub fn with<R>(f: impl FnOnce(&Runtime) -> R) -> R {
        RUNTIME.with(f)
    }

    pub fn node(&self, id: NodeId) -> Ref<'_, NodeInner> {
        Ref::map(self.nodes.borrow(), |arena| &arena[id])
    }

    pub fn node_mut(&self, id: NodeId) -> RefMut<'_, NodeInner> {
        RefMut::map(self.nodes.borrow_mut(), |arena| &mut arena[id])
    }
}
