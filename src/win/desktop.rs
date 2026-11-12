use crate::config::Config;
use crate::win::ffi::*;
use std::process::Child;
use std::ptr;
use std::sync::atomic::{AtomicIsize, Ordering};

pub struct DesktopWallpaper {
    pub hwnd: HWND,
    pub child: Child,
}

static FOUND_WORKERW: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn enum_windows_find_workerw(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let shell_name = to_wide("SHELLDLL_DefView");
    let def_view = FindWindowExW(hwnd, 0, shell_name.as_ptr(), ptr::null());
    if def_view != 0 {
        let worker_name = to_wide("WorkerW");
        let w = FindWindowExW(0, hwnd, worker_name.as_ptr(), ptr::null());
        if w != 0 {
            FOUND_WORKERW.store(w, Ordering::SeqCst);
            return 0; // stop enumerating
        }
    }
    1
}

unsafe fn find_desktop_host() -> HWND {
    let progman_name = to_wide("Progman");
    let progman = FindWindowW(progman_name.as_ptr(), ptr::null());
    if progman != 0 {
        let mut res: usize = 0;
        let _ = SendMessageTimeoutW(progman, 0x052C, 0x000D, 0x0001, SMTO_NORMAL, 1000, &mut res);
        let _ = SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut res);
    }

    FOUND_WORKERW.store(0, Ordering::SeqCst);
    EnumWindows(enum_windows_find_workerw, 0);
    let found = FOUND_WORKERW.load(Ordering::SeqCst);
    if found != 0 {
        return found;
    }
    progman
}

unsafe extern "system" fn host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub fn start_desktop(cfg: &Config) -> Result<DesktopWallpaper, Box<dyn std::error::Error>> {
    unsafe {
        SetProcessDPIAware();
        let hinst = GetModuleHandleW(ptr::null());
        let class_name = to_wide("GLMatrixLiveDesktopHost");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(host_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: 0,
            hCursor: LoadCursorW(0, IDC_ARROW as *const u16),
            hbrBackground: 0,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let workerw = find_desktop_host();
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(GetSystemMetrics(SM_CXSCREEN));
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(GetSystemMetrics(SM_CYSCREEN));

        let title = to_wide("GLMatrixLiveWallpaperHost");
        let hwnd = if workerw != 0 {
            let h = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_DISABLED | WS_CLIPCHILDREN,
                0,
                0,
                vw,
                vh,
                workerw,
                0,
                hinst,
                ptr::null_mut(),
            );
            if h != 0 {
                SetParent(h, workerw);
                MoveWindow(h, 0, 0, vw, vh, 1);
            }
            h
        } else {
            let h = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE | WS_DISABLED | WS_CLIPCHILDREN,
                vx,
                vy,
                vw,
                vh,
                0,
                0,
                hinst,
                ptr::null_mut(),
            );
            if h != 0 {
                SetWindowPos(h, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            h
        };

        if hwnd == 0 {
            return Err("Gagal membuat jendela host desktop WorkerW".into());
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let child = cfg.spawn_glmatrix(&cfg.desktop, hwnd as usize as u64)?;
        Ok(DesktopWallpaper { hwnd, child })
    }
}

pub fn stop_desktop(sess: &mut Option<DesktopWallpaper>) {
    if let Some(mut s) = sess.take() {
        let _ = s.child.kill();
        let _ = s.child.wait();
        unsafe {
            if s.hwnd != 0 {
                DestroyWindow(s.hwnd);
            }
        }
    }
}
