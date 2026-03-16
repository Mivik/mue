use macroquad::math::Rect;
use mue_core::prelude::*;
use taffy::Dimension;

use crate::{node::NodeInner, runtime::Runtime};

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

    align_items: Option<taffy::AlignItems>;
    align_self: Option<taffy::AlignSelf>;
    justify_items: Option<taffy::AlignItems>;
    justify_self: Option<taffy::AlignSelf>;
    align_content: Option<taffy::AlignContent>;
    justify_content: Option<taffy::JustifyContent>;

    flex_direction: taffy::FlexDirection;
    flex_wrap: taffy::FlexWrap;

    flex_basis: Dimension = Dimension::auto();
    flex_grow: f32 = 0.0;
    flex_shrink: f32 = 1.0;
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

            align_items: style.align_items.get_clone(),
            align_self: style.align_self.get_clone(),
            justify_items: style.justify_items.get_clone(),
            justify_self: style.justify_self.get_clone(),
            align_content: style.align_content.get_clone(),
            justify_content: style.justify_content.get_clone(),

            flex_direction: style.flex_direction.get_clone(),
            flex_wrap: style.flex_wrap.get_clone(),

            flex_basis: style.flex_basis.get_clone(),
            flex_grow: style.flex_grow.get_clone(),
            flex_shrink: style.flex_shrink.get_clone(),

            ..Default::default()
        })
    }
}

#[derive(Clone, Copy)]
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

pub fn use_layout() -> Layout {
    NodeInner::with_mut(|node| {
        if node.layout.is_some() {
            panic!("use_layout can only be called once per node");
        }

        let layout_id =
            Runtime::with(|rt| rt.taffy.borrow_mut().new_leaf(Default::default()).unwrap());
        let layout = Layout::new(layout_id);

        node.layout = Some(layout);
        // Setup style effect
        node.apply_style(Style::new());
        node.check_layout_children_effect();
        layout
    })
}
