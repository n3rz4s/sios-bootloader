use super::style::Style;

pub const ELEMENT_TEXT_CAPACITY: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Root,
    Panel,
    Label,
    Button,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementError {
    TextTooLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementId {
    index: u32,
    generation: u32,
}

impl ElementId {
    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    kind: ElementKind,

    visible: bool,
    enabled: bool,

    style: Style,

    text: [u8; ELEMENT_TEXT_CAPACITY],
    text_len: u8,
}

impl Element {
    pub const fn new(kind: ElementKind) -> Self {
        Self {
            kind,

            visible: true,
            enabled: true,

            style: Style::new(),

            text: [0; ELEMENT_TEXT_CAPACITY],
            text_len: 0,
        }
    }

    pub const fn root() -> Self {
        Self::new(ElementKind::Root)
    }

    pub fn with_text(kind: ElementKind, text: &str) -> Result<Self, ElementError> {
        let mut element = Self::new(kind);

        element.set_text(text)?;

        Ok(element)
    }

    pub const fn kind(&self) -> ElementKind {
        self.kind
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub const fn style(&self) -> &Style {
        &self.style
    }

    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_text(&mut self, text: &str) -> Result<(), ElementError> {
        let bytes = text.as_bytes();

        if bytes.len() > ELEMENT_TEXT_CAPACITY {
            return Err(ElementError::TextTooLong);
        }

        self.text.fill(0);

        self.text[..bytes.len()].copy_from_slice(bytes);

        self.text_len = bytes.len() as u8;

        Ok(())
    }

    pub fn text(&self) -> &str {
        let bytes = &self.text[..self.text_len as usize];

        core::str::from_utf8(bytes).unwrap_or("")
    }

    pub const fn has_text(&self) -> bool {
        self.text_len > 0
    }
}
