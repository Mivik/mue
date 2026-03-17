use taffy::Display;

use crate::{
    layout::{use_layout, Style},
    Styleable,
};

#[mue_macros::node]
pub fn flexbox(mut style: Style) {
    use_layout(&mut style);
}

pub fn div() -> FlexboxBuilder {
    flexbox().display(Display::Block)
}
