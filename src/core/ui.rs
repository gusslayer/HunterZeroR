//! Shared pixel-art widgets, scrolling state, help view, and D-pad editors.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::core::font;
use crate::core::input::Action;
use crate::core::system_stats::SystemStats;
use crate::core::validation::{normalize_hostname, normalize_mac};
use crate::gfx::{wrap_text, Canvas};
use crate::theme::*;

/// Result of an editor receiving one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditResult {
    Saved,
    Cancelled,
}

// ---------------------------------------------------------------------------
// Basic widgets
// ---------------------------------------------------------------------------

/// Draw a crisp one-pixel frame with small corner ticks.
pub fn draw_frame(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Color,
    fill: Option<Color>,
) {
    if let Some(fill) = fill {
        canvas.fill_rect(x, y, width, height, fill);
    }
    canvas.stroke_rect(x, y, width, height, color);
    canvas.line(x, y + 4, x + 4, y, color);
    canvas.line(
        x + width - 5,
        y + height - 1,
        x + width - 1,
        y + height - 5,
        color,
    );
}

pub fn draw_chevron(canvas: &mut Canvas, x: i32, y: i32, width: i32, height: i32, color: Color) {
    let middle = y + height / 2;
    canvas.line(x, y, x + width - 2, middle, color);
    canvas.line(x + width - 2, middle, x, y + height - 1, color);
}

/// Render one selectable row with a consistent pixel-art style.
pub fn draw_row(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    label: &str,
    value: &str,
    selected: bool,
    icon: Option<&str>,
) {
    if selected {
        canvas.fill_rect(x, y, width, height, SELECTED_BG);
        canvas.stroke_rect(x, y, width, height, ACCENT_COLOR);
        canvas.fill_rect(x, y, 3, height, ACCENT_COLOR);
    }
    let text_y = y + (height - 8) / 2;
    let mut text_x = x + 7;
    if let Some(icon) = icon {
        draw_icon(
            canvas,
            icon,
            x + 5,
            y + (height - 14) / 2,
            14,
            if selected { ACCENT_COLOR } else { TEXT_COLOR },
        );
        text_x = x + 24;
    }
    let label = font::plain_text(label).to_ascii_uppercase();
    let value_text = if value.is_empty() {
        String::new()
    } else {
        let value = font::plain_text(value).to_ascii_uppercase();
        if value.chars().count() > 12 {
            value.chars().take(12).collect::<String>()
        } else {
            value
        }
    };
    let value_width = if value_text.is_empty() {
        0
    } else {
        font::text_width(&value_text, 1)
    };
    let right_margin = if selected { 20 } else { 10 };
    let label_max_w = (width - (text_x - x) - right_margin - value_width - 6).max(0);
    canvas.draw_text_clipped(&label, text_x, text_y, label_max_w, TEXT_COLOR, 1);
    if !value_text.is_empty() {
        canvas.draw_text(
            &value_text,
            x + width - right_margin - value_width,
            text_y,
            ACCENT_COLOR,
            1,
        );
    }
    if selected {
        draw_chevron(canvas, x + width - 13, y + 3, 9, height - 6, ACCENT_COLOR);
    }
}

/// Draw small arrows when a panel has rows hidden above or below.
pub fn draw_scroll_indicator(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    has_above: bool,
    has_below: bool,
    color: Color,
) {
    let center = x + width - 6;
    if has_above {
        canvas.fill_triangle(
            (center - 3, y + 3),
            (center + 3, y + 3),
            (center, y + 7),
            color,
        );
    }
    if has_below {
        canvas.fill_triangle(
            (center - 3, y + height - 7),
            (center + 3, y + height - 7),
            (center, y + height - 3),
            color,
        );
    }
}

/// Draw a boxed action key and return its width so the caption can follow.
pub fn draw_keycap(canvas: &mut Canvas, label: &str, x: i32, y: i32, color: Color) -> i32 {
    let label = font::plain_text(label).to_ascii_uppercase();
    let width = font::text_width(&label, 1) + 6;
    canvas.stroke_rect(x, y, width, 12, color);
    canvas.draw_text(&label, x + 3, y + 2, color, 1);
    width
}

/// Draw a segmented bar representing a clamped 0..100 percentage.
pub fn draw_meter(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    percent: u32,
    color: Color,
) {
    let clamped = percent.min(100);
    let filled = ((4 * clamped + 50) / 100) as i32;
    let segment = ((width - 3) / 4).max(1);
    for index in 0..4 {
        let segment_x = x + index * (segment + 1);
        canvas.fill_rect(
            segment_x,
            y,
            segment,
            height,
            if index < filled { color } else { GRID_COLOR },
        );
    }
}

#[derive(Clone)]
struct IconBitmap {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn icon_cache() -> &'static OnceLock<HashMap<String, IconBitmap>> {
    static CACHE: OnceLock<HashMap<String, IconBitmap>> = OnceLock::new();
    &CACHE
}

fn icon_path(name: &str) -> Option<std::path::PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(std::path::PathBuf::from(format!("assets/icons/{name}.png")));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(format!("assets/icons/{name}.png")));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn load_icon_bitmap(name: &str) -> Option<IconBitmap> {
    let path = icon_path(name)?;
    let file = std::fs::File::open(path).ok()?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];
    let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(bytes),
        png::ColorType::Rgb => {
            for c in bytes.chunks(3) {
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for c in bytes.chunks(2) {
                rgba.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &g in bytes {
                rgba.extend_from_slice(&[g, g, g, 255]);
            }
        }
        _ => return None,
    }
    Some(IconBitmap {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn icon_bitmap(name: &str) -> Option<&'static IconBitmap> {
    let cache = icon_cache().get_or_init(|| {
        let mut map = HashMap::new();
        let mut dirs = vec![std::path::PathBuf::from("assets/icons")];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                dirs.push(dir.join("assets/icons"));
            }
        }
        for icons_dir in dirs {
            if let Ok(entries) = std::fs::read_dir(icons_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("png") {
                        continue;
                    }
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        continue;
                    };
                    if map.contains_key(stem) {
                        continue;
                    }
                    if let Some(icon) = load_icon_bitmap(stem) {
                        map.insert(stem.to_string(), icon);
                    }
                }
            }
        }
        map
    });
    cache.get(name)
}

/// Draw a PNG icon from `assets/icons/<name>.png`, tinted with `color`.
/// Falls back to a simple rectangle if the icon is missing.
pub fn draw_icon(canvas: &mut Canvas, name: &str, x: i32, y: i32, size: i32, color: Color) {
    let s = size.max(1);
    if let Some(icon) = icon_bitmap(name) {
        let (r, g, b) = ((color >> 16) & 0xFF, (color >> 8) & 0xFF, color & 0xFF);
        for dy in 0..s {
            let sy = (dy as u32 * icon.height / s as u32).min(icon.height - 1);
            for dx in 0..s {
                let sx = (dx as u32 * icon.width / s as u32).min(icon.width - 1);
                let idx = ((sy * icon.width + sx) * 4) as usize;
                if idx + 3 >= icon.rgba.len() {
                    continue;
                }
                let alpha = icon.rgba[idx + 3];
                if alpha < 32 {
                    continue;
                }
                let a = alpha as u32;
                let pr = ((r * a + 127) / 255) as u8;
                let pg = ((g * a + 127) / 255) as u8;
                let pb = ((b * a + 127) / 255) as u8;
                let px = rgb(pr, pg, pb);
                canvas.fill_rect(x + dx, y + dy, 1, 1, px);
            }
        }
        return;
    }
    canvas.stroke_rect(x + 2, y + 2, s - 4, s - 4, color);
}

// ---------------------------------------------------------------------------
// Global HUD
// ---------------------------------------------------------------------------

/// Fixed header: 6 system indicators, y 0..27.
pub fn draw_global_header(canvas: &mut Canvas, stats: &SystemStats) {
    canvas.fill_rect(0, 0, SCREEN_WIDTH as i32, HEADER_HEIGHT, BG_COLOR);
    canvas.hline(0, SCREEN_WIDTH as i32 - 1, HEADER_HEIGHT - 1, GRID_COLOR);

    let icon_y = 6;
    let text_y = 9; // 14px icon, 8px text -> vertically centered around y=6..20

    // CPU x=4
    draw_icon(canvas, "cpu", 4, icon_y, 14, TEXT_COLOR);
    canvas.draw_text(
        &format!("{}%", stats.cpu_percent),
        22,
        text_y,
        TEXT_COLOR,
        1,
    );

    // RAM x=60
    draw_icon(canvas, "memory", 60, icon_y, 14, TEXT_COLOR);
    canvas.draw_text(
        &format!("{}%", stats.ram_percent),
        78,
        text_y,
        TEXT_COLOR,
        1,
    );

    // Disk x=116
    draw_icon(canvas, "disk", 116, icon_y, 14, TEXT_COLOR);
    canvas.draw_text(
        &format!("{}%", stats.disk_percent),
        134,
        text_y,
        TEXT_COLOR,
        1,
    );

    // Wi-Fi x=176
    let wifi_color = if stats.wifi_connected {
        SUCCESS_COLOR
    } else {
        MUTED_COLOR
    };
    draw_icon(canvas, "wifi", 176, icon_y, 14, wifi_color);

    // Bluetooth x=200
    let bt_color = if stats.bluetooth_active {
        ACCENT_COLOR
    } else {
        MUTED_COLOR
    };
    draw_icon(canvas, "bluetooth", 200, icon_y, 14, bt_color);

    // Battery x=228
    let battery_color = if stats.battery_charging {
        SUCCESS_COLOR
    } else if stats.battery_percent.unwrap_or(100) < 20 {
        ERROR_COLOR
    } else {
        TEXT_COLOR
    };
    draw_icon(canvas, "battery", 228, icon_y, 14, battery_color);
    if stats.battery_charging {
        // small "+" indicator inside/right of battery icon
        canvas.draw_text("+", 240, 7, battery_color, 1);
    }
    let battery_text = match stats.battery_percent {
        Some(value) => format!("{value}%"),
        None => "N/D".to_string(),
    };
    let battery_text_color = if stats.battery_percent.is_none() {
        MUTED_COLOR
    } else {
        battery_color
    };
    canvas.draw_text(&battery_text, 246, text_y, battery_text_color, 1);
}

/// Fixed footer: 4 global hints, y 211..240.
pub fn draw_global_footer(canvas: &mut Canvas) {
    canvas.fill_rect(0, FOOTER_TOP, SCREEN_WIDTH as i32, FOOTER_HEIGHT, BG_COLOR);
    canvas.hline(0, SCREEN_WIDTH as i32 - 1, FOOTER_TOP, GRID_COLOR);
    let hints = [
        ("DPAD Mover", 4),
        ("A Abrir", 96),
        ("START Ok", 172),
        ("B Volver", 252),
    ];
    for (text, x) in hints {
        let max_w = (SCREEN_WIDTH as i32 - x - 4).max(0);
        canvas.draw_text_clipped(text, x, FOOTER_TOP + 11, max_w, TEXT_COLOR, 1);
    }
}

// ---------------------------------------------------------------------------
// Scrolling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub count: usize,
    pub capacity: usize,
    pub selected: usize,
    pub offset: usize,
}

impl ScrollState {
    pub fn new(capacity: usize) -> Self {
        Self {
            count: 0,
            capacity: capacity.max(1),
            selected: 0,
            offset: 0,
        }
    }

    pub fn update(&mut self, count: usize, capacity: Option<usize>) {
        self.count = count;
        if let Some(capacity) = capacity {
            self.capacity = capacity.max(1);
        }
        self.clamp();
    }

    pub fn reset(&mut self, count: usize) {
        self.selected = 0;
        self.offset = 0;
        self.update(count, None);
    }

    fn clamp(&mut self) {
        if self.count == 0 {
            self.selected = 0;
            self.offset = 0;
            return;
        }
        self.selected = self.selected.min(self.count - 1);
        let max_offset = self.count.saturating_sub(self.capacity);
        self.offset = self.offset.min(max_offset);
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + self.capacity {
            self.offset = self.selected + 1 - self.capacity;
        }
    }

    pub fn move_by(&mut self, step: i32) {
        if self.count == 0 {
            return;
        }
        let count = self.count as i32;
        self.selected = ((self.selected as i32 + step) % count + count) as usize % self.count;
        self.clamp();
    }

    pub fn first(&self) -> usize {
        self.offset
    }

    pub fn last(&self) -> usize {
        (self.offset + self.capacity).min(self.count)
    }

    pub fn has_above(&self) -> bool {
        self.offset > 0
    }

    pub fn has_below(&self) -> bool {
        self.last() < self.count
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new(1)
    }
}

// ---------------------------------------------------------------------------
// Help view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct HelpSection {
    pub heading: &'static str,
    pub paragraphs: &'static [&'static str],
}

const HELP_TOP: i32 = 36;
const HELP_BOTTOM: i32 = 178;
const HELP_LINE_STEP: i32 = 11;

pub fn help_capacity() -> usize {
    (((HELP_BOTTOM - HELP_TOP) / HELP_LINE_STEP).max(1)) as usize
}

fn build_help_lines(sections: &[HelpSection]) -> Vec<(String, bool)> {
    let mut lines = Vec::new();
    for section in sections {
        lines.push((font::plain_text(section.heading).to_ascii_uppercase(), true));
        for paragraph in section.paragraphs {
            for line in wrap_text(paragraph, 35) {
                lines.push((line, false));
            }
        }
        lines.push((String::new(), false));
    }
    lines
}

/// Render help sections. The caller's scroll offset is clamped in place.
pub fn draw_help(canvas: &mut Canvas, sections: &[HelpSection], scroll: &mut ScrollState) {
    canvas.fill_rect(
        0,
        CONTENT_TOP,
        SCREEN_WIDTH as i32,
        CONTENT_HEIGHT,
        BG_COLOR,
    );
    let lines = build_help_lines(sections);
    let capacity = help_capacity();
    scroll.update(lines.len(), Some(capacity));
    let first = scroll.offset.min(lines.len().saturating_sub(capacity));
    let mut y = HELP_TOP;
    for (text, heading) in lines.iter().skip(first).take(capacity) {
        if !text.is_empty() {
            canvas.draw_text(
                text,
                16,
                y,
                if *heading { ACCENT_COLOR } else { TEXT_COLOR },
                1,
            );
        }
        y += HELP_LINE_STEP;
    }
    canvas.draw_text("A: VOLVER   B: SALIR", 16, 194, ACCENT_COLOR, 1);
}

// ---------------------------------------------------------------------------
// IPv4 editor
// ---------------------------------------------------------------------------

pub struct Ipv4Editor {
    pub octets: [u8; 4],
    saved: [u8; 4],
    octet: usize,
    digit: usize,
    notice: String,
}

impl Ipv4Editor {
    pub fn new(address: &str) -> Self {
        let octets = crate::core::validation::parse_ipv4(address).unwrap_or([192, 168, 1, 1]);
        Self {
            octets,
            saved: octets,
            octet: 0,
            digit: 0,
            notice: String::new(),
        }
    }

    pub fn address(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }

    pub fn begin(&mut self) {
        self.saved = self.octets;
        self.notice.clear();
    }

    fn change_digit(&mut self, step: i32) {
        let digits = format!("{:03}", self.octets[self.octet]);
        let mut bytes = digits.into_bytes();
        let index = self.digit.min(2);
        let current = (bytes[index] - b'0') as i32;
        bytes[index] = b'0' + (((current + step) % 10 + 10) % 10) as u8;
        let text = String::from_utf8(bytes).unwrap_or_default();
        self.octets[self.octet] = text.parse::<u16>().unwrap_or(0).min(255) as u8;
        self.notice.clear();
    }

    pub fn handle(&mut self, action: Action) -> Option<EditResult> {
        match action {
            Action::Left => {
                if self.digit > 0 {
                    self.digit -= 1;
                } else {
                    self.octet = (self.octet + 3) % 4;
                    self.digit = 2;
                }
                self.notice.clear();
            }
            Action::Right => {
                if self.digit < 2 {
                    self.digit += 1;
                } else {
                    self.octet = (self.octet + 1) % 4;
                    self.digit = 0;
                }
                self.notice.clear();
            }
            Action::Up => self.change_digit(1),
            Action::Down => self.change_digit(-1),
            Action::Open => {
                self.saved = self.octets;
                return Some(EditResult::Saved);
            }
            Action::Back => {
                self.octets = self.saved;
                self.notice.clear();
                return Some(EditResult::Cancelled);
            }
            _ => {}
        }
        None
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        canvas.draw_text("EDITAR IP", 15, 44, ACCENT_COLOR, 1);
        for index in 0..4 {
            let group_x = 18 + index as i32 * 72;
            let label = format!("OCTETO {}", index + 1);
            let color = if index == self.octet {
                ACCENT_COLOR
            } else {
                MUTED_COLOR
            };
            canvas.draw_text(&label, group_x, 74, color, 1);
            let digits = format!("{:03}", self.octets[index]);
            for (digit_index, character) in digits.chars().enumerate() {
                let digit_x = group_x + digit_index as i32 * 12;
                let selected = index == self.octet && digit_index == self.digit;
                if selected {
                    canvas.fill_rect(digit_x - 1, 90, 13, 16, ACCENT_COLOR);
                }
                canvas.draw_text(
                    &character.to_string(),
                    digit_x,
                    92,
                    if selected { BG_COLOR } else { TEXT_COLOR },
                    1,
                );
            }
            if index < 3 {
                canvas.draw_text(".", group_x + 38, 92, ACCENT_COLOR, 1);
            }
        }
        canvas.draw_text(&format!("IP: {}", self.address()), 15, 126, TEXT_COLOR, 1);
        let lines = [
            format!("OCTETO {}/4 DIGITO {}/3", self.octet + 1, self.digit + 1),
            "UP/DOWN DIGITO LEFT/RIGHT MOVER".to_string(),
            "A GUARDAR  B CANCELAR  0-255".to_string(),
        ];
        for (row, line) in lines.iter().enumerate() {
            canvas.draw_text(line, 15, 146 + row as i32 * 14, TEXT_COLOR, 1);
        }
        if !self.notice.is_empty() {
            canvas.draw_text(&self.notice, 15, 194, ACCENT_COLOR, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Bluetooth MAC editor
// ---------------------------------------------------------------------------

pub const DEFAULT_MAC: &str = "FC:FC:48:11:22:33";

pub struct MacEditor {
    pub pairs: [u8; 6],
    saved: [u8; 6],
    position: usize,
    notice: String,
}

impl MacEditor {
    pub fn new(address: &str) -> Self {
        let bytes = crate::core::validation::mac_bytes(address)
            .unwrap_or([0xFC, 0xFC, 0x48, 0x11, 0x22, 0x33]);
        Self {
            pairs: bytes,
            saved: bytes,
            position: 0,
            notice: String::new(),
        }
    }

    pub fn value(&self) -> String {
        self.pairs
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    pub fn begin(&mut self) {
        self.saved = self.pairs;
        self.notice.clear();
    }

    fn current_digit(&self) -> u8 {
        let byte = self.pairs[self.position / 2];
        if self.position % 2 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        }
    }

    fn change_digit(&mut self, step: i32) {
        let pair = self.position / 2;
        let nibble = self.position % 2;
        let current = self.current_digit() as i32;
        let next = ((current + step) % 16 + 16) % 16;
        if nibble == 0 {
            self.pairs[pair] = (self.pairs[pair] & 0x0F) | ((next as u8) << 4);
        } else {
            self.pairs[pair] = (self.pairs[pair] & 0xF0) | next as u8;
        }
        self.notice.clear();
    }

    pub fn handle(&mut self, action: Action) -> Option<EditResult> {
        match action {
            Action::Up => self.change_digit(1),
            Action::Down => self.change_digit(-1),
            Action::Left => {
                self.position = (self.position + 11) % 12;
                self.notice.clear();
            }
            Action::Right => {
                self.position = (self.position + 1) % 12;
                self.notice.clear();
            }
            Action::Open => {
                if let Err(error) = normalize_mac(&self.value()) {
                    self.notice = error;
                } else {
                    self.saved = self.pairs;
                    self.notice.clear();
                    return Some(EditResult::Saved);
                }
            }
            Action::Back => {
                self.pairs = self.saved;
                self.notice.clear();
                return Some(EditResult::Cancelled);
            }
            _ => {}
        }
        None
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        draw_frame(canvas, 10, 32, 300, 30, GRID_COLOR, Some(PANEL_COLOR));
        canvas.draw_text("MAC", 16, 36, MUTED_COLOR, 1);
        let value = self.value();
        let selected_index = self.position + self.position / 2;
        let mut x = 16;
        for (index, character) in value.chars().enumerate() {
            let color = if character == ':' {
                MUTED_COLOR
            } else if index == selected_index {
                ACCENT_COLOR
            } else {
                TEXT_COLOR
            };
            canvas.draw_text(&character.to_string(), x, 48, color, 1);
            x += 8;
        }

        let cell_width = 34;
        let cell_height = 20;
        let grid_x = (SCREEN_WIDTH as i32 - cell_width * 4) / 2;
        for index in 0..16u8 {
            let column = index as i32 % 4;
            let row = index as i32 / 4;
            let cell_x = grid_x + column * cell_width;
            let cell_y = 76 + row * (cell_height + 3);
            let selected = index == self.current_digit();
            if selected {
                canvas.fill_rect(cell_x, cell_y, cell_width - 2, cell_height, ACCENT_COLOR);
            } else {
                canvas.stroke_rect(cell_x, cell_y, cell_width - 2, cell_height, GRID_COLOR);
            }
            let character = format!("{index:X}");
            canvas.draw_text(
                &character,
                cell_x + (cell_width - 2 - 8) / 2,
                cell_y + 6,
                if selected { BG_COLOR } else { TEXT_COLOR },
                1,
            );
        }
        if !self.notice.is_empty() {
            canvas.draw_text(&self.notice, 16, 192, ACCENT_COLOR, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Hostname editor
// ---------------------------------------------------------------------------

const HOSTNAME_GRID: [[&str; 8]; 5] = [
    ["A", "B", "C", "D", "E", "F", "G", "H"],
    ["I", "J", "K", "L", "M", "N", "O", "P"],
    ["Q", "R", "S", "T", "U", "V", "W", "X"],
    ["Y", "Z", "0", "1", "2", "3", "4", "5"],
    ["6", "7", "8", "9", "-", ".", "DEL", "OK"],
];
const HOSTNAME_MAX: usize = 24;

pub struct HostnameEditor {
    characters: Vec<char>,
    saved: Vec<char>,
    row: usize,
    column: usize,
    notice: String,
}

impl HostnameEditor {
    pub fn new(hostname: &str) -> Self {
        let initial = normalize_hostname(hostname).unwrap_or_else(|_| "example.com".to_string());
        let characters: Vec<char> = initial.chars().collect();
        Self {
            saved: characters.clone(),
            characters,
            row: 0,
            column: 0,
            notice: String::new(),
        }
    }

    pub fn value(&self) -> String {
        self.characters.iter().collect()
    }

    pub fn begin(&mut self) {
        self.saved = self.characters.clone();
        self.notice.clear();
    }

    pub fn handle(&mut self, action: Action) -> Option<EditResult> {
        match action {
            Action::Up => {
                self.row = (self.row + 4) % HOSTNAME_GRID.len();
                self.notice.clear();
            }
            Action::Down => {
                self.row = (self.row + 1) % HOSTNAME_GRID.len();
                self.notice.clear();
            }
            Action::Left => {
                self.column = (self.column + 7) % 8;
                self.notice.clear();
            }
            Action::Right => {
                self.column = (self.column + 1) % 8;
                self.notice.clear();
            }
            Action::Open => return self.stamp(),
            Action::Back => {
                self.characters = self.saved.clone();
                self.notice.clear();
                return Some(EditResult::Cancelled);
            }
            _ => {}
        }
        None
    }

    fn stamp(&mut self) -> Option<EditResult> {
        match HOSTNAME_GRID[self.row][self.column] {
            "DEL" => {
                self.characters.pop();
                self.notice.clear();
            }
            "OK" => match normalize_hostname(&self.value()) {
                Ok(_) => {
                    self.saved = self.characters.clone();
                    self.notice.clear();
                    return Some(EditResult::Saved);
                }
                Err(error) => self.notice = error,
            },
            choice => {
                if self.characters.len() < HOSTNAME_MAX {
                    self.characters
                        .push(choice.chars().next().unwrap_or('a').to_ascii_lowercase());
                    self.notice.clear();
                }
            }
        }
        None
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.fill_rect(
            0,
            CONTENT_TOP,
            SCREEN_WIDTH as i32,
            CONTENT_HEIGHT,
            BG_COLOR,
        );
        canvas.fill_rect(10, 32, 300, 30, PANEL_COLOR);
        canvas.stroke_rect(10, 32, 300, 30, GRID_COLOR);
        canvas.draw_text("HOST", 16, 36, MUTED_COLOR, 1);
        let value = self.value();
        canvas.draw_text_clipped(&value, 16, 48, 280, TEXT_COLOR, 1);
        let cursor_x = 16 + value.chars().count().min(34) as i32 * 8;
        canvas.fill_rect(cursor_x, 48, 8, 8, ACCENT_COLOR);

        let cell_width = 26;
        let cell_height = 20;
        let grid_x = (SCREEN_WIDTH as i32 - cell_width * 8) / 2;
        for (row_index, row) in HOSTNAME_GRID.iter().enumerate() {
            for (column_index, choice) in row.iter().enumerate() {
                let x = grid_x + column_index as i32 * cell_width;
                let y = 72 + row_index as i32 * 22;
                let active = row_index == self.row && column_index == self.column;
                if active {
                    canvas.fill_rect(x, y, cell_width - 2, cell_height, ACCENT_COLOR);
                } else {
                    canvas.stroke_rect(x, y, cell_width - 2, cell_height, GRID_COLOR);
                }
                let label = if *choice == "DEL" { "<-" } else { *choice };
                let color = if active {
                    BG_COLOR
                } else if *choice == "DEL" || *choice == "OK" {
                    ACCENT_COLOR
                } else {
                    TEXT_COLOR
                };
                let label_width = font::text_width(label, 1);
                canvas.draw_text(
                    label,
                    x + (cell_width - 2 - label_width) / 2,
                    y + 6,
                    color,
                    1,
                );
            }
        }
        if !self.notice.is_empty() {
            canvas.draw_text_clipped(&self.notice, 16, 192, 288, ACCENT_COLOR, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_state_keeps_selection_visible() {
        let mut scroll = ScrollState::new(3);
        scroll.update(10, None);
        scroll.move_by(9);
        assert_eq!(scroll.selected, 9);
        assert_eq!(scroll.offset, 7);
        assert!(scroll.has_above());
        assert!(!scroll.has_below());
    }

    #[test]
    fn mac_editor_wraps_nibbles() {
        let mut editor = MacEditor::new(DEFAULT_MAC);
        editor.handle(Action::Up);
        assert_eq!(editor.value(), "0C:FC:48:11:22:33");
        editor.handle(Action::Down);
        assert_eq!(editor.value(), "FC:FC:48:11:22:33");
    }

    #[test]
    fn ipv4_editor_changes_the_selected_digit() {
        let mut editor = Ipv4Editor::new("102.168.1.1");
        editor.handle(Action::Up);
        assert_eq!(editor.address(), "202.168.1.1");
    }
}
