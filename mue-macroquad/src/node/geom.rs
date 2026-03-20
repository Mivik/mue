use std::rc::Rc;

use macroquad::color::WHITE;
use mue_core::signal::Access;

use crate::{
    hook::on_render,
    layout::{use_layout, Layout},
    paint::{use_paint, Shapes},
    style::Style,
};

#[mue_macros::node]
pub fn circle(style: &mut Style) {
    let Layout { rect, .. } = use_layout(style);
    let paint = use_paint(style);
    let shapes = paint.build(move |p| {
        let r = rect.get();
        p.fill_circle(r.center(), r.w.min(r.h) / 2., WHITE);
    });
    on_render(move |_| {
        shapes.get_clone().draw();
    });
}

#[mue_macros::node]
pub fn shape(shapes: Rc<Shapes>) {
    on_render(move |_| {
        shapes.get_clone().draw();
    });
}
