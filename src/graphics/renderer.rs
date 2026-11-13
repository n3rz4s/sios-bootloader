use super::{Color, FrameBuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        let right = self.x as i64 + self.width as i64;
        let bottom = self.y as i64 + self.height as i64;

        x >= self.x && y >= self.y && (x as i64) < right && (y as i64) < bottom
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let left = (self.x as i64).max(other.x as i64);
        let top = (self.y as i64).max(other.y as i64);

        let right = (self.x as i64 + self.width as i64).min(other.x as i64 + other.width as i64);

        let bottom = (self.y as i64 + self.height as i64).min(other.y as i64 + other.height as i64);

        if right <= left || bottom <= top {
            return None;
        }

        Some(Self {
            x: left as i32,
            y: top as i32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        })
    }
}

pub struct Renderer<'fb, 'gop> {
    framebuffer: &'fb mut FrameBuffer<'gop>,

    screen: Rect,
    clip: Rect,
}

impl<'fb, 'gop> Renderer<'fb, 'gop> {
    pub fn new(framebuffer: &'fb mut FrameBuffer<'gop>) -> Self {
        let width = framebuffer.width().min(i32::MAX as usize) as u32;

        let height = framebuffer.height().min(i32::MAX as usize) as u32;

        let screen = Rect::new(0, 0, width, height);

        Self {
            framebuffer,
            screen,
            clip: screen,
        }
    }

    pub const fn width(&self) -> u32 {
        self.screen.width
    }

    pub const fn height(&self) -> u32 {
        self.screen.height
    }

    pub const fn dimensions(&self) -> (u32, u32) {
        (self.screen.width, self.screen.height)
    }

    pub fn set_clip(&mut self, rect: Rect) {
        self.clip = self
            .screen
            .intersection(rect)
            .unwrap_or(Rect::new(0, 0, 0, 0));
    }

    pub fn reset_clip(&mut self) {
        self.clip = self.screen;
    }

    pub fn clear(&mut self, color: Color) {
        self.framebuffer.clear(color);
    }

    #[inline]
    fn blend_pixel_unchecked(&mut self, x: i32, y: i32, color: Color) -> bool {
        if color.a == 0 {
            return false;
        }

        let output = if color.a == 255 {
            color
        } else {
            let Some(background) = self.framebuffer.get_pixel(x as usize, y as usize) else {
                return false;
            };

            color.blend_over(background)
        };

        self.framebuffer.put_pixel(x as usize, y as usize, output)
    }

    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) -> bool {
        if !self.clip.contains(x, y) || color.a == 0 {
            return false;
        }

        self.blend_pixel_unchecked(x, y, color)
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if color.a == 0 {
            return;
        }

        let Some(visible) = rect.intersection(self.clip) else {
            return;
        };

        if color.a == 255 {
            self.framebuffer.fill_rect(
                visible.x as usize,
                visible.y as usize,
                visible.width as usize,
                visible.height as usize,
                color,
            );

            return;
        }

        let right = visible.x as i64 + visible.width as i64;
        let bottom = visible.y as i64 + visible.height as i64;

        for y in visible.y as i64..bottom {
            for x in visible.x as i64..right {
                self.blend_pixel_unchecked(x as i32, y as i32, color);
            }
        }
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        if rect.is_empty() {
            return;
        }

        let Some(visible) = rect.intersection(self.clip) else {
            return;
        };

        let right = rect.x as i64 + rect.width as i64 - 1;
        let bottom = rect.y as i64 + rect.height as i64 - 1;

        let right = right.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        let bottom = bottom.clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        for x in visible.x..(visible.x + visible.width as i32) {
            self.draw_pixel(x, rect.y, color);

            if rect.height > 1 {
                self.draw_pixel(x, bottom, color);
            }
        }

        for y in visible.y..(visible.y + visible.height as i32) {
            self.draw_pixel(rect.x, y, color);

            if rect.width > 1 {
                self.draw_pixel(right, y, color);
            }
        }
    }

    pub fn draw_line(&mut self, start: Point, end: Point, color: Color) {
        let Some((mut x0, mut y0, x1, y1)) = self.clip_line(start, end) else {
            return;
        };

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };

        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut error = dx + dy;

        loop {
            self.draw_pixel(x0 as i32, y0 as i32, color);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let twice_error = error * 2;

            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }

            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn clip_line(&self, start: Point, end: Point) -> Option<(i64, i64, i64, i64)> {
        if self.clip.is_empty() {
            return None;
        }

        const LEFT: u8 = 1;
        const RIGHT: u8 = 2;
        const TOP: u8 = 4;
        const BOTTOM: u8 = 8;

        let min_x = self.clip.x as i64;
        let min_y = self.clip.y as i64;

        let max_x = min_x + self.clip.width as i64 - 1;
        let max_y = min_y + self.clip.height as i64 - 1;

        let mut x0 = start.x as i64;
        let mut y0 = start.y as i64;

        let mut x1 = end.x as i64;
        let mut y1 = end.y as i64;

        for _ in 0..8 {
            let mut code0 = 0;
            let mut code1 = 0;

            if x0 < min_x {
                code0 |= LEFT;
            } else if x0 > max_x {
                code0 |= RIGHT;
            }

            if y0 < min_y {
                code0 |= TOP;
            } else if y0 > max_y {
                code0 |= BOTTOM;
            }

            if x1 < min_x {
                code1 |= LEFT;
            } else if x1 > max_x {
                code1 |= RIGHT;
            }

            if y1 < min_y {
                code1 |= TOP;
            } else if y1 > max_y {
                code1 |= BOTTOM;
            }

            if (code0 | code1) == 0 {
                return Some((x0, y0, x1, y1));
            }

            if (code0 & code1) != 0 {
                return None;
            }

            let code = if code0 != 0 { code0 } else { code1 };

            let (new_x, new_y) = if code & TOP != 0 {
                let denominator = y1 - y0;

                if denominator == 0 {
                    return None;
                }

                let numerator = ((x1 - x0) as i128 * (min_y - y0) as i128) / denominator as i128;

                (i64::try_from(x0 as i128 + numerator).ok()?, min_y)
            } else if code & BOTTOM != 0 {
                let denominator = y1 - y0;

                if denominator == 0 {
                    return None;
                }

                let numerator = ((x1 - x0) as i128 * (max_y - y0) as i128) / denominator as i128;

                (i64::try_from(x0 as i128 + numerator).ok()?, max_y)
            } else if code & RIGHT != 0 {
                let denominator = x1 - x0;

                if denominator == 0 {
                    return None;
                }

                let numerator = ((y1 - y0) as i128 * (max_x - x0) as i128) / denominator as i128;

                (max_x, i64::try_from(y0 as i128 + numerator).ok()?)
            } else {
                let denominator = x1 - x0;

                if denominator == 0 {
                    return None;
                }

                let numerator = ((y1 - y0) as i128 * (min_x - x0) as i128) / denominator as i128;

                (min_x, i64::try_from(y0 as i128 + numerator).ok()?)
            };

            if code == code0 {
                x0 = new_x;
                y0 = new_y;
            } else {
                x1 = new_x;
                y1 = new_y;
            }
        }

        None
    }

    pub fn draw_circle(&mut self, center: Point, radius: u32, color: Color) {
        let max_radius = self.screen.width.max(self.screen.height);

        if radius > max_radius {
            return;
        }

        let mut x = radius as i64;
        let mut y = 0i64;

        let mut decision = 1 - x;

        while x >= y {
            self.plot_circle_points(center.x as i64, center.y as i64, x, y, color);

            y += 1;

            if decision < 0 {
                decision += 2 * y + 1;
            } else {
                x -= 1;
                decision += 2 * (y - x) + 1;
            }
        }
    }

    pub fn fill_circle(&mut self, center: Point, radius: u32, color: Color) {
        let max_radius = self.screen.width.max(self.screen.height);

        if radius > max_radius {
            return;
        }

        let radius = radius as i64;
        let radius_squared = (radius * radius) as u64;

        for dy in -radius..=radius {
            let remaining = radius_squared - (dy * dy) as u64;
            let dx = integer_sqrt(remaining) as i64;

            let y = center.y as i64 + dy;

            if y < i32::MIN as i64 || y > i32::MAX as i64 {
                continue;
            }

            let left = (center.x as i64 - dx).max(self.clip.x as i64);

            let clip_right = self.clip.x as i64 + self.clip.width as i64 - 1;

            let right = (center.x as i64 + dx).min(clip_right);

            if right < left {
                continue;
            }

            self.fill_rect(
                Rect::new(left as i32, y as i32, (right - left + 1) as u32, 1),
                color,
            );
        }
    }

    fn plot_circle_points(&mut self, center_x: i64, center_y: i64, x: i64, y: i64, color: Color) {
        let points = [
            (center_x + x, center_y + y),
            (center_x + y, center_y + x),
            (center_x - y, center_y + x),
            (center_x - x, center_y + y),
            (center_x - x, center_y - y),
            (center_x - y, center_y - x),
            (center_x + y, center_y - x),
            (center_x + x, center_y - y),
        ];

        for (x, y) in points {
            if let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) {
                self.draw_pixel(x, y, color);
            }
        }
    }

    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: u32, color: Color) {
        let Some(visible) = rect.intersection(self.clip) else {
            return;
        };

        let radius = radius.min(rect.width / 2).min(rect.height / 2);

        let bottom = visible.y as i64 + visible.height as i64;

        for y in visible.y as i64..bottom {
            if let Some((left, right)) = rounded_rect_span(rect, radius, y) {
                self.fill_horizontal_span(y as i32, left, right, color);
            }
        }
    }

    fn fill_horizontal_span(&mut self, y: i32, left: i64, right: i64, color: Color) {
        let clip_left = self.clip.x as i64;

        let clip_right = clip_left + self.clip.width as i64;

        let left = left.max(clip_left);
        let right = right.min(clip_right);

        if right <= left {
            return;
        }

        self.fill_rect(Rect::new(left as i32, y, (right - left) as u32, 1), color);
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: u32, color: Color) {
        let Some(visible) = rect.intersection(self.clip) else {
            return;
        };

        let inner = Rect::new(
            rect.x.saturating_add(1),
            rect.y.saturating_add(1),
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        );

        let inner_radius = radius.saturating_sub(1);

        let bottom = visible.y as i64 + visible.height as i64;

        for y in visible.y as i64..bottom {
            let Some((outer_left, outer_right)) = rounded_rect_span(rect, radius, y) else {
                continue;
            };

            if let Some((inner_left, inner_right)) = rounded_rect_span(inner, inner_radius, y) {
                let inner_left = inner_left.clamp(outer_left, outer_right);

                let inner_right = inner_right.clamp(outer_left, outer_right);

                self.fill_horizontal_span(y as i32, outer_left, inner_left, color);

                self.fill_horizontal_span(y as i32, inner_right, outer_right, color);
            } else {
                self.fill_horizontal_span(y as i32, outer_left, outer_right, color);
            }
        }
    }
}

fn rounded_rect_span(rect: Rect, radius: u32, y: i64) -> Option<(i64, i64)> {
    if rect.is_empty() {
        return None;
    }

    let top = rect.y as i64;
    let bottom = top + rect.height as i64;

    if y < top || y >= bottom {
        return None;
    }

    let radius = radius.min(rect.width / 2).min(rect.height / 2) as i64;

    let left = rect.x as i64;
    let width = rect.width as i64;
    let height = rect.height as i64;

    if radius == 0 {
        return Some((left, left + width));
    }

    let local_y = y - top;

    let dy = if local_y < radius {
        radius - local_y
    } else if local_y >= height - radius {
        local_y - (height - radius - 1)
    } else {
        0
    };

    let remaining = radius * radius - dy * dy;

    let dx = integer_sqrt(remaining as u64) as i64;

    Some((left + radius - dx, left + width - radius + dx))
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }

    let mut current = value;
    let mut next = (current + 1) / 2;

    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }

    current
}
