use crate::graphics::Color;

use crate::ui::{
    element::{Element, ElementKind},
    style::{Dimension, LayoutDirection, Style},
};

pub struct Panel;

impl Panel {
    pub fn new() -> Element {
        let style = Style::new()
            .with_width(Dimension::Fill)
            .with_height(Dimension::Fill)
            .with_direction(LayoutDirection::Vertical)
            .with_padding(10)
            .with_gap(10)
            .with_background(Color::rgb(31, 24, 44))
            .with_foreground(Color::WHITE)
            .with_border_radius(12);

        let mut element = Element::new(ElementKind::Panel);

        element.set_style(style);

        element
    }
}
