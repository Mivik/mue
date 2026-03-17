use macroquad::math::Rect;
use mue_core::prelude::*;
use paste::paste;
use taffy::{Dimension, Display};

use crate::{node::NodeInner, runtime::Runtime, Matrix};

pub fn size<S: Clone + PartialEq + 'static>(
    width: Prop<S>,
    height: Prop<S>,
) -> Prop<taffy::Size<S>> {
    // TODO: optimize when input are static?
    computed(move |_| taffy::Size {
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
            $(pub(crate) $name: Option<Prop<$ty>>),*
        }

        pub trait Styleable: Sized {
            fn style_mut(&mut self) -> &mut Style;

            $(
                fn $name(mut self, value: impl IntoProp<$ty>) -> Self {
                    self.style_mut().$name = Some(value.into_prop());
                    self
                }
            )*
        }

        impl Styleable for Style {
            fn style_mut(&mut self) -> &mut Style {
                self
            }
        }

        impl Style {
            pub fn merge(mut self, other: Self) -> Self {
                $(
                    self.$name = self.$name.or(other.$name);
                )*
                self
            }

            $(
                paste! {
                    #[allow(dead_code)]
                    pub(crate) fn [<take_ $name>](&mut self) -> Prop<$ty> {
                        self.$name.take().unwrap_or_else(|| define_style!(@extract_default ($ty, $($default)?)))
                    }
                }
            )*
        }
    };
}

define_style! {
    // Taffy
    display: Display;

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

    // Paint
    transform: Matrix = Matrix::identity();
    opacity: f32 = 1.0;
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn build_taffy(&mut self) -> ReadSignal<taffy::Style> {
        let size: Prop<taffy::Size<Dimension>> = size(self.take_width(), self.take_height());
        let display = self.take_display();

        let align_items = self.take_align_items();
        let align_self = self.take_align_self();
        let justify_items = self.take_justify_items();
        let justify_self = self.take_justify_self();
        let align_content = self.take_align_content();
        let justify_content = self.take_justify_content();

        let flex_direction = self.take_flex_direction();
        let flex_wrap = self.take_flex_wrap();

        let flex_basis = self.take_flex_basis();
        let flex_grow = self.take_flex_grow();
        let flex_shrink = self.take_flex_shrink();

        computed(move |_| taffy::Style {
            display: display.get(),

            size: size.get(),

            align_items: align_items.get(),
            align_self: align_self.get(),
            justify_items: justify_items.get(),
            justify_self: justify_self.get(),
            align_content: align_content.get(),
            justify_content: justify_content.get(),

            flex_direction: flex_direction.get(),
            flex_wrap: flex_wrap.get(),

            flex_basis: flex_basis.get(),
            flex_grow: flex_grow.get(),
            flex_shrink: flex_shrink.get(),

            ..Default::default()
        })
    }
}

#[non_exhaustive]
pub struct Layout {
    pub rect: ReadSignal<Rect>,
}

pub(crate) struct OwnedLayout {
    pub rect: Signal<Rect>,
}

impl Default for OwnedLayout {
    fn default() -> Self {
        Self {
            rect: Signal::null(),
        }
    }
}

pub fn use_layout(style: &mut Style) -> Layout {
    let style = style.build_taffy();
    NodeInner::with_mut(|node| {
        if node.layout_id.is_some() {
            panic!("use_layout can only be called once per node");
        }

        let layout_id =
            Runtime::with(|rt| rt.taffy.borrow_mut().new_leaf(Default::default()).unwrap());
        watch_effect(move || {
            Runtime::with(|rt| {
                rt.taffy
                    .borrow_mut()
                    .set_style(layout_id, style.get_clone())
                    .unwrap();
            });
        });

        node.layout_id = Some(layout_id);
        node.layout = OwnedLayout {
            rect: signal(Rect::default()),
        };

        let children = *node.children;
        node.scope.run(|| {
            watch_effect(move || {
                Runtime::with(|rt| {
                    let children = children.get_clone();
                    let mut taffy = rt.taffy.borrow_mut();
                    taffy.remove_children_range(layout_id, ..).unwrap();
                    for child_layout_id in children
                        .iter()
                        .filter_map(|node| rt.node(node.id).layout_id)
                    {
                        taffy.add_child(layout_id, child_layout_id).unwrap();
                    }
                });
            });
        });

        Layout {
            rect: *node.layout.rect,
        }
    })
}
