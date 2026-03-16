use macroquad::prelude::*;
use mue_core::prelude::*;
use mue_macroquad::{node::*, App, Style};
use taffy::{AlignItems, Dimension, FlexDirection};

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

    // let mut sprites = vec![];
    // for i in 0..4 {
    //     sprites.push(
    //         sprite().styled(
    //             Style::new()
    //                 .width(Dimension::auto())
    //                 .height(
    //                     time.map(move |t| {
    //                         Dimension::percent((t * (i + 1) as f32).sin() * 0.5 + 0.5)
    //                     }),
    //                 )
    //                 .flex_grow(1.),
    //         ),
    //     );
    // }
    let sprites =
        KeyedChildren::new(
            computed(move || {
                let time = time.get();
                let count = ((time * 2.) as usize + 1).min(10);
                (0..count).collect()
            }),
            |&value| value,
            move |&i| {
                sprite().styled(
                    Style::new()
                        .width(Dimension::auto())
                        .height(time.map(move |t| {
                            Dimension::percent((t * (i + 1) as f32).sin() * 0.5 + 0.5)
                        }))
                        .flex_grow(1.),
                )
            },
        );

    let root = flexbox(sprites).styled(
        Style::new()
            .flex_direction(FlexDirection::Row)
            .width(Dimension::percent(1.))
            .height(Dimension::percent(1.))
            .justify_items(AlignItems::Stretch),
    );

    let app = App::new(root);

    loop {
        clear_background(BLACK);

        time.set(get_time() as f32);

        app.frame();

        next_frame().await;
    }
}
