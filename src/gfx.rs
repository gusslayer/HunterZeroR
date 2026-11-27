//! A tiny software canvas for the 320x240 virtual screen.
//!
//! Rendering into a plain pixel buffer keeps the application independent from
//! OpenGL/GLES versions: the only thing SDL2 has to do is present one texture.
//! That matters on the Pi Zero W, whose VideoCore IV GPU only offers GLES 2.0.

use crate::core::font;
use crate::theme::Color;

/// A 32-bit ARGB pixel buffer with a few drawing primitives.
pub struct Canvas {
    width: i32,
    height: i32,
    pixels: Vec<u32>,
}

impl Canvas {
    /// Create a black canvas.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width as i32,
            height: height as i32,
            pixels: vec![0xFF00_0000; (width * height) as usize],
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// Raw ARGB8888 pixels, ready for `Texture::update`.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Pixels as bytes (little-endian ARGB8888, which SDL calls ARGB8888).
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: `u32` has no padding and the slice length is exact.  SDL's
        // ARGB8888 format uses the same byte order on the little-endian hosts
        // supported by this project (armhf and x86_64).
        unsafe {
            std::slice::from_raw_parts(
                self.pixels.as_ptr() as *const u8,
                self.pixels.len() * std::mem::size_of::<u32>(),
            )
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) {
        if width <= 0 || height <= 0 {
            return;
        }
        let left = x.max(0);
        let top = y.max(0);
        let right = (x + width).min(self.width);
        let bottom = (y + height).min(self.height);
        if left >= right || top >= bottom {
            return;
        }
        for row in top..bottom {
            let start = (row * self.width + left) as usize;
            let end = (row * self.width + right) as usize;
            self.pixels[start..end].fill(color);
        }
    }

    pub fn stroke_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) {
        if width <= 0 || height <= 0 {
            return;
        }
        self.hline(x, x + width - 1, y, color);
        self.hline(x, x + width - 1, y + height - 1, color);
        self.vline(x, y, y + height - 1, color);
        self.vline(x + width - 1, y, y + height - 1, color);
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, color: Color) {
        if y < 0 || y >= self.height {
            return;
        }
        let start = x0.min(x1).max(0);
        let end = x0.max(x1).min(self.width - 1);
        if start > end {
            return;
        }
        let row = (y * self.width) as usize;
        self.pixels[row + start as usize..=row + end as usize].fill(color);
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, color: Color) {
        if x < 0 || x >= self.width {
            return;
        }
        let start = y0.min(y1).max(0);
        let end = y0.max(y1).min(self.height - 1);
        for y in start..=end {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Draw a one-pixel line with Bresenham's algorithm.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.set_pixel(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x += sx;
            }
            if doubled <= dx {
                error += dx;
                y += sy;
            }
        }
    }

    /// Fill a triangle using a simple scanline test.
    pub fn fill_triangle(&mut self, p0: (i32, i32), p1: (i32, i32), p2: (i32, i32), color: Color) {
        let min_x = p0.0.min(p1.0).min(p2.0).max(0);
        let max_x = p0.0.max(p1.0).max(p2.0).min(self.width - 1);
        let min_y = p0.1.min(p1.1).min(p2.1).max(0);
        let max_y = p0.1.max(p1.1).max(p2.1).min(self.height - 1);
        let area = edge(p0, p1, p2);
        if area == 0 {
            return;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let point = (x, y);
                let w0 = edge(p1, p2, point);
                let w1 = edge(p2, p0, point);
                let w2 = edge(p0, p1, point);
                let inside = if area > 0 {
                    w0 >= 0 && w1 >= 0 && w2 >= 0
                } else {
                    w0 <= 0 && w1 <= 0 && w2 <= 0
                };
                if inside {
                    self.pixels[(y * self.width + x) as usize] = color;
                }
            }
        }
    }

    /// Draw text using the 8x8 bitmap font.  Returns the advance in pixels.
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, color: Color, scale: i32) -> i32 {
        let scale = scale.max(1);
        let flattened = font::plain_text(text);
        let mut cursor = x;
        // Fast path for scale == 1: write directly into the pixel buffer.
        if scale == 1 {
            if x >= 0 && y >= 0 && x < self.width && y < self.height {
                for character in flattened.chars() {
                    let upper = character.to_ascii_uppercase();
                    let rows = font::glyph(upper);
                    for (row_index, row) in rows.iter().enumerate() {
                        let ty = y + row_index as i32;
                        if ty < 0 || ty >= self.height {
                            continue;
                        }
                        let base = ty * self.width as i32;
                        for (column_index, bit) in row.chars().enumerate() {
                            if bit == '1' {
                                let tx = cursor + column_index as i32;
                                if tx >= 0 && tx < self.width {
                                    self.pixels[(base + tx) as usize] = color;
                                }
                            }
                        }
                    }
                    cursor += 8;
                }
            } else {
                // Fall back to the scale > 1 path if outside canvas bounds.
                for character in flattened.chars() {
                    let upper = character.to_ascii_uppercase();
                    let rows = font::glyph(upper);
                    for (row_index, row) in rows.iter().enumerate() {
                        for (column_index, bit) in row.chars().enumerate() {
                            if bit == '1' {
                                self.fill_rect(
                                    cursor + column_index as i32 * scale,
                                    y + row_index as i32 * scale,
                                    scale,
                                    scale,
                                    color,
                                );
                            }
                        }
                    }
                    cursor += 8 * scale;
                }
            }
            return cursor - x;
        }
        // Original logic for scale > 1.
        for character in flattened.chars() {
            let upper = character.to_ascii_uppercase();
            let rows = font::glyph(upper);
            for (row_index, row) in rows.iter().enumerate() {
                for (column_index, bit) in row.chars().enumerate() {
                    if bit == '1' {
                        self.fill_rect(
                            cursor + column_index as i32 * scale,
                            y + row_index as i32 * scale,
                            scale,
                            scale,
                            color,
                        );
                    }
                }
            }
            cursor += 8 * scale;
        }
        cursor - x
    }

    /// Draw text centred inside a horizontal span.
    pub fn draw_text_centered(
        &mut self,
        text: &str,
        center_x: i32,
        y: i32,
        color: Color,
        scale: i32,
    ) -> i32 {
        let width = font::text_width(text, scale);
        self.draw_text(text, center_x - width / 2, y, color, scale)
    }

    /// Draw text clipped to `max_width` pixels, adding `..` when truncated.
    pub fn draw_text_clipped(
        &mut self,
        text: &str,
        x: i32,
        y: i32,
        max_width: i32,
        color: Color,
        scale: i32,
    ) -> i32 {
        let scale = scale.max(1);
        let flattened = font::plain_text(text);
        let max_chars = (max_width / (8 * scale)).max(0) as usize;
        if flattened.chars().count() <= max_chars {
            return self.draw_text(&flattened, x, y, color, scale);
        }
        if max_chars <= 2 {
            let clipped: String = flattened.chars().take(max_chars).collect();
            return self.draw_text(&clipped, x, y, color, scale);
        }
        let clipped: String = flattened.chars().take(max_chars - 2).collect();
        self.draw_text(&format!("{clipped}.."), x, y, color, scale)
    }

    /// Wrap text into lines of at most `max_chars` characters.
    pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
        wrap_text(text, max_chars)
    }
}

fn edge(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> i32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Wrap plain text into lines no longer than `max_chars` characters.
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in font::plain_text(text).split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if candidate.chars().count() <= max_chars {
            current = candidate;
        } else {
            if !current.is_empty() {
                lines.push(current);
            }
            if word.chars().count() > max_chars {
                let mut chunk = String::new();
                for character in word.chars() {
                    chunk.push(character);
                    if chunk.chars().count() >= max_chars {
                        lines.push(std::mem::take(&mut chunk));
                    }
                }
                current = chunk;
            } else {
                current = word.to_string();
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::TEXT_COLOR;

    #[test]
    fn fill_rect_clips_to_canvas() {
        let mut canvas = Canvas::new(10, 10);
        canvas.fill_rect(-5, -5, 20, 20, TEXT_COLOR);
        assert_eq!(canvas.pixels().len(), 100);
        assert_eq!(canvas.pixels()[0], TEXT_COLOR);
    }

    #[test]
    fn wrapping_splits_long_words() {
        let lines = wrap_text("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }
}
