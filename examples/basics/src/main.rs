use macroquad::prelude::*;
use mue_core::prelude::*;
use mue_macroquad::{
    node::*,
    shader::SharedTexture,
    style::{Styleable, StyleableExt},
    App, Matrix, Vector,
};
use taffy::{AlignItems, Dimension};

fn main() {
    macroquad::Window::from_config(
        macroquad::window::Conf {
            window_title: "mue".to_string(),
            window_width: 973,
            window_height: 608,
            ..Default::default()
        },
        async {
            the_main().await;
        },
    );
}

#[mue_macros::component]
fn view(texture: SharedTexture, time: f32) -> impl IntoNode {
    let sprites = map_keyed(
        Prop::Dynamic(computed(move |_| {
            let time = time.get();
            let count = ((time * 2.) as usize + 1).min(10);
            (0..count).collect()
        })),
        |&value| value,
        move |&i| {
            image(texture.clone())
                .w_auto()
                .height(time.map(move |t: f32| {
                    Dimension::percent((t.min(4.) * (i + 1) as f32).sin() * 0.5 + 0.5)
                }))
                .opacity(time.map(move |t| (t * (i + 1) as f32).sin().abs()))
                .flex_grow(1.)
        },
    );

    let row = flexbox()
        .children(sprites)
        .flex_row()
        .w_full()
        .h_auto()
        .flex_grow(1.)
        .justify_items(AlignItems::Stretch);

    flexbox()
        .children((
            row,
            circle()
                .h_auto()
                .flex_grow(1.)
                .transform(time.map(|t| {
                    Matrix::new_translation(&Vector::new(
                        (t * 20.).cos() * 20.,
                        (t * 20.).sin() * 20.,
                    ))
                }))
                .opacity(time.map(|t| t.sin() * 0.5 + 0.5))
                .show_if(time.map(|t| t >= 0.2)),
        ))
        .flex_column()
        .size_full()
}

async fn the_main() {
    let time = signal(0.);
    set_pc_assets_folder("assets");
    let texture: SharedTexture = load_texture("test.png").await.unwrap().into();

    let root = view(texture, *time);
    let app = App::new(root);

    loop {
        clear_background(BLACK);

        time.set(get_time() as f32);

        app.frame();

        next_frame().await;
    }
}
