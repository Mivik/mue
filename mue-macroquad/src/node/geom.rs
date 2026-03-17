use std::rc::Rc;

use macroquad::color::WHITE;
use mue_core::{prop::Prop, signal::Access};

use crate::{
    hook::on_render,
    layout::{use_layout, Style},
    paint::{use_paint, Shape},
    shader::IntoShader,
    Layout, Point,
};

#[mue_macros::node]
pub fn circle(mut style: Style) {
    let Layout { rect, .. } = use_layout(&mut style);
    let paint = use_paint(&mut style);
    paint.build_fill_circle(
        rect.map(|r| Point::new(r.center().x, r.center().y)),
        rect.map(|r| r.w.min(r.h) / 2.),
        Prop::Static(WHITE.into_shader()),
    );
    on_render(move |_| {
        let r = rect.get();
        paint.with(|p| {
            let ct = r.center();
            p.fill_circle(
                Point::new(ct.x, ct.y),
                r.w.min(r.h) / 2.,
                macroquad::prelude::WHITE,
            );
        });
    });
}

#[mue_macros::node]
pub fn shape(shape: Rc<Shape>) {
    on_render(move |_| {
        shape.get_clone().draw();
    });
}
