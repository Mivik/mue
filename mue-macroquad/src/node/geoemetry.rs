use crate::{hook::on_render, layout::use_layout, node::Node, Paint, Point};

pub fn circle() -> Node {
    Node::build(|| {
        let layout = use_layout();

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
    })
}
