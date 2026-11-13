use crate::graphics::Color;

use crate::ui::{
    element::{Element, ElementError, ElementKind},
    style::{Dimension, Style},
};

pub struct Button;

impl Button {
    pub fn new(text: &str) -> Result<Element, ElementError> {
        let style = Style::new()
            .with_width(Dimension::Fill)
            .with_height(Dimension::Pixels(48))
            .with_background(Color::rgb(55, 36, 78))
            .with_foreground(Color::WHITE)
            .with_padding(12)
            .with_border_radius(8)
            .with_font_scale(1);

        let mut element = Element::with_text(ElementKind::Button, text)?;

        element.set_style(style);

        Ok(element)
    }
}
