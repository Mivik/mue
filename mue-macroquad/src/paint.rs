use std::{cell::RefCell, rc::Rc};

use lyon::{
    path::PathEvent,
    tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
        StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
    },
};
use macroquad::prelude::*;
use mue_core::{
    effect::{computed, computed_always},
    scope::{inject, provide_with},
    signal::{Access, ReadSignal},
    IntoProp, Prop,
};

use crate::{
    shader::{IntoShader, Shader},
    Matrix, Point, SharedTexture, Style,
};

thread_local! {
    static PAINT: RefCell<Paint> = RefCell::new(Paint::new());
}

#[derive(Clone)]
#[repr(transparent)]
struct Transform(Prop<Matrix>);

pub fn with_transform(transform: impl IntoProp<Matrix>) {
    let transform = transform.into_prop();
    provide_with::<Transform>(|prev| {
        Transform(match prev.cloned() {
            Some(Transform(prev)) => computed(move |_| prev.get() * transform.get()).into(),
            None => transform,
        })
    });
}

#[derive(Clone)]
struct Opacity(Prop<f32>);

pub fn with_opacity(opacity: impl IntoProp<f32>) {
    let opacity = opacity.into_prop();
    provide_with::<Opacity>(|prev| {
        Opacity(match prev.cloned() {
            Some(Opacity(prev)) => computed(move |_| prev.get() * opacity.get()).into(),
            None => opacity,
        })
    });
}

pub struct UsePaint {
    transform: Prop<Matrix>,
    opacity: Prop<f32>,
}
impl UsePaint {
    pub fn with<R>(&self, f: impl FnOnce(&mut Paint) -> R) -> R {
        PAINT.with_borrow_mut(|paint| {
            paint.transform = self.transform.get();
            paint.alpha = self.opacity.get();
            f(paint)
        })
    }

    fn build_shape<S>(
        &self,
        shader: Prop<S>,
        mut f: impl FnMut(&mut Paint, f32, &mut BuffersBuilder<Vertex, u16, ShaderConstructor<S::Target>>)
            + 'static,
    ) -> ReadSignal<Rc<Shape>>
    where
        S: Clone + IntoShader + 'static,
    {
        let transform = self.transform.clone();
        let alpha = self.opacity.clone();
        computed_always(move |_| {
            let mut shape = Shape::default();
            shape.build_from(
                transform.get(),
                alpha.get(),
                shader.get_clone().into_shader(),
                |tol, shaded| PAINT.with_borrow_mut(|p| f(p, tol, shaded)),
            );
            Rc::new(shape)
        })
    }

    pub fn build_fill_path<I, S>(
        &self,
        path: impl IntoProp<I>,
        shader: impl IntoProp<S>,
    ) -> ReadSignal<Rc<Shape>>
    where
        I: Clone + IntoIterator<Item = PathEvent> + 'static,
        S: Clone + Shader + 'static,
    {
        let path = path.into_prop();
        self.build_shape(shader.into_prop(), move |p, tol, shaded| {
            p.fill_tess
                .tessellate(path.get_clone(), &FillOptions::tolerance(tol), shaded)
                .unwrap();
        })
    }

    pub fn build_fill_rect<S>(
        &self,
        rect: impl IntoProp<Rect>,
        shader: impl IntoProp<S>,
    ) -> ReadSignal<Rc<Shape>>
    where
        S: Clone + Shader + 'static,
    {
        let rect = rect.into_prop();
        self.build_shape(shader.into_prop(), move |p, tol, shaded| {
            p.fill_tess
                .tessellate_rectangle(
                    &lyon::geom::Box2D::from_origin_and_size(
                        lyon::geom::point(rect.get().x, rect.get().y),
                        lyon::geom::size(rect.get().w, rect.get().h),
                    ),
                    &FillOptions::tolerance(tol),
                    shaded,
                )
                .unwrap();
        })
    }

    pub fn build_fill_circle<S>(
        &self,
        center: impl IntoProp<Point>,
        radius: impl IntoProp<f32>,
        shader: Prop<S>,
    ) -> ReadSignal<Rc<Shape>>
    where
        S: Clone + Shader + 'static,
    {
        let center = center.into_prop();
        let radius = radius.into_prop();
        self.build_shape(shader, move |p, tol, shaded| {
            let center = center.get();
            p.fill_tess
                .tessellate_circle(
                    lyon::geom::point(center.x, center.y),
                    radius.get(),
                    &FillOptions::tolerance(tol),
                    shaded,
                )
                .unwrap();
        })
    }
}

pub fn use_paint(style: &mut Style) -> UsePaint {
    fn extract<T: Clone + PartialEq + 'static>(
        prev: Option<Prop<T>>,
        style: Option<Prop<T>>,
        default: impl Fn() -> T,
        mut reduce: impl FnMut(T, T) -> T + 'static,
    ) -> Prop<T> {
        match (prev, style) {
            (Some(prev), Some(style)) => {
                computed(move |_| reduce(prev.get_clone(), style.get_clone())).into()
            }
            (Some(prev), None) => prev,
            (None, Some(style)) => style,
            (None, None) => default().into(),
        }
    }

    UsePaint {
        transform: extract(
            inject::<Transform>().map(|it| it.0),
            style.transform,
            Matrix::identity,
            |a, b| a * b,
        ),
        opacity: extract(
            inject::<Opacity>().map(|it| it.0),
            style.opacity,
            || 1.0,
            |a, b| a * b,
        ),
    }
}

#[derive(Default)]
pub struct Shape {
    buffers: VertexBuffers<Vertex, u16>,
    texture: Option<SharedTexture>,
}

impl Shape {
    fn build_from<S: Shader>(
        &mut self,
        transform: Matrix,
        alpha: f32,
        shader: S,
        f: impl FnOnce(f32, &mut BuffersBuilder<Vertex, u16, ShaderConstructor<S>>),
    ) {
        self.buffers.clear();
        self.texture = shader.texture();

        let scale = transform.svd_unordered(false, false).singular_values.max();
        let tolerance = 0.2 / scale;

        let shaded = ShaderConstructor {
            transform,
            shader,
            alpha,
        };
        f(
            tolerance,
            &mut BuffersBuilder::new(&mut self.buffers, shaded),
        );
    }

    pub fn draw(&self) {
        let gl = unsafe { get_internal_gl() }.quad_gl;
        gl.texture(self.texture.as_deref().copied());
        gl.draw_mode(DrawMode::Triangles);
        gl.geometry(&self.buffers.vertices, &self.buffers.indices);
    }
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
    shape: Shape,

    transform: Matrix,
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
            shape: Shape::default(),

            transform: Matrix::identity(),
            alpha: 1.,
        }
    }

    pub fn fill_path(
        &mut self,
        path: impl IntoIterator<Item = PathEvent>,
        shader: impl IntoShader,
    ) {
        self.shape.build_from(
            self.transform,
            self.alpha,
            shader.into_shader(),
            |tol, shaded| {
                self.fill_tess
                    .tessellate(path, &FillOptions::tolerance(tol), shaded)
                    .unwrap();
            },
        );
        self.shape.draw();
    }

    pub fn fill_rect(&mut self, rect: Rect, shader: impl IntoShader) {
        self.shape.build_from(
            self.transform,
            self.alpha,
            shader.into_shader(),
            |tol, shaded| {
                self.fill_tess
                    .tessellate_rectangle(
                        &lyon::geom::Box2D::from_origin_and_size(
                            lyon::geom::point(rect.x, rect.y),
                            lyon::geom::size(rect.w, rect.h),
                        ),
                        &FillOptions::tolerance(tol),
                        shaded,
                    )
                    .unwrap();
            },
        );
        self.shape.draw();
    }

    pub fn fill_circle(&mut self, center: Point, radius: f32, shader: impl IntoShader) {
        self.shape.build_from(
            self.transform,
            self.alpha,
            shader.into_shader(),
            |tol, shaded| {
                self.fill_tess
                    .tessellate_circle(
                        lyon::geom::point(center.x, center.y),
                        radius,
                        &FillOptions::tolerance(tol),
                        shaded,
                    )
                    .unwrap();
            },
        );
        self.shape.draw();
    }
}
