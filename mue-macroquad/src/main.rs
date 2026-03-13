use macroquad::prelude::*;
use mue_core::prelude::*;
use mue_macroquad::{node::*, styled, App, Style};
use taffy::Dimension;

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

async fn the_main() {
    let time = signal(0.0);

    let mut sprites = vec![];
    for i in 0..4 {
        sprites.push(styled(
            Style::new().width(Dimension::percent(1.)).height(
                time.map(move |t| Dimension::percent((t * (i + 1) as f32).sin() * 0.5 + 0.5)),
            ),
            sprite,
        ));
    }
    let root = styled(
        Style::new()
            .width(Dimension::percent(1.))
            .height(Dimension::percent(1.)),
        || flexbox(sprites),
    );

    let app = App::new(root);

    loop {
        clear_background(BLACK);

        time.set(get_time() as f32);

        app.frame();

        next_frame().await
    }
}
