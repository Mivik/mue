use macroquad::prelude::*;

use crate::{hook::on_render, layout::use_layout, Node, Style};

pub fn sprite() -> Node {
    Node::build(move || {
        let layout = use_layout(Style::new());
        on_render(move |_| {
            let r = layout.resolve();
            draw_rectangle(r.x, r.y, r.w, r.h, WHITE);
        });
    })
}
