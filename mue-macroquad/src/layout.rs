use std::{cell::RefCell, mem};

use macroquad::math::Rect;
use mue_core::prelude::*;
use taffy::Dimension;

use crate::{node::NodeContext, runtime::Runtime, Node};

pub fn size<S: Clone + PartialEq + 'static>(
    width: Prop<S>,
    height: Prop<S>,
) -> Prop<taffy::Size<S>> {
    // TODO: optimize when input are static?
    computed(move || taffy::Size {
        width: width.get_clone(),
        height: height.get_clone(),
    })
    .into()
}

macro_rules! define_style {
    (@extract_default ($ty:ty, $default:expr)) => {
        Prop::Static($default)
    };

    (@extract_default ($ty:ty, )) => {
        Prop::Static(<$ty>::default())
    };

    (
        $($name:ident : $ty:ty $( = $default:expr)?;)*
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct Style {
            $(pub $name: Option<Prop<$ty>>),*
        }

        struct ComputedStyle {
            $(pub $name: Prop<$ty>),*
        }

        impl Style {
            $(
                pub fn $name(mut self, value: impl Into<Prop<$ty>>) -> Self {
                    self.$name = Some(value.into());
                    self
                }
            )*

            pub fn merge(&mut self, other: Self) {
                $(
                    self.$name = self.$name.or(other.$name);
                )*
            }

            fn compute(self) -> ComputedStyle {
                ComputedStyle {
                    $(
                        $name: self.$name.unwrap_or_else(|| define_style!(@extract_default ($ty, $($default)?))),
                    )*
                }
            }
        }
    };
}

define_style! {
    width: Dimension = Dimension::auto();
    height: Dimension = Dimension::auto();
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn build(self) -> ReadSignal<taffy::Style> {
        let style = self.compute();
        let size = size(style.width, style.height);
        computed(move || taffy::Style {
            size: size.get(),
            ..Default::default()
        })
    }
}

pub struct Layout {
    id: taffy::NodeId,
}

impl Layout {
    pub(crate) fn new(id: taffy::NodeId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> taffy::NodeId {
        self.id
    }

    pub fn resolve(&self) -> Rect {
        Runtime::with(|rt| {
            let taffy = rt.taffy.borrow();
            let taffy::Layout { location, size, .. } = taffy.layout(self.id).unwrap();
            Rect::new(location.x, location.y, size.width, size.height)
        })
    }
}

thread_local! {
    static STYLE: RefCell<Style> = RefCell::default();
}

pub fn use_layout(mut style: Style) -> Layout {
    STYLE.with_borrow_mut(|s| style.merge(mem::take(s)));
    let style = style.build();

    NodeContext::with_mut(|ctx| {
        let layout_id =
            Runtime::with(|rt| rt.taffy.borrow_mut().new_leaf(style.get_clone()).unwrap());
        watch(Prop::Dynamic(style), move |_| {
            Runtime::with(|rt| {
                rt.taffy
                    .borrow_mut()
                    .set_style(layout_id, style.get_clone())
                    .unwrap();
            });
        });

        ctx.layout_ids
            .get_mut()
            .expect("cannot mix use_layout and custom layout ids")
            .push(layout_id);

        Layout::new(layout_id)
    })
}

pub fn styled(style: Style, f: impl FnOnce() -> Node) -> Node {
    STYLE.with_borrow_mut(|s| s.merge(style));
    f()
}
