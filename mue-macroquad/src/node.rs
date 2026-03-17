mod children;
mod container;
mod geom;
pub mod image;

pub use children::{join_children, map_keyed, show_if, Children, IntoChildren};
pub use container::{div, flexbox, FlexboxBuilder};
pub use geom::circle;
pub use image::image;

use std::{cell::RefCell, ops::Deref};

use macroquad::math::Rect;
use mue_core::{prop::Prop, scope::Scope, signal::Access, Disposable, Owned};
use slotmap::new_key_type;

use crate::{hook::Hooks, layout::OwnedLayout, runtime::Runtime};

new_key_type! {
    pub(crate) struct NodeId;
}

thread_local! {
    #[allow(clippy::vec_box)]
    static CONTEXT: RefCell<Vec<Box<NodeInner>>> = RefCell::default();
}

pub(crate) struct NodeInner {
    pub scope: Scope,
    pub hooks: Hooks,

    pub(crate) layout_id: Option<taffy::NodeId>,
    pub(crate) layout: OwnedLayout,

    pub(crate) children: Owned<Children>,
}

impl NodeInner {
    pub fn new(children: Owned<Children>) -> Self {
        Self {
            scope: Scope::new(),
            hooks: Hooks::default(),
            layout_id: None,
            layout: OwnedLayout::default(),

            children,
        }
    }

    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        let mut ctx = CONTEXT.with_borrow_mut(|stack| stack.pop().expect("no node context found"));
        let result = f(ctx.as_mut());
        CONTEXT.with_borrow_mut(|stack| stack.push(ctx));
        result
    }

    pub fn register(self) -> NodeId {
        Runtime::with(|rt| rt.nodes.borrow_mut().insert(self))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NodeRef {
    pub(crate) id: NodeId,
}

impl NodeRef {
    pub(crate) fn render(&self) {
        Runtime::with(|rt| {
            let node = rt.node(self.id);
            node.hooks.render.invoke(&());
            if let Some(layout_id) = node.layout_id {
                let taffy = rt.taffy.borrow();
                let layout = taffy.layout(layout_id).unwrap();
                let loc = layout.location;
                let size = layout.size;
                node.layout
                    .rect
                    .set(Rect::new(loc.x, loc.y, size.width, size.height));
            }
            if !node.children.is_null() {
                for child in node.children.get_clone().iter() {
                    child.render();
                }
            }
        });
    }
}

impl Disposable for NodeRef {
    fn dispose(&self) {
        let _ = Runtime::try_with(|rt| {
            let Some(node) = rt.nodes.borrow_mut().remove(self.id) else {
                return;
            };
            node.scope.dispose();
            if let Some(layout_id) = node.layout_id {
                rt.taffy.borrow_mut().remove(layout_id).unwrap();
            }
            if !node.children.is_null() {
                for child in node.children.get_clone().iter() {
                    child.dispose();
                }
                node.children.dispose();
            }
        });
    }
}

#[repr(transparent)]
pub struct Node(Owned<NodeRef>);

impl Node {
    pub fn build(f: impl FnOnce()) -> Self {
        Self::build_with_children(Children::null().owned(), f)
    }

    pub fn build_with_children(children: Owned<Children>, f: impl FnOnce()) -> Self {
        let node = NodeInner::new(children);
        let scope = node.scope;
        CONTEXT.with_borrow_mut(|stack| stack.push(Box::new(node)));
        scope.run(f);
        let node = CONTEXT.with_borrow_mut(|stack| stack.pop().unwrap());
        Self(
            NodeRef {
                id: node.register(),
            }
            .owned(),
        )
    }
}

impl Deref for Node {
    type Target = NodeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait IntoNode {
    fn into_node(self) -> Node;

    fn show_if(self, condition: impl Into<Prop<bool>>) -> Owned<Children>
    where
        Self: Sized,
    {
        children::show_if(condition.into(), self.into_node())
    }
}

impl IntoNode for Node {
    fn into_node(self) -> Node {
        self
    }
}
