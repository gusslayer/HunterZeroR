//! Linux framebuffer (`/dev/fb0`) display and evdev input.
//!
//! This is the path used on the GPi Case 2W. With `vc4-kms-dpi-generic` the
//! kernel exposes a 640x480 framebuffer through fbdev emulation; no X server,
//! SDL or GLES is required.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::core::input::{Action, Debouncer};
use crate::core::logging;
use crate::gfx::Canvas;

const FBIOBLANK: libc::c_ulong = 0x4611;
const FB_BLANK_UNBLANK: libc::c_int = 0;

fn hide_console_cursor() {
    // Oculta el cursor parpadeante del VT para que no se superponga al framebuffer.
    for path in ["/dev/tty1", "/dev/tty0", "/dev/console"] {
        if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(path) {
            let _ = std::io::Write::write_all(&mut file, b"\x1b[?25l\x1b[?1c");
            let _ = std::io::Write::flush(&mut file);
        }
    }
    let _ = std::fs::write("/sys/class/graphics/fbcon/cursor_blink", b"0");
}

pub struct FbBackend {
    file: File,
    map: *mut u8,
    len: usize,
    width: usize,
    height: usize,
    stride: usize,
    bpp: usize,
    input: EvdevInput,
    debouncer: Debouncer,
    description: String,
}

impl FbBackend {
    pub fn new() -> Result<Self, String> {
        let device = std::env::var("HUNTERZERO_FB")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                (0..4)
                    .map(|index| PathBuf::from(format!("/dev/fb{index}")))
                    .find(|path| path.exists())
            })
            .ok_or_else(|| display_error("no se encontro /dev/fb0"))?;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&device)
            .map_err(|error| {
                display_error(&format!("no se pudo abrir {}: {error}", device.display()))
            })?;

        let name = device
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "fb0".to_string());
        let sysfs = PathBuf::from("/sys/class/graphics").join(&name);
        let (width, height) = read_virtual_size(&sysfs).unwrap_or((640, 480));
        let bpp = read_number(&sysfs.join("bits_per_pixel")).unwrap_or(32);
        let stride = read_number(&sysfs.join("stride")).unwrap_or(width * bpp / 8);
        if bpp != 16 && bpp != 24 && bpp != 32 {
            return Err(display_error(&format!(
                "formato de framebuffer no soportado: {bpp} bpp"
            )));
        }
        if width == 0 || height == 0 || stride < width * bpp / 8 {
            return Err(display_error("geometria de framebuffer invalida"));
        }
        let len = stride
            .checked_mul(height)
            .ok_or_else(|| display_error("tamano de framebuffer desbordado"))?;
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                std::os::unix::io::AsRawFd::as_raw_fd(&file),
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(display_error("mmap del framebuffer fallo"));
        }
        let _ = unsafe {
            libc::ioctl(
                std::os::unix::io::AsRawFd::as_raw_fd(&file),
                FBIOBLANK,
                FB_BLANK_UNBLANK,
            )
        };

        hide_console_cursor();
        let input = EvdevInput::open_all();
        logging::log(&format!(
            "Framebuffer {} {}x{} {}bpp stride {}; {} dispositivos de entrada",
            device.display(),
            width,
            height,
            bpp,
            stride,
            input.count()
        ));

        Ok(Self {
            file,
            map: map as *mut u8,
            len,
            width,
            height,
            stride,
            bpp,
            input,
            debouncer: Debouncer::new(),
            description: format!("framebuffer {}x{} {}bpp", width, height, bpp),
        })
    }

    fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
        let offset = y * self.stride + x * self.bpp / 8;
        if offset + self.bpp / 8 > self.len {
            return;
        }
        unsafe {
            let pointer = self.map.add(offset);
            match self.bpp {
                32 => std::ptr::write_unaligned(pointer as *mut u32, color),
                24 => {
                    let red = ((color >> 16) & 0xFF) as u8;
                    let green = ((color >> 8) & 0xFF) as u8;
                    let blue = (color & 0xFF) as u8;
                    *pointer = blue;
                    *pointer.add(1) = green;
                    *pointer.add(2) = red;
                }
                16 => {
                    let red = ((color >> 16) & 0xFF) as u16;
                    let green = ((color >> 8) & 0xFF) as u16;
                    let blue = (color & 0xFF) as u16;
                    let packed = ((red >> 3) << 11) | ((green >> 2) << 5) | (blue >> 3);
                    std::ptr::write_unaligned(pointer as *mut u16, packed);
                }
                _ => {}
            }
        }
    }
}

impl super::Backend for FbBackend {
    fn present(&mut self, canvas: &Canvas) -> Result<(), String> {
        let canvas_width = canvas.width() as usize;
        let canvas_height = canvas.height() as usize;
        let scale = (self.width / canvas_width)
            .min(self.height / canvas_height)
            .max(1);
        let offset_x = (self.width.saturating_sub(canvas_width * scale)) / 2;
        let offset_y = (self.height.saturating_sub(canvas_height * scale)) / 2;
        let pixels = canvas.pixels();
        for y in 0..canvas_height {
            for x in 0..canvas_width {
                let color = pixels[y * canvas_width + x];
                for dy in 0..scale {
                    let target_y = offset_y + y * scale + dy;
                    if target_y >= self.height {
                        continue;
                    }
                    for dx in 0..scale {
                        let target_x = offset_x + x * scale + dx;
                        if target_x < self.width {
                            self.write_pixel(target_x, target_y, color);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn poll(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        for action in self.input.poll() {
            if self.debouncer.accept(action) {
                actions.push(action);
            }
        }
        actions
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

impl Drop for FbBackend {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.map as *mut libc::c_void, self.len);
        }
        let _ = &self.file;
    }
}

fn read_number(path: &Path) -> Option<usize> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| content.trim().parse().ok())
}

fn read_virtual_size(sysfs: &Path) -> Option<(usize, usize)> {
    let content = std::fs::read_to_string(sysfs.join("virtual_size")).ok()?;
    let (width, height) = content.trim().split_once(',')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

fn display_error(detail: &str) -> String {
    let mut message = format!("No se pudo iniciar la pantalla: {detail}.");
    let nodes = ["/dev/dri/card0", "/dev/fb0", "/dev/fb1"];
    let found: Vec<&str> = nodes
        .iter()
        .copied()
        .filter(|path| Path::new(path).exists())
        .collect();
    if found.is_empty() {
        message.push_str(
            " No se detectan /dev/dri/card* ni /dev/fb*: revisa los overlays KMS/DPI de /boot/firmware/config.txt y los mensajes vc4/drm del kernel.",
        );
    } else {
        message.push_str(&format!(" Nodos detectados: {}.", found.join(", ")));
    }
    message
}

// ---------------------------------------------------------------------------
// evdev
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const ABS_HAT0X: u16 = 16;
const ABS_HAT0Y: u16 = 17;
const KEY_ESC: u16 = 1;
const KEY_BACKSPACE: u16 = 14;
const KEY_TAB: u16 = 15;
const KEY_Q: u16 = 16;
const KEY_ENTER: u16 = 28;
const KEY_LEFTCTRL: u16 = 29;
const KEY_SPACE: u16 = 57;
const KEY_F10: u16 = 68;
const KEY_KPENTER: u16 = 96;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_UP: u16 = 103;
const KEY_LEFT: u16 = 105;
const KEY_RIGHT: u16 = 106;
const KEY_DOWN: u16 = 108;
const BTN_A: u16 = 304;
const BTN_B: u16 = 305;
const BTN_NORTH: u16 = 307;
const BTN_WEST: u16 = 308;
const BTN_START: u16 = 315;
const BTN_MODE: u16 = 316;
const BTN_DPAD_UP: u16 = 544;
const BTN_DPAD_DOWN: u16 = 545;
const BTN_DPAD_LEFT: u16 = 546;
const BTN_DPAD_RIGHT: u16 = 547;

struct EvdevInput {
    devices: Vec<File>,
    ctrl: bool,
}

impl EvdevInput {
    fn open_all() -> Self {
        let mut devices = Vec::new();
        for index in 0..32 {
            let path = format!("/dev/input/event{index}");
            match OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)
            {
                Ok(file) => devices.push(file),
                Err(_) => continue,
            }
        }
        if devices.is_empty() {
            logging::log(
                "No se pudo abrir ningun /dev/input/event*; la interfaz funcionara sin botones fisicos.",
            );
        }
        Self {
            devices,
            ctrl: false,
        }
    }

    fn count(&self) -> usize {
        self.devices.len()
    }

    fn poll(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        let event_size = std::mem::size_of::<InputEvent>();
        if event_size == 0 || event_size > 32 {
            return actions;
        }
        let mut buffer = [0u8; 32 * 64];
        for device in &mut self.devices {
            loop {
                match device.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        for chunk in buffer[..read].chunks_exact(event_size) {
                            let event = unsafe {
                                std::ptr::read_unaligned(chunk.as_ptr() as *const InputEvent)
                            };
                            if event.type_ == EV_KEY {
                                if event.value != 1 {
                                    if event.value == 0
                                        && (event.code == KEY_LEFTCTRL
                                            || event.code == KEY_RIGHTCTRL)
                                    {
                                        self.ctrl = false;
                                    }
                                    continue;
                                }
                                if event.code == KEY_LEFTCTRL || event.code == KEY_RIGHTCTRL {
                                    self.ctrl = true;
                                    continue;
                                }
                                if let Some(action) = key_action(event.code, self.ctrl) {
                                    actions.push(action);
                                }
                            } else if event.type_ == EV_ABS {
                                if let Some(action) = hat_action(event.code, event.value) {
                                    actions.push(action);
                                }
                            }
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }
        actions
    }
}

fn key_action(code: u16, ctrl: bool) -> Option<Action> {
    match code {
        KEY_UP | BTN_DPAD_UP => Some(Action::Up),
        KEY_DOWN | BTN_DPAD_DOWN => Some(Action::Down),
        KEY_LEFT | BTN_DPAD_LEFT => Some(Action::Left),
        KEY_RIGHT | BTN_DPAD_RIGHT => Some(Action::Right),
        KEY_ENTER | KEY_KPENTER | KEY_SPACE | BTN_A | BTN_NORTH => Some(Action::Open),
        KEY_ESC | KEY_BACKSPACE | BTN_B => Some(Action::Back),
        KEY_TAB | BTN_START | BTN_MODE => Some(Action::Start),
        KEY_F10 => Some(Action::Quit),
        KEY_Q if ctrl => Some(Action::Quit),
        _ => None,
    }
}

fn hat_action(code: u16, value: i32) -> Option<Action> {
    match (code, value) {
        (ABS_HAT0Y, -1) => Some(Action::Up),
        (ABS_HAT0Y, 1) => Some(Action::Down),
        (ABS_HAT0X, -1) => Some(Action::Left),
        (ABS_HAT0X, 1) => Some(Action::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evdev_maps_the_gpi_buttons() {
        assert_eq!(key_action(KEY_UP, false), Some(Action::Up));
        assert_eq!(key_action(BTN_A, false), Some(Action::Open));
        assert_eq!(key_action(BTN_B, false), Some(Action::Back));
        assert_eq!(key_action(BTN_START, false), Some(Action::Start));
        assert_eq!(key_action(KEY_Q, true), Some(Action::Quit));
        assert_eq!(key_action(KEY_Q, false), None);
    }
}
