use super::Color;

use uefi::{
    Result, Status,
    proto::console::gop::{FrameBuffer as UefiFrameBuffer, GraphicsOutput, PixelFormat},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PixelOrder {
    Rgb,
    Bgr,
}

pub struct FrameBuffer<'gop> {
    buffer: UefiFrameBuffer<'gop>,

    width: usize,
    height: usize,
    stride: usize,

    pixel_order: PixelOrder,
}

impl<'gop> FrameBuffer<'gop> {
    pub fn new(gop: &'gop mut GraphicsOutput) -> Result<Self> {
        let mode_info = gop.current_mode_info();

        let (width, height) = mode_info.resolution();
        let stride = mode_info.stride();

        if width == 0 || height == 0 || stride < width {
            return Err(Status::BAD_BUFFER_SIZE.into());
        }

        let pixel_order = match mode_info.pixel_format() {
            PixelFormat::Rgb => PixelOrder::Rgb,
            PixelFormat::Bgr => PixelOrder::Bgr,

            PixelFormat::Bitmask | PixelFormat::BltOnly => {
                return Err(Status::UNSUPPORTED.into());
            }
        };

        let buffer = gop.frame_buffer();

        let required_size = match stride
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
        {
            Some(size) => size,
            None => return Err(Status::BAD_BUFFER_SIZE.into()),
        };

        if buffer.size() < required_size {
            return Err(Status::BAD_BUFFER_SIZE.into());
        }

        Ok(Self {
            buffer,

            width,
            height,
            stride,

            pixel_order,
        })
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn stride(&self) -> usize {
        self.stride
    }

    fn pixel_offset(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let pixel_index = y.checked_mul(self.stride)?.checked_add(x)?;

        let offset = pixel_index.checked_mul(4)?;
        let end = offset.checked_add(4)?;

        if end > self.buffer.size() {
            return None;
        }

        Some(offset)
    }

    #[inline(always)]
    fn write_pixel_at(&mut self, offset: usize, color: Color) {
        unsafe {
            match self.pixel_order {
                PixelOrder::Rgb => {
                    self.buffer.write_byte(offset, color.r);
                    self.buffer.write_byte(offset + 1, color.g);
                    self.buffer.write_byte(offset + 2, color.b);
                }

                PixelOrder::Bgr => {
                    self.buffer.write_byte(offset, color.b);
                    self.buffer.write_byte(offset + 1, color.g);
                    self.buffer.write_byte(offset + 2, color.r);
                }
            }

            self.buffer.write_byte(offset + 3, 0);
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: Color) -> bool {
        let Some(offset) = self.pixel_offset(x, y) else {
            return false;
        };

        self.write_pixel_at(offset, color);

        true
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<Color> {
        let offset = self.pixel_offset(x, y)?;

        let (r, g, b) = match self.pixel_order {
            PixelOrder::Rgb => (
                unsafe { self.buffer.read_byte(offset) },
                unsafe { self.buffer.read_byte(offset + 1) },
                unsafe { self.buffer.read_byte(offset + 2) },
            ),

            PixelOrder::Bgr => (
                unsafe { self.buffer.read_byte(offset + 2) },
                unsafe { self.buffer.read_byte(offset + 1) },
                unsafe { self.buffer.read_byte(offset) },
            ),
        };

        Some(Color::rgb(r, g, b))
    }

    pub fn clear(&mut self, color: Color) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        let start_x = x.min(self.width);
        let start_y = y.min(self.height);

        let end_x = x.saturating_add(width).min(self.width);
        let end_y = y.saturating_add(height).min(self.height);

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        for py in start_y..end_y {
            let row_start = (py * self.stride + start_x) * 4;

            let mut offset = row_start;

            for _ in start_x..end_x {
                self.write_pixel_at(offset, color);
                offset += 4;
            }
        }
    }
}
