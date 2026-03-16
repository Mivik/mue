mod children;
mod container;
mod geoemetry;
pub mod image;

pub use children::{join_children, map_keyed, show_if, Children, IntoChildren};
pub use container::{div, flexbox};
pub use geoemetry::circle;
pub use image::image;

use std::{cell::RefCell, ops::Deref};

use mue_core::{
    effect::{watch_effect, Effect},
    signal::{Access, ReadSignal},
    Disposable, Owned, Prop, Scope,
};
use slotmap::new_key_type;

use crate::{hook::Hooks, runtime::Runtime, Layout, Style};

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

    style: Style,
    pub(crate) layout: Option<Layout>,
    style_signal: Owned<ReadSignal<taffy::Style>>,
    style_effect: Owned<Effect>,

    children: Owned<Children>,
    layout_children_effect: Owned<Effect>,
}

impl Default for NodeInner {
    fn default() -> Self {
        Self {
            scope: Scope::new(),
            hooks: Hooks::default(),

            style: Style::default(),
            layout: None,
            style_signal: ReadSignal::null().owned(),
            style_effect: Effect::null().owned(),

            children: ReadSignal::null().owned(),
            layout_children_effect: Effect::null().owned(),
        }
    }
}

impl NodeInner {
    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        let mut ctx = CONTEXT.with_borrow_mut(|stack| stack.pop().expect("no node context found"));
        let result = f(ctx.as_mut());
        CONTEXT.with_borrow_mut(|stack| stack.push(ctx));
        result
    }

    pub(crate) fn check_layout_children_effect(&mut self) {
        if !self.layout_children_effect.is_null() {
            return;
        }

        let Some(layout) = self.layout else {
            return;
        };
        let children = *self.children;
        if children.is_null() {
            return;
        }
        self.layout_children_effect = self
            .scope
            .run(|| {
                watch_effect(move || {
                    Runtime::with(|rt| {
                        let children = children.get_clone();
                        let mut taffy = rt.taffy.borrow_mut();
                        taffy.remove_children_range(layout.id(), ..).unwrap();
                        for child_layout in
                            children.iter().filter_map(|node| rt.node(node.id).layout)
                        {
                            taffy.add_child(layout.id(), child_layout.id()).unwrap();
                        }
                    });
                })
            })
            .owned();
    }

    pub fn set_children(children: Owned<Children>) {
        Self::with_mut(|node| {
            if !node.children.is_null() {
                panic!("cannot set children of a node more than once");
            }
            node.children = children;
            node.check_layout_children_effect();
        });
    }

    pub fn apply_style(&mut self, style: Style) {
        let layout = self
            .layout
            .expect("cannot apply style to a node without layout");
        self.style.merge(style);

        self.scope.run(|| {
            self.style_signal = self.style.build().owned();

            let style_signal = *self.style_signal;
            self.style_effect = watch_effect(move || {
                Runtime::with(|rt| {
                    rt.taffy
                        .borrow_mut()
                        .set_style(layout.id(), style_signal.get_clone())
                        .unwrap();
                });
            })
            .owned();
        });
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
        Runtime::with(|rt| {
            let Some(node) = rt.nodes.borrow_mut().remove(self.id) else {
                return;
            };
            node.scope.dispose();
            if let Some(layout) = node.layout {
                rt.taffy.borrow_mut().remove(layout.id()).unwrap();
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
        let node = NodeInner::default();
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

    pub fn styled(self, style: Style) -> Self {
        Runtime::with(|rt| {
            rt.node_mut(self.id).apply_style(style);
        });
        self
    }

    pub fn show_if(self, condition: impl Into<Prop<bool>>) -> Owned<Children> {
        children::show_if(condition, self)
    }
}

impl Deref for Node {
    type Target = NodeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
