use crate::graphics::Color;

use crate::ui::{
    element::{Element, ElementKind},
    style::{Dimension, LayoutDirection, Style},
};

pub struct List;

impl List {
    pub fn new() -> Element {
        let style = Style::new()
            .with_width(Dimension::Fill)
            .with_height(Dimension::Fill)
            .with_direction(LayoutDirection::Vertical)
            .with_padding(8)
            .with_gap(6)
            .with_background(Color::TRANSPARENT)
            .with_foreground(Color::WHITE);

        let mut element = Element::new(ElementKind::List);

        element.set_style(style);

        element
    }
}
