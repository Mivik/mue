use macroquad::prelude::*;
use mue_core::prelude::*;
use mue_macroquad::{
    node::*,
    style::{Styleable, StyleableExt},
    App,
};

fn main() {
    macroquad::Window::from_config(
        macroquad::window::Conf {
            window_title: "mue – events".to_string(),
            window_width: 800,
            window_height: 600,
            ..Default::default()
        },
        async { the_main().await },
    );
}

#[mue_macros::component]
fn view() -> impl Component {
    let pressed = signal(false);
    let count = signal(0);

    text(computed(move |_| {
        format!("Clicked {} times\nPressed: {}", count.get(), pressed.get()).into()
    }))
    .use_pressed(pressed)
    .on_click(move |_| count.update(|c| *c += 1))
}

async fn the_main() {
    let root = view();

    let mut app = App::new(root);

    loop {
        clear_background(BLACK);
        app.frame();
        next_frame().await;
    }
}
