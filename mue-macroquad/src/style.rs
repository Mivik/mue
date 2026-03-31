mod ext;

use cosmic_text::Align;
pub use ext::StyleableExt;

use mue_core::{
    effect::computed,
    prop::Prop,
    signal::{Access, ReadSignal},
    Owned,
};
use paste::paste;
use std::mem;
use taffy::{BoxSizing, Dimension, Display, Position, Size};

use crate::{
    event::pointer::PointerEvent,
    hook::{HookFn, NodeHooks},
    math::Matrix,
    node::{Children, IntoChildren},
};

pub struct SizeProp<S> {
    width: Prop<S>,
    height: Prop<S>,
    full: Option<Prop<Size<S>>>,
}
impl<S> SizeProp<S>
where
    S: Clone + PartialEq + 'static,
{
    fn spread(&mut self) {
        if let Some(full) = self.full.take() {
            self.width = full.clone().map(|s| s.width);
            self.height = full.map(|s| s.height);
        }
    }

    pub fn width(mut self, value: impl Into<Prop<S>>) -> Self {
        self.spread();
        self.width = value.into();
        self
    }

    pub fn height(mut self, value: impl Into<Prop<S>>) -> Self {
        self.spread();
        self.height = value.into();
        self
    }

    pub fn set(mut self, value: impl Into<Prop<Size<S>>>) -> Self {
        self.full = Some(value.into());
        self
    }

    pub fn build(self) -> Prop<taffy::Size<S>> {
        self.full.unwrap_or_else(|| {
            computed(move |_| taffy::Size {
                width: self.width.get_clone(),
                height: self.height.get_clone(),
            })
            .into()
        })
    }
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
        @hooks {
            $($hook_name:ident : HookFn<$hook_ty:ty>;)*
        }
    ) => {
        #[derive(Default)]
        pub struct Style {
            $(pub(crate) $name: Option<Prop<$ty>>,)*
            $(pub $hook_name: HookFn<$hook_ty>,)*
            pub(crate) children: Option<Owned<Children>>,
        }

        pub trait Styleable: Sized {
            fn style_mut(&mut self) -> &mut Style;

            $(
                fn $name(mut self, value: impl Into<Prop<$ty>>) -> Self {
                    self.style_mut().$name = Some(value.into());
                    self
                }
            )*

            $(
                fn $hook_name(mut self, hook: impl FnMut(&$hook_ty) + 'static) -> Self {
                    self.style_mut().$hook_name.append(hook);
                    self
                }
            )*

            fn children(mut self, children: impl IntoChildren) -> Self {
                self.style_mut().children = Some(children.into_children());
                self
            }
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
                $(
                    self.$hook_name.extend(other.$hook_name);
                )*
                self.children = self.children.or(other.children);
                self
            }

            pub fn provide_defaults(&mut self, mut default: Self) {
                $(
                    if self.$name.is_none() {
                        self.$name = default.$name;
                    }
                )*
                $(
                    mem::swap(&mut self.$hook_name, &mut default.$hook_name);
                    self.$hook_name.extend(default.$hook_name);
                )*
                if self.children.is_none() {
                    self.children = default.children;
                }
            }

            paste! {
                $(
                    #[allow(dead_code)]
                    pub(crate) fn [<take_ $name>](&mut self) -> Prop<$ty> {
                        self.$name.take().unwrap_or_else(|| define_style!(@extract_default ($ty, $($default)?)))
                    }
                )*

                $(
                    #[allow(dead_code)]
                    pub(crate) fn [<take_ $hook_name>](&mut self) -> HookFn<$hook_ty> {
                        mem::take(&mut self.$hook_name)
                    }
                )*
            }
        }
    };
}

define_style! {
    // Taffy
    box_sizing: BoxSizing;

    display: Display;

    width: Dimension = Dimension::auto();
    height: Dimension = Dimension::auto();

    min_width: Dimension = Dimension::auto();
    min_height: Dimension = Dimension::auto();

    align_items: Option<taffy::AlignItems>;
    align_self: Option<taffy::AlignSelf>;
    justify_items: Option<taffy::AlignItems>;
    justify_self: Option<taffy::AlignSelf>;
    align_content: Option<taffy::AlignContent>;
    justify_content: Option<taffy::JustifyContent>;

    position: Position;

    flex_direction: taffy::FlexDirection;
    flex_wrap: taffy::FlexWrap;

    flex_basis: Dimension = Dimension::auto();
    flex_grow: f32 = 0.0;
    flex_shrink: f32 = 1.0;

    // Paint
    transform: Matrix = Matrix::IDENTITY;
    opacity: f32 = 1.0;

    // Text
    text_align: Option<Align>;

    @hooks {
        on_render: HookFn<()>;
        on_pointer_event: HookFn<PointerEvent>;
        on_hover_event: HookFn<PointerEvent>;

        on_tap: HookFn<()>;
        on_long_press: HookFn<()>;
    }
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn build_hooks(&mut self) -> NodeHooks {
        NodeHooks {
            render: self.take_on_render(),
            pointer_event: self.take_on_pointer_event(),
            hover_event: self.take_on_hover_event(),
        }
    }

    pub(crate) fn build_taffy(&mut self) -> ReadSignal<taffy::Style> {
        fn to_size<S>(width: Prop<S>, height: Prop<S>) -> Prop<taffy::Size<S>>
        where
            S: Clone + PartialEq + 'static,
        {
            computed(move |_| taffy::Size {
                width: width.get_clone(),
                height: height.get_clone(),
            })
            .into()
        }

        let size = to_size(self.take_width(), self.take_height());
        let min_size = to_size(self.take_min_width(), self.take_min_height());
        let display = self.take_display();

        let align_items = self.take_align_items();
        let align_self = self.take_align_self();
        let justify_items = self.take_justify_items();
        let justify_self = self.take_justify_self();
        let align_content = self.take_align_content();
        let justify_content = self.take_justify_content();

        let position = self.take_position();

        let flex_direction = self.take_flex_direction();
        let flex_wrap = self.take_flex_wrap();

        let flex_basis = self.take_flex_basis();
        let flex_grow = self.take_flex_grow();
        let flex_shrink = self.take_flex_shrink();

        computed(move |_| taffy::Style {
            display: display.get(),

            size: size.get(),
            min_size: min_size.get(),

            align_items: align_items.get(),
            align_self: align_self.get(),
            justify_items: justify_items.get(),
            justify_self: justify_self.get(),
            align_content: align_content.get(),
            justify_content: justify_content.get(),

            position: position.get(),

            flex_direction: flex_direction.get(),
            flex_wrap: flex_wrap.get(),

            flex_basis: flex_basis.get(),
            flex_grow: flex_grow.get(),
            flex_shrink: flex_shrink.get(),

            ..Default::default()
        })
    }
}
