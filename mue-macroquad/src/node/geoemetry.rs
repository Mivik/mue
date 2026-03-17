use crate::{hook::on_render, layout::{use_layout, Style}, Paint, Point};

#[mue_macros::node]
pub fn circle(style: Style) {
    let layout = use_layout(style);
    on_render(move |_| {
        let r = layout.resolve();
        Paint::with_mut(|p| {
            let ct = r.center();
            p.fill_circle(
                Point::new(ct.x, ct.y),
                r.w.min(r.h) / 2.,
                macroquad::prelude::WHITE,
            );
        });
    });
}
