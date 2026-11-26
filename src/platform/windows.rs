//! Windows GDI development window.
//!
//! The Pi target uses the framebuffer backend; this backend exists so the same
//! binary can be built and exercised on a normal Windows machine without SDL
//! or any C toolchain.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    LoadCursorW, PeekMessageW, RegisterClassW, ShowWindow, TranslateMessage, UnregisterClassW,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG, PM_REMOVE, SW_SHOW, WM_CLOSE,
    WM_DESTROY, WM_KEYDOWN, WM_SYSKEYDOWN, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

use crate::core::input::{Action, Debouncer};
use crate::gfx::Canvas;
use crate::theme::{WINDOW_HEIGHT, WINDOW_WIDTH};

static EVENTS: Mutex<Vec<Action>> = Mutex::new(Vec::new());
static QUIT: AtomicBool = AtomicBool::new(false);

pub struct GdiBackend {
    hwnd: HWND,
    instance: *mut core::ffi::c_void,
    class_name: Vec<u16>,
    width: i32,
    height: i32,
    debouncer: Debouncer,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            QUIT.store(true, Ordering::SeqCst);
            0
        }
        WM_DESTROY => {
            QUIT.store(true, Ordering::SeqCst);
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let virtual_key = wparam as u16;
            let is_repeat = (lparam as usize & (1 << 30)) != 0;
            if !is_repeat {
                if let Some(action) = key_action(virtual_key) {
                    if let Ok(mut events) = EVENTS.lock() {
                        events.push(action);
                    }
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

impl GdiBackend {
    pub fn new() -> Result<Self, String> {
        let class_name = to_wide("HunterZeroRustWindow");
        let title = to_wide("HunterZero");
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            if instance.is_null() {
                return Err("GetModuleHandleW fallo.".to_string());
            }
            let window_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            if RegisterClassW(&window_class) == 0 {
                let error = GetLastError();
                // ERROR_CLASS_ALREADY_EXISTS = 1410: fine when a previous
                // backend instance registered the same class.
                if error != 1410 {
                    return Err(format!("RegisterClassW fallo (codigo {error})."));
                }
            }

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: WINDOW_WIDTH as i32,
                bottom: WINDOW_HEIGHT as i32,
            };
            AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, 0);
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null_mut(),
            );
            if hwnd == 0 {
                return Err(format!(
                    "CreateWindowExW fallo (codigo {}).",
                    GetLastError()
                ));
            }
            ShowWindow(hwnd, SW_SHOW);
            Ok(Self {
                hwnd,
                instance,
                class_name,
                width: WINDOW_WIDTH as i32,
                height: WINDOW_HEIGHT as i32,
                debouncer: Debouncer::new(),
            })
        }
    }
}

impl super::Backend for GdiBackend {
    fn present(&mut self, canvas: &Canvas) -> Result<(), String> {
        unsafe {
            let device_context = GetDC(self.hwnd);
            if device_context.is_null() {
                return Err("GetDC fallo.".to_string());
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: canvas.width(),
                biHeight: -canvas.height(),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            };
            let result = StretchDIBits(
                device_context,
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                canvas.width(),
                canvas.height(),
                canvas.pixels().as_ptr() as *const core::ffi::c_void,
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(self.hwnd, device_context);
            if result == 0 {
                Err("StretchDIBits fallo.".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn poll(&mut self) -> Vec<Action> {
        let mut message: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        let mut actions = Vec::new();
        if QUIT.load(Ordering::SeqCst) {
            actions.push(Action::Quit);
        }
        if let Ok(mut events) = EVENTS.lock() {
            actions.append(&mut events);
        }
        actions.retain(|action| self.debouncer.accept(*action));
        actions
    }

    fn description(&self) -> String {
        format!("GDI {}x{}", self.width, self.height)
    }
}

impl Drop for GdiBackend {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            UnregisterClassW(self.class_name.as_ptr(), self.instance);
        }
    }
}

fn key_action(virtual_key: u16) -> Option<Action> {
    match virtual_key {
        0x26 | 0x57 => Some(Action::Up),    // Up / W
        0x28 | 0x53 => Some(Action::Down),  // Down / S
        0x25 | 0x41 => Some(Action::Left),  // Left / A
        0x27 | 0x44 => Some(Action::Right), // Right / D
        0x0D | 0x20 => Some(Action::Open),  // Return / Space
        0x1B | 0x08 => Some(Action::Back),  // Escape / Backspace
        0x09 | 0x11 => Some(Action::Start), // Tab / Ctrl
        0x79 => Some(Action::Quit),         // F10
        0x51 => {
            // Ctrl+Q
            let ctrl_down = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
            ctrl_down.then_some(Action::Quit)
        }
        _ => None,
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_keys_map_to_actions() {
        assert_eq!(key_action(0x26), Some(Action::Up));
        assert_eq!(key_action(0x0D), Some(Action::Open));
        assert_eq!(key_action(0x1B), Some(Action::Back));
        assert_eq!(key_action(0x09), Some(Action::Start));
    }
}
