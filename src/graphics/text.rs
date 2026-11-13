use super::{BitmapFont, Color, Point, Rect, Renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMetrics {
    pub width: u32,
    pub height: u32,
    pub lines: u32,
}

pub fn measure_text(font: &BitmapFont, text: &str, scale: u32) -> TextMetrics {
    let scale = scale.clamp(1, 16);

    let advance = font.advance().saturating_mul(scale);
    let line_height = font.line_height().saturating_mul(scale);

    let mut current_width = 0u32;
    let mut maximum_width = 0u32;
    let mut lines = 1u32;

    for character in text.chars() {
        match character {
            '\n' => {
                maximum_width = maximum_width.max(current_width);

                current_width = 0;
                lines = lines.saturating_add(1);
            }

            '\r' => {}

            '\t' => {
                current_width = current_width.saturating_add(advance.saturating_mul(4));
            }

            _ => {
                current_width = current_width.saturating_add(advance);
            }
        }
    }

    maximum_width = maximum_width.max(current_width);

    let height = font
        .height()
        .saturating_mul(scale)
        .saturating_add(line_height.saturating_mul(lines.saturating_sub(1)));

    TextMetrics {
        width: maximum_width,
        height,
        lines,
    }
}

pub fn draw_text(
    renderer: &mut Renderer,
    font: &BitmapFont,
    position: Point,
    text: &str,
    color: Color,
    scale: u32,
) -> TextMetrics {
    let scale = scale.clamp(1, 16);
    let scale_i32 = scale as i32;

    let mut cursor_x = position.x;
    let mut cursor_y = position.y;

    for character in text.chars() {
        match character {
            '\n' => {
                cursor_x = position.x;

                cursor_y =
                    cursor_y.saturating_add((font.line_height() as i32).saturating_mul(scale_i32));
            }

            '\r' => {}

            '\t' => {
                cursor_x = cursor_x.saturating_add(
                    (font.advance() as i32)
                        .saturating_mul(scale_i32)
                        .saturating_mul(4),
                );
            }

            character => {
                let glyph = font.glyph(character);

                for (row, bits) in glyph.iter().enumerate() {
                    for column in 0..font.width() {
                        let mask = 1u8 << (font.width() - 1 - column);

                        if *bits & mask == 0 {
                            continue;
                        }

                        let x = cursor_x.saturating_add((column as i32).saturating_mul(scale_i32));

                        let y = cursor_y.saturating_add((row as i32).saturating_mul(scale_i32));

                        renderer.fill_rect(Rect::new(x, y, scale, scale), color);
                    }
                }

                cursor_x =
                    cursor_x.saturating_add((font.advance() as i32).saturating_mul(scale_i32));
            }
        }
    }

    measure_text(font, text, scale)
}
