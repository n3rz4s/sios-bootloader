use crate::graphics::Color;

use crate::ui::{
    element::{Element, ElementError, ElementKind},
    style::{Dimension, Style},
};

pub struct Label;

impl Label {
    pub fn new(text: &str) -> Result<Element, ElementError> {
        let style = Style::new()
            .with_width(Dimension::Fill)
            .with_height(Dimension::Pixels(24))
            .with_foreground(Color::WHITE)
            .with_font_scale(1);

        let mut element = Element::with_text(ElementKind::Label, text)?;

        element.set_style(style);

        Ok(element)
    }
}
