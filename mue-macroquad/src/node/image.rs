use macroquad::prelude::*;
use mue_core::{prelude::Access, prop::PropValue};

use crate::{
    hook::on_render,
    layout::{use_layout, Layout},
    math::{vec2, Rect, Vector},
    paint::use_paint,
    shader::{SharedTexture, TextureShader},
    style::Style,
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

impl PropValue for ObjectFit {}

#[mue_macros::node]
pub fn image(
    mut style: Style,
    texture: SharedTexture,
    #[default] object_fit: ObjectFit,
    #[default(vec2(0.5, 0.5))] object_position: Vector,
    #[default] region: Option<Rect>,
    #[default(WHITE)] color: Color,
) {
    let Layout { rect, .. } = use_layout(&mut style);
    let paint = use_paint(&mut style);

    let texture_clone = texture.clone();
    let shapes = paint.build(move |p| {
        let texture = texture_clone.get_clone();
        let region = region
            .get_clone()
            .unwrap_or_else(|| Rect::new(0., 0., texture.width(), texture.height()));

        let uv_region = Rect::new(
            region.x / texture.width(),
            region.y / texture.height(),
            region.w / texture.width(),
            region.h / texture.height(),
        );
        let (draw_rect, adjusted_uv) = calculate_draw_rect_and_uv(
            object_fit.get(),
            object_position.get(),
            rect.get(),
            region.w,
            region.h,
            uv_region,
        );

        let shader = TextureShader::new(texture, adjusted_uv, draw_rect, color.get());
        p.fill_rect(draw_rect, shader);
    });

    on_render(move |_| {
        shapes.get_clone().draw();
    });
}

fn calculate_draw_rect_and_uv(
    object_fit: ObjectFit,
    object_position: Vector,
    container: Rect,
    image_width: f32,
    image_height: f32,
    uv_region: Rect,
) -> (Rect, Rect) {
    let container_aspect = container.w / container.h;
    let image_aspect = image_width / image_height;

    let (draw_width, draw_height) = match object_fit {
        ObjectFit::Fill => (container.w, container.h),
        ObjectFit::Cover => {
            if image_aspect > container_aspect {
                let h = container.h;
                (h * image_aspect, h)
            } else {
                let w = container.w;
                (w, w / image_aspect)
            }
        }
        ObjectFit::Contain => {
            if image_aspect > container_aspect {
                let w = container.w;
                (w, w / image_aspect)
            } else {
                let h = container.h;
                (h * image_aspect, h)
            }
        }
        ObjectFit::ScaleDown => {
            if image_aspect > container_aspect {
                let w = container.w.min(image_width);
                (w, w / image_aspect)
            } else {
                let h = container.h.min(image_height);
                (h * image_aspect, h)
            }
        }
        ObjectFit::None => (image_width, image_height),
    };

    let offset_x = (container.w - draw_width) * object_position.x;
    let offset_y = (container.h - draw_height) * object_position.y;
    let draw_rect = Rect::new(
        container.x + offset_x,
        container.y + offset_y,
        draw_width,
        draw_height,
    );
    let draw_rect_clamped = draw_rect.intersect(&container).unwrap();
    let uv_rect = Rect::new(
        uv_region.x + (draw_rect_clamped.x - draw_rect.x) / draw_rect.w * uv_region.w,
        uv_region.y + (draw_rect_clamped.y - draw_rect.y) / draw_rect.h * uv_region.h,
        draw_rect_clamped.w / draw_rect.w * uv_region.w,
        draw_rect_clamped.h / draw_rect.h * uv_region.h,
    );
    (draw_rect_clamped, uv_rect)
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
