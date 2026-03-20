use taffy::Display;

use crate::{
    layout::use_layout,
    style::{Style, Styleable},
};

#[mue_macros::node]
pub fn flexbox(style: &mut Style) {
    use_layout(style);
}

pub fn div() -> FlexboxBuilder {
    flexbox().display(Display::Block)
}
