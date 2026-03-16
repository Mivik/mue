use std::cell::RefCell;

use lyon::{
    path::PathEvent,
    tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
        StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
    },
};
use macroquad::prelude::*;

use crate::{
    shader::{IntoShader, Shader},
    Matrix, Point, Vector,
};

thread_local! {
    static PAINT: RefCell<Paint> = RefCell::new(Paint::new());
}

struct ShaderConstructor<T: Shader> {
    transform: Matrix,
    shader: T,
    alpha: f32,
}
impl<T: Shader> FillVertexConstructor<Vertex> for ShaderConstructor<T> {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        let pos = vertex.position();
        self.shader
            .new_vertex(&self.transform, &Point::new(pos.x, pos.y), self.alpha)
    }
}
impl<T: Shader> StrokeVertexConstructor<Vertex> for ShaderConstructor<T> {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        let pos = vertex.position();
        self.shader
            .new_vertex(&self.transform, &Point::new(pos.x, pos.y), self.alpha)
    }
}

pub struct Paint {
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
    vertex_buffers: VertexBuffers<Vertex, u16>,

    transforms: Vec<Matrix>,
    alpha: f32,
}

impl Default for Paint {
    fn default() -> Self {
        Self::new()
    }
}

impl Paint {
    pub fn new() -> Self {
        Self {
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            vertex_buffers: VertexBuffers::new(),

            transforms: vec![Matrix::identity()],
            alpha: 1.,
        }
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut Self) -> R) -> R {
        PAINT.with_borrow_mut(f)
    }

    pub fn with_transform<T>(&mut self, transform: Matrix, f: impl FnOnce(&mut Self) -> T) -> T {
        let last = self.transforms.last().unwrap();
        self.transforms.push(last * transform);
        let result = f(self);
        self.transforms.pop();
        result
    }

    fn emit_lyon(&mut self, texture: Option<Texture2D>) {
        let gl = unsafe { get_internal_gl() }.quad_gl;
        gl.texture(texture);
        gl.draw_mode(DrawMode::Triangles);
        gl.geometry(
            &std::mem::take(&mut self.vertex_buffers.vertices),
            &std::mem::take(&mut self.vertex_buffers.indices),
        );
    }

    fn draw_lyon<T: Shader>(&mut self, shader: T, f: impl FnOnce(&mut Self, f32, ShaderConstructor<T>)) {
        let scale = self
            .transforms
            .last()
            .unwrap()
            .svd_unordered(false, false)
            .singular_values
            .max();
        let tolerance = 0.2 / scale;

        let shaded = ShaderConstructor {
            transform: *self.transforms.last().unwrap(),
            shader,
            alpha: self.alpha,
        };
        let tex = shaded.shader.texture();
        f(self, tolerance, shaded);
        self.emit_lyon(tex);
    }

    pub fn fill_path(
        &mut self,
        path: impl IntoIterator<Item = PathEvent>,
        shader: impl IntoShader,
    ) {
        self.draw_lyon(shader.into_shader(), |this, tol, shaded| {
            this.fill_tess
                .tessellate(
                    path,
                    &FillOptions::tolerance(tol),
                    &mut BuffersBuilder::new(&mut this.vertex_buffers, shaded),
                )
                .unwrap();
        });
    }

    pub fn fill_circle(&mut self, center: Point, radius: f32, shader: impl IntoShader) {
        self.draw_lyon(shader.into_shader(), |this,tol, shaded| {
            this.fill_tess
                .tessellate_circle(
                    lyon::geom::point(center.x, center.y),
                    radius,
                    &FillOptions::tolerance(tol),
                    &mut BuffersBuilder::new(&mut this.vertex_buffers, shaded),
                )
                .unwrap();
        });
    }
}
