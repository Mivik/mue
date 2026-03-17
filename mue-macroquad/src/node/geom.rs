use std::rc::Rc;

use macroquad::color::WHITE;
use mue_core::signal::Access;

use crate::{
    hook::on_render,
    layout::{use_layout, Style},
    paint::{use_paint, Shapes},
    Layout, Point,
};

#[mue_macros::node]
pub fn circle(mut style: Style) {
    let Layout { rect, .. } = use_layout(&mut style);
    let paint = use_paint(&mut style);
    let shapes = paint.build(move |p| {
        let r = rect.get();
        let ct = r.center();
        p.fill_circle(Point::new(ct.x, ct.y), r.w.min(r.h) / 2., WHITE);
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
