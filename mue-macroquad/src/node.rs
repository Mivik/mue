mod children;
mod flexbox;
mod sprite;

pub use flexbox::flexbox;
pub use sprite::sprite;

use std::{cell::RefCell, rc::Rc};

use mue_core::{
    effect::{watch_effect, Effect},
    prelude::create_scope,
    signal::{Access, ReadSignal},
    Prop, Scope,
};
use slotmap::new_key_type;

use crate::{hook::Hooks, runtime::Runtime, Layout, Style};

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
    style_signal: ReadSignal<taffy::Style>,
    style_effect: Effect,

    pub children: Prop<Rc<[Node]>>,
}

impl Default for NodeInner {
    fn default() -> Self {
        Self {
            scope: Scope::null(),
            hooks: Hooks::default(),

            style: Style::default(),
            layout: None,
            style_signal: ReadSignal::null(),
            style_effect: Effect::null(),

            children: Prop::Static(Rc::new([])),
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

    pub fn apply_style(&mut self, style: Style) {
        let layout = self
            .layout
            .expect("cannot apply style to a node without layout");
        self.style.merge(style);

        self.style_signal.dispose();
        self.style_signal = self.style.build();
        if !self.scope.is_null() {
            self.scope.push_signal(self.style_signal);
        }

        let style_signal = self.style_signal;
        self.style_effect.dispose();
        self.style_effect = watch_effect(move || {
            Runtime::with(|rt| {
                rt.taffy
                    .borrow_mut()
                    .set_style(layout.id(), style_signal.get_clone())
                    .unwrap();
            });
        });
        if !self.scope.is_null() {
            self.scope.push_effect(self.style_effect);
        }
    }

    pub fn register(self) -> NodeId {
        Runtime::with(|rt| rt.nodes.borrow_mut().insert(self))
    }
}

#[derive(Clone, Copy)]
pub struct Node {
    pub(crate) id: NodeId,
}

impl Node {
    pub fn build(f: impl FnOnce()) -> Self {
        let prev_context = CONTEXT.replace(Some(NodeInner::default()));
        let scope = create_scope(f);
        let mut context = CONTEXT.replace(prev_context).unwrap();
        context.scope = scope;
        Self {
            id: context.register(),
        }
    }

    pub(crate) fn render(&self) {
        Runtime::with(|rt| {
            let node = rt.node(self.id);
            node.hooks.render.invoke(&());
            for child in node.children.get_clone().iter() {
                child.render();
            }
        });
    }

    pub fn dispose(self) {
        Runtime::with(|rt| {
            if let Some(node) = rt.nodes.borrow_mut().remove(self.id) {
                node.scope.dispose();
                for child in node.children.get_clone().iter() {
                    child.dispose();
                }
            }
        });
    }

    pub fn styled(self, style: Style) -> Self {
        Runtime::with(|rt| {
            rt.node_mut(self.id).apply_style(style);
        });
        self
    }
}
