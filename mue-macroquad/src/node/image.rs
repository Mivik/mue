use macroquad::prelude::*;
use mue_core::{signal::Access, Prop};

use crate::{
    hook::on_render,
    layout::use_layout,
    node::{div, Node},
    prop::{prop, prop_or},
    SharedTexture,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectFit {
    Cover,
    Contain,
    Fill,
    ScaleDown,
    None,
}

#[derive(Clone)]
pub struct ImageRegion(Rect);

pub fn image(texture: impl Into<Prop<SharedTexture>>) -> Node {
    let texture = texture.into();
    let fit = prop_or(ObjectFit::Fill);
    let region = prop::<ImageRegion>();
    Node::build(move || {
        let layout = use_layout();
        on_render(move |_| {
            let r = layout.resolve();
            draw_texture_ex(
                *texture.get_clone(),
                r.x,
                r.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(r.w, r.h)),
                    source: region.as_ref().map(|it| it.get_clone().0),
                    rotation: 0.,
                    flip_x: false,
                    flip_y: false,
                    pivot: None,
                },
            );
        });
    })
}
