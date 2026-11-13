#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }

    pub fn blend_over(self, background: Self) -> Self {
        let alpha = u32::from(self.a);
        let inverse_alpha = 255 - alpha;

        let red = (u32::from(self.r) * alpha + u32::from(background.r) * inverse_alpha + 127) / 255;

        let green =
            (u32::from(self.g) * alpha + u32::from(background.g) * inverse_alpha + 127) / 255;

        let blue =
            (u32::from(self.b) * alpha + u32::from(background.b) * inverse_alpha + 127) / 255;

        Self::rgb(red as u8, green as u8, blue as u8)
    }
}
