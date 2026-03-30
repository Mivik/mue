mod children;
mod container;
mod geom;
pub mod image;
pub mod text;

pub use children::{join_children, map_keyed, show_if, Children, IntoChildren};
pub use container::{div, flexbox, FlexboxBuilder};
pub use geom::circle;
pub use image::image;
pub use text::text;

use std::{cell::RefCell, mem, ops::Deref};

use mue_core::{prop::Prop, scope::Scope, signal::Access, Disposable, Owned};
use slotmap::{new_key_type, Key};

use crate::{
    event::pointer::HitTestFn,
    gesture::{GestureId, TapGesture},
    hook::Hooks,
    layout::OwnedLayout,
    math::{Rect, Vector},
    runtime::Runtime,
    style::{Style, Styleable, StyleableExt},
};

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
    pub(crate) hit_test: Option<HitTestFn>,
    pub(crate) gestures: Vec<GestureId>,

    pub(crate) children: Owned<Children>,
}

impl NodeInner {
    pub fn new(children: Owned<Children>) -> Self {
        Self {
            scope: Scope::new(),
            hooks: Hooks::default(),

            layout_id: None,
            layout: OwnedLayout::default(),
            hit_test: None,
            gestures: Vec::new(),

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
    pub(crate) fn null() -> Self {
        Self { id: NodeId::null() }
    }

    pub(crate) fn render(&self, mut origin: Vector) {
        Runtime::with(|rt| {
            let node = rt.node(self.id);
            if let Some(layout_id) = node.layout_id {
                let taffy = rt.taffy.borrow();
                let layout = taffy.layout(layout_id).unwrap();
                let loc = layout.location;
                let size = layout.size;
                origin.x += loc.x;
                origin.y += loc.y;
                node.layout
                    .rect
                    .set(Rect::new(origin.x, origin.y, size.width, size.height));
            }
            node.hooks.render.invoke(&());
            if !node.children.is_null() {
                for child in node.children.get_clone().iter() {
                    child.render(origin);
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
            for gesture_id in node.gestures {
                rt.gestures.borrow_mut().remove(gesture_id);
            }
        });
    }
}

#[repr(transparent)]
pub struct Node(Owned<NodeRef>);

impl Node {
    pub fn build(f: impl FnOnce()) -> Self {
        Self::build_with_style(Style::default(), |_style| f())
    }

    pub fn build_with_style(mut style: Style, f: impl FnOnce(&mut Style)) -> Self {
        let node = NodeInner::new(style.children.take().unwrap_or_else(|| ().into_children()));
        let scope = node.scope;
        CONTEXT.with_borrow_mut(|stack| stack.push(Box::new(node)));
        scope.run(|| f(&mut style));
        let mut node = CONTEXT.with_borrow_mut(|stack| stack.pop().unwrap());
        append_hooks(node.as_mut(), &mut style);
        Self(
            NodeRef {
                id: node.register(),
            }
            .owned(),
        )
    }
}

fn append_hooks(node: &mut NodeInner, style: &mut Style) {
    Runtime::with(|rt| {
        let mut gestures = rt.gestures.borrow_mut();
        let mut insert = |gesture| node.gestures.push(gestures.insert(gesture));
        if !style.on_click.is_empty()
            || !style.on_tap_down.is_empty()
            || !style.on_tap_up.is_empty()
            || !style.on_tap_cancel.is_empty()
        {
            let mut gesture = Box::new(TapGesture::default());
            gesture.on_click = mem::take(&mut style.on_click);
            gesture.on_tap_down = mem::take(&mut style.on_tap_down);
            gesture.on_tap_up = mem::take(&mut style.on_tap_up);
            gesture.on_tap_cancel = mem::take(&mut style.on_tap_cancel);
            insert(gesture);
        }
    })
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

pub trait Component: IntoNode + Styleable + StyleableExt {}
impl<T: IntoNode + Styleable + StyleableExt> Component for T {}
