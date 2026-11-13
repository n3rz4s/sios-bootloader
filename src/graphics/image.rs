use super::{Color, Rect, Renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    ZeroDimensions,
    SizeOverflow,
    BufferTooSmall,
}

#[derive(Debug, Clone, Copy)]
pub struct Image<'pixels> {
    width: u32,
    height: u32,

    pixels: &'pixels [Color],
}

impl<'pixels> Image<'pixels> {
    pub fn new(width: u32, height: u32, pixels: &'pixels [Color]) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::ZeroDimensions);
        }

        let required_pixels = (width as usize)
            .checked_mul(height as usize)
            .ok_or(ImageError::SizeOverflow)?;

        if pixels.len() < required_pixels {
            return Err(ImageError::BufferTooSmall);
        }

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = (y as usize)
            .checked_mul(self.width as usize)?
            .checked_add(x as usize)?;

        self.pixels.get(index).copied()
    }

    pub fn draw(&self, renderer: &mut Renderer, x: i32, y: i32) {
        self.draw_scaled(renderer, Rect::new(x, y, self.width, self.height));
    }

    pub fn draw_scaled(&self, renderer: &mut Renderer, destination: Rect) {
        if destination.is_empty() {
            return;
        }

        for dy in 0..destination.height {
            let source_y = (u64::from(dy) * u64::from(self.height)) / u64::from(destination.height);

            let destination_y = destination.y as i64 + i64::from(dy);

            let Ok(destination_y) = i32::try_from(destination_y) else {
                continue;
            };

            for dx in 0..destination.width {
                let source_x =
                    (u64::from(dx) * u64::from(self.width)) / u64::from(destination.width);

                let destination_x = destination.x as i64 + i64::from(dx);

                let Ok(destination_x) = i32::try_from(destination_x) else {
                    continue;
                };

                let Some(color) = self.get_pixel(source_x as u32, source_y as u32) else {
                    continue;
                };

                renderer.draw_pixel(destination_x, destination_y, color);
            }
        }
    }
}
