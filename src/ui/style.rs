use crate::graphics::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Fill,
    Pixels(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Overlay,
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    width: Dimension,
    height: Dimension,

    direction: LayoutDirection,

    padding: u32,
    gap: u32,

    background: Color,
    foreground: Color,

    opacity: u8,

    border_color: Color,
    border_width: u32,
    border_radius: u32,

    font_scale: u32,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            width: Dimension::Fill,
            height: Dimension::Fill,

            direction: LayoutDirection::Overlay,

            padding: 0,
            gap: 0,

            background: Color::TRANSPARENT,
            foreground: Color::WHITE,

            opacity: 255,

            border_color: Color::TRANSPARENT,
            border_width: 0,
            border_radius: 0,

            font_scale: 1,
        }
    }

    pub const fn width(&self) -> Dimension {
        self.width
    }

    pub const fn height(&self) -> Dimension {
        self.height
    }

    pub fn with_width(mut self, width: Dimension) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: Dimension) -> Self {
        self.height = height;
        self
    }

    pub const fn direction(&self) -> LayoutDirection {
        self.direction
    }

    pub const fn padding(&self) -> u32 {
        self.padding
    }

    pub const fn gap(&self) -> u32 {
        self.gap
    }

    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn background(&self) -> Color {
        self.background
    }

    pub const fn foreground(&self) -> Color {
        self.foreground
    }

    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    pub fn with_foreground(mut self, foreground: Color) -> Self {
        self.foreground = foreground;
        self
    }

    pub const fn opacity(&self) -> u8 {
        self.opacity
    }

    pub fn with_opacity(mut self, opacity: u8) -> Self {
        self.opacity = opacity;
        self
    }

    pub const fn border_color(&self) -> Color {
        self.border_color
    }

    pub const fn border_width(&self) -> u32 {
        self.border_width
    }

    pub const fn border_radius(&self) -> u32 {
        self.border_radius
    }

    pub const fn font_scale(&self) -> u32 {
        self.font_scale
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn with_border_width(mut self, width: u32) -> Self {
        self.border_width = width;
        self
    }

    pub fn with_border_radius(mut self, radius: u32) -> Self {
        self.border_radius = radius;
        self
    }

    pub fn with_border(mut self, width: u32, radius: u32, color: Color) -> Self {
        self.border_width = width;
        self.border_radius = radius;
        self.border_color = color;
        self
    }

    pub fn with_font_scale(mut self, scale: u32) -> Self {
        self.font_scale = scale;
        self
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}
