use std::{
    cell::{Ref, RefCell},
    thread::AccessError,
};

use slotmap::SlotMap;
use taffy::TaffyTree;

use crate::{
    layout::MeasureFn,
    node::{text::FontState, NodeId, NodeInner},
};

thread_local! {
    static RUNTIME: Runtime = Runtime::new();
}

pub(crate) struct Runtime {
    pub taffy: RefCell<TaffyTree<Box<dyn MeasureFn>>>,
    pub nodes: RefCell<SlotMap<NodeId, NodeInner>>,

    pub fonts: RefCell<FontState>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            taffy: RefCell::new(TaffyTree::new()),
            nodes: RefCell::new(SlotMap::with_key()),

            fonts: RefCell::new(FontState::default()),
        }
    }

    pub fn with<R>(f: impl FnOnce(&Runtime) -> R) -> R {
        RUNTIME.with(f)
    }
    pub fn try_with<R>(f: impl FnOnce(&Runtime) -> R) -> Result<R, AccessError> {
        RUNTIME.try_with(f)
    }

    pub fn with_taffy_mut<R>(f: impl FnOnce(&mut TaffyTree<Box<dyn MeasureFn>>) -> R) -> R {
        RUNTIME.with(|rt| f(&mut rt.taffy.borrow_mut()))
    }
    pub fn with_fonts_mut<R>(f: impl FnOnce(&mut FontState) -> R) -> R {
        RUNTIME.with(|rt| f(&mut rt.fonts.borrow_mut()))
    }

    pub fn node(&self, id: NodeId) -> Ref<'_, NodeInner> {
        Ref::map(self.nodes.borrow(), |arena| &arena[id])
    }
}
