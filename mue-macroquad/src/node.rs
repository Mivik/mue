mod flexbox;
mod sprite;

pub use flexbox::flexbox;
pub use sprite::sprite;

use std::cell::RefCell;

use mue_core::{effect::Effect, Prop, Scope};
use slotmap::new_key_type;
use smallvec::SmallVec;

use crate::{hook::Hooks, runtime::Runtime};

new_key_type! {
    pub(crate) struct NodeId;
}

thread_local! {
    static CONTEXT: RefCell<Option<NodeContext>> = const { RefCell::new(None) };
}

pub(crate) struct NodeInner {
    pub scope: Scope,
    pub hooks: Hooks,
    pub children: Vec<Node>,
    pub layout_ids: Prop<SmallVec<[taffy::NodeId; 1]>>,
    pub style_effect: Effect,
}

impl NodeInner {
    pub fn new(scope: Scope, context: NodeContext) -> Self {
        Self {
            scope,
            hooks: context.hooks,
            children: context.children,
            layout_ids: context.layout_ids,
            style_effect: Effect::null(),
        }
    }

    pub fn register(self) -> NodeId {
        Runtime::with(|rt| rt.nodes.borrow_mut().insert(self))
    }
}

#[derive(Default)]
pub(crate) struct NodeContext {
    pub hooks: Hooks,
    pub children: Vec<Node>,
    pub layout_ids: Prop<SmallVec<[taffy::NodeId; 1]>>,
}

impl NodeContext {
    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        CONTEXT.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().expect("no node context found");
            f(ctx)
        })
    }

    pub(crate) fn set_children( children: Vec<Node>) {
        Self::with_mut(|ctx| ctx.children = children);
    }
}

#[derive(Clone, Copy)]
pub struct Node {
    pub(crate) id: NodeId,
}

impl Node {
    pub fn build(f: impl FnOnce()) -> Self {
        let mut scope = Scope::new();
        let context = scope.run(|| {
            let prev_context = CONTEXT.replace(Some(NodeContext::default()));
            f();
            CONTEXT.replace(prev_context).unwrap()
        });
        let id = NodeInner::new(scope, context).register();
        Self { id }
    }

    pub(crate) fn render(&self) {
        Runtime::with(|rt| {
            let node = rt.node(self.id);
            node.hooks.render.invoke(&());
            for child in &node.children {
                child.render();
            }
        });
    }
}
