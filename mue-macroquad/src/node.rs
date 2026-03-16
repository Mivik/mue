mod children;
mod flexbox;
mod sprite;

pub use flexbox::flexbox;
pub use sprite::sprite;

use std::{cell::RefCell, ops::Deref, rc::Rc};

use mue_core::{
    effect::{watch_effect, Effect},
    prelude::create_scope,
    signal::{Access, ReadSignal},
    Disposable, Owned, Scope,
};
use slotmap::new_key_type;

use crate::{hook::Hooks, node::children::Children, runtime::Runtime, Layout, Style};

new_key_type! {
    pub(crate) struct NodeId;
}

thread_local! {
    static CONTEXT: RefCell<Option<NodeInner>> = const { RefCell::new(None) };
}

pub(crate) struct NodeInner {
    pub scope: Scope,
    pub hooks: Hooks,

    style: Style,
    pub(crate) layout: Option<Layout>,
    style_signal: Owned<ReadSignal<taffy::Style>>,
    style_effect: Owned<Effect>,

    children: Option<Rc<dyn Children>>,
    layout_children_effect: Owned<Effect>,
}

impl Default for NodeInner {
    fn default() -> Self {
        Self {
            scope: Scope::null(),
            hooks: Hooks::default(),

            style: Style::default(),
            layout: None,
            style_signal: ReadSignal::null().owned(),
            style_effect: Effect::null().owned(),

            children: None,
            layout_children_effect: Effect::null().owned(),
        }
    }
}

impl NodeInner {
    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        CONTEXT.with_borrow_mut(|ctx| {
            let ctx = ctx.as_mut().expect("no node context found");
            f(ctx)
        })
    }

    pub(crate) fn check_layout_children_effect(&mut self) {
        if !self.layout_children_effect.is_null() {
            return;
        }

        let Some(layout) = self.layout else {
            return;
        };
        let Some(children) = self.children.clone() else {
            return;
        };
        self.layout_children_effect = watch_effect(move || {
            Runtime::with(|rt| {
                let mut taffy = rt.taffy.borrow_mut();
                taffy.remove_children_range(layout.id(), ..).unwrap();
                for child_layout in children
                    .get_clone()
                    .iter()
                    .filter_map(|node| rt.node(node.id).layout)
                {
                    taffy.add_child(layout.id(), child_layout.id()).unwrap();
                }
            });
        })
        .owned();
    }

    pub fn set_children(children: Rc<dyn Children>) {
        Self::with_mut(|node| {
            if node.children.is_some() {
                panic!("cannot set children of a node more than once");
            }
            node.children = Some(children);
            node.check_layout_children_effect();
        });
    }

    pub fn apply_style(&mut self, style: Style) {
        let layout = self
            .layout
            .expect("cannot apply style to a node without layout");
        self.style.merge(style);

        self.style_signal = self.style.build().owned();
        if !self.scope.is_null() {
            self.scope.push_signal(*self.style_signal);
        }

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
        if !self.scope.is_null() {
            self.scope.push_effect(*self.style_effect);
        }
    }

    pub fn register(self) -> NodeId {
        Runtime::with(|rt| rt.nodes.borrow_mut().insert(self))
    }
}

#[derive(Clone, Copy)]
pub struct NodeRef {
    pub(crate) id: NodeId,
}

impl NodeRef {
    pub(crate) fn render(&self) {
        Runtime::with(|rt| {
            let node = rt.node(self.id);
            node.hooks.render.invoke(&());
            if let Some(children) = &node.children {
                for child in children.get_clone().iter() {
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
            if let Some(children) = &node.children {
                for child in children.get_clone().iter() {
                    child.dispose();
                }
            }
            if let Some(layout) = node.layout {
                rt.taffy.borrow_mut().remove(layout.id()).unwrap();
            }
            if let Some(children) = &node.children {
                children.dispose();
            }
        });
    }
}

#[repr(transparent)]
pub struct Node(Owned<NodeRef>);

impl Node {
    pub fn build(f: impl FnOnce()) -> Self {
        let prev_context = CONTEXT.replace(Some(NodeInner::default()));
        let scope = create_scope(f);
        let mut context = CONTEXT.replace(prev_context).unwrap();
        context.scope = scope;
        Self(
            NodeRef {
                id: context.register(),
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
}

impl Deref for Node {
    type Target = NodeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
