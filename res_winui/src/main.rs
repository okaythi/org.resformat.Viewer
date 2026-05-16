#![windows_subsystem = "windows"]

use std::sync::Mutex;
use std::fs;
use res_core::decode::decode_res_to_rgb;
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Controls::Dialogs::*,
};

// Global state to hold the decoded image
static APP_STATE: Mutex<Option<(i32, i32, Vec<u8>)>> = Mutex::new(None);

const IDM_OPEN: usize = 101;
const IDM_OPEN_NEW: usize = 102;

fn main() -> Result<()> {
    // Intercept OS file path if the user double-clicked a .res file
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path = &args[1];
        load_image_to_state(path);
    }

    unsafe {
        let instance = GetModuleHandleW(None)?;
        let window_class = w!("RES_Viewer_Class");

        let wc = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance.into(),
            lpszClassName: window_class,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        debug_assert!(atom != 0);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
            w!("Res Format Viewer"), 
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, 900, 700,
            None, None, instance, None,
        );

        if hwnd.0 == 0 { return Err(Error::from_win32()); }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        Ok(())
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_CREATE => {
                // Safely unwrap the Result types for the native menus
                if let (Ok(hmenu), Ok(hfilemenu)) = (CreateMenu(), CreatePopupMenu()) {
                    let _ = AppendMenuW(hfilemenu, MF_STRING, IDM_OPEN, w!("Open...\tCtrl+O"));
                    let _ = AppendMenuW(hfilemenu, MF_STRING, IDM_OPEN_NEW, w!("Open in new window"));
                    let _ = AppendMenuW(hmenu, MF_POPUP, hfilemenu.0 as usize, w!("File"));
                    let _ = SetMenu(window, hmenu);
                }
                LRESULT(0)
            }

            WM_COMMAND => {
                let command_id = wparam.0 & 0xFFFF;
                if command_id == IDM_OPEN as usize {
                    open_file_dialog(window);
                } else if command_id == IDM_OPEN_NEW as usize {
                    if let Ok(exe_path) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe_path).spawn();
                    }
                }
                LRESULT(0)
            }

            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(window, &mut ps);

                if let Ok(state) = APP_STATE.lock() {
                    if let Some((width, height, ref pixels)) = *state {
                        let mut rect = RECT::default();
                        GetClientRect(window, &mut rect);
                        let win_w = rect.right - rect.left;
                        let win_h = rect.bottom - rect.top;

                        let mut bmi = BITMAPINFO::default();
                        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                        bmi.bmiHeader.biWidth = width;
                        bmi.bmiHeader.biHeight = -height;
                        bmi.bmiHeader.biPlanes = 1;
                        bmi.bmiHeader.biBitCount = 24;
                        bmi.bmiHeader.biCompression = BI_RGB.0;

                        StretchDIBits(
                            hdc,
                            0, 0, win_w, win_h, 
                            0, 0, width, height,
                            Some(pixels.as_ptr() as *const _),
                            &bmi, DIB_RGB_COLORS, SRCCOPY,
                        );
                    }
                }
                EndPaint(window, &ps);
                LRESULT(0)
            }

            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

fn load_image_to_state(path: &str) -> bool {
    let clean_path = path.trim_end_matches('\0');
    if let Ok(bytes) = fs::read(clean_path) {
        if let Ok((w, h, mut rgb)) = decode_res_to_rgb(&bytes) {
            
            // 1. Swap RGB to BGR for Windows
            for chunk in rgb.chunks_exact_mut(3) {
                chunk.swap(0, 2); 
            }

            // 2. The Stride Fix: Pad rows to a multiple of 4 bytes (DWORD boundary)
            let row_bytes = (w * 3) as usize;
            let padded_row_bytes = (row_bytes + 3) & !3; // Bitwise round up to nearest multiple of 4
            
            let mut padded_bgr = Vec::with_capacity(padded_row_bytes * (h as usize));
            
            for row in rgb.chunks_exact(row_bytes) {
                padded_bgr.extend_from_slice(row);
                // Inject the dead bytes at the end of the row to satisfy Windows
                padded_bgr.resize(padded_bgr.len() + (padded_row_bytes - row_bytes), 0);
            }

            if let Ok(mut state) = APP_STATE.lock() {
                *state = Some((w as i32, h as i32, padded_bgr));
                return true;
            }
        }
    }
    false
}

unsafe fn open_file_dialog(hwnd: HWND) {
    let mut filename = [0u16; 260];
    let mut ofn = OPENFILENAMEW::default();
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFile = PWSTR(filename.as_mut_ptr());
    ofn.nMaxFile = filename.len() as u32;
    ofn.lpstrFilter = w!("RES Images\0*.res\0All Files\0*.*\0\0");

    if GetOpenFileNameW(&mut ofn).as_bool() {
        let path = String::from_utf16_lossy(&filename);
        if load_image_to_state(&path) {
            InvalidateRect(hwnd, None, true); 
        }
    }
}