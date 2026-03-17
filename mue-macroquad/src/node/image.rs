use macroquad::prelude::*;
use mue_core::prelude::Access;

use crate::{
    hook::on_render,
    layout::{use_layout, Style},
    SharedTexture,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ObjectFit {
    #[default]
    Fill,
    Cover,
    Contain,
    ScaleDown,
    None,
}

#[mue_macros::node]
pub fn image(
    style: Style,
    texture: SharedTexture,
    #[default] object_fit: ObjectFit,
    #[default] region: Option<Rect>,
) {
    let layout = use_layout(style);
    on_render(move |_| {
        let r = layout.resolve();
        draw_texture_ex(
            *texture.get_clone(),
            r.x,
            r.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(r.w, r.h)),
                source: region.get_clone(),
                ..Default::default()
            },
        );
    });
}

impl ImageBuilder {
    pub fn object_fill(self) -> Self {
        self.object_fit(ObjectFit::Fill)
    }

    pub fn object_cover(self) -> Self {
        self.object_fit(ObjectFit::Cover)
    }

    pub fn object_contain(self) -> Self {
        self.object_fit(ObjectFit::Contain)
    }

    pub fn object_scale_down(self) -> Self {
        self.object_fit(ObjectFit::ScaleDown)
    }

    pub fn object_none(self) -> Self {
        self.object_fit(ObjectFit::None)
    }
}
