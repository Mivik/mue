use taffy::{Dimension, FlexDirection};

use super::Styleable;

pub trait StyleableExt: Styleable {
    fn w_full(self) -> Self {
        self.width(Dimension::percent(1.))
    }
    fn w_auto(self) -> Self {
        self.width(Dimension::auto())
    }

    fn h_full(self) -> Self {
        self.height(Dimension::percent(1.))
    }
    fn h_auto(self) -> Self {
        self.height(Dimension::auto())
    }

    fn size_full(self) -> Self {
        self.width(Dimension::percent(1.))
            .height(Dimension::percent(1.))
    }

    fn flex_row(self) -> Self {
        self.flex_direction(FlexDirection::Row)
    }
    fn flex_column(self) -> Self {
        self.flex_direction(FlexDirection::Column)
    }
}

impl<T: Styleable> StyleableExt for T {}
