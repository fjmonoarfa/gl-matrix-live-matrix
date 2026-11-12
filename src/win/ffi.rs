#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::os::raw::{c_int, c_uint, c_void};

pub type HWND = isize;
pub type HDC = isize;
pub type HGLRC = isize;
pub type HINSTANCE = isize;
pub type HMENU = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HBRUSH = isize;
pub type HFONT = isize;
pub type HKEY = isize;
pub type HANDLE = isize;
pub type ATOM = u16;
pub type BOOL = i32;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;

pub type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
pub type WNDENUMPROC = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

// Window styles & messages
pub const WS_OVERLAPPED: u32 = 0x0000_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_DISABLED: u32 = 0x0800_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_MINIMIZEBOX: u32 = 0x0002_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_BORDER: u32 = 0x0080_0000;

pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

pub const BS_PUSHBUTTON: u32 = 0x0000_0000;
pub const BS_DEFPUSHBUTTON: u32 = 0x0000_0001;
pub const BS_AUTOCHECKBOX: u32 = 0x0000_0003;
pub const BS_GROUPBOX: u32 = 0x0000_0007;

pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const ES_NUMBER: u32 = 0x2000;
pub const ES_AUTOHSCROLL: u32 = 0x0080;
pub const TBS_HORZ: u32 = 0x0000;
pub const TBS_AUTOTICKS: u32 = 0x0001;

pub const CS_OWNDC: u32 = 0x0020;
pub const CS_HREDRAW: u32 = 0x0002;
pub const CS_VREDRAW: u32 = 0x0001;

pub const CW_USEDEFAULT: i32 = -2147483648;

pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_SETFONT: u32 = 0x0030;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_HSCROLL: u32 = 0x0114;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_RBUTTONDOWN: u32 = 0x0204;

pub const BM_GETCHECK: u32 = 0x00F0;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const BST_UNCHECKED: usize = 0;
pub const BST_CHECKED: usize = 1;

pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CB_SETCURSEL: u32 = 0x014E;

pub const TBM_GETPOS: u32 = 0x0400;
pub const TBM_SETPOS: u32 = 0x0405;
pub const TBM_SETRANGE: u32 = 0x0406;

pub const PM_REMOVE: u32 = 0x0001;

pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;

pub const SMTO_NORMAL: u32 = 0x0000;
pub const HWND_BOTTOM: HWND = 1;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SW_SHOW: i32 = 5;

pub const COLOR_WINDOW: isize = 5;
pub const IDC_ARROW: usize = 32512;
pub const DEFAULT_GUI_FONT: i32 = 17;

pub const PFD_TYPE_RGBA: u8 = 0;
pub const PFD_MAIN_PLANE: u8 = 0;
pub const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
pub const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
pub const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;

pub const HKEY_CURRENT_USER: HKEY = -0x7FFF_FFFFisize; // 0x80000001
pub const KEY_SET_VALUE: u32 = 0x0002;
pub const KEY_QUERY_VALUE: u32 = 0x0001;
pub const REG_SZ: u32 = 1;
pub const ERROR_ALREADY_EXISTS: u32 = 183;

pub const MB_OK: u32 = 0x0000_0000;
pub const MB_YESNO: u32 = 0x0000_0004;
pub const MB_ICONINFORMATION: u32 = 0x0000_0040;
pub const IDYES: i32 = 6;

pub const ICC_BAR_CLASSES: u32 = 0x0000_0004;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
}

#[repr(C)]
pub struct WNDCLASSW {
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
pub struct PIXELFORMATDESCRIPTOR {
    pub nSize: u16,
    pub nVersion: u16,
    pub dwFlags: u32,
    pub iPixelType: u8,
    pub cColorBits: u8,
    pub cRedBits: u8,
    pub cRedShift: u8,
    pub cGreenBits: u8,
    pub cGreenShift: u8,
    pub cBlueBits: u8,
    pub cBlueShift: u8,
    pub cAlphaBits: u8,
    pub cAlphaShift: u8,
    pub cAccumBits: u8,
    pub cAccumRedBits: u8,
    pub cAccumGreenBits: u8,
    pub cAccumBlueBits: u8,
    pub cAccumAlphaBits: u8,
    pub cDepthBits: u8,
    pub cStencilBits: u8,
    pub cAuxBuffers: u8,
    pub iLayerType: u8,
    pub bReserved: u8,
    pub dwLayerMask: u32,
    pub dwVisibleMask: u32,
    pub dwDamageMask: u32,
}

#[repr(C)]
pub struct LASTINPUTINFO {
    pub cbSize: u32,
    pub dwTime: u32,
}

#[repr(C)]
pub struct INITCOMMONCONTROLSEX {
    pub dwSize: u32,
    pub dwICC: u32,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    pub fn GetLastError() -> u32;
    pub fn GetTickCount() -> u32;
    pub fn CreateMutexW(lpMutexAttributes: *mut c_void, bInitialOwner: BOOL, lpName: *const u16) -> HANDLE;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM;
    pub fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn DefWindowProcW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: u32, wMsgFilterMax: u32) -> BOOL;
    pub fn PeekMessageW(
        lpMsg: *mut MSG,
        hWnd: HWND,
        wMsgFilterMin: u32,
        wMsgFilterMax: u32,
        wRemoveMsg: u32,
    ) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn SendMessageW(hWnd: HWND, Msg: u32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn SendMessageTimeoutW(
        hWnd: HWND,
        Msg: u32,
        wParam: WPARAM,
        lParam: LPARAM,
        fuFlags: u32,
        uTimeout: u32,
        lpdwResult: *mut usize,
    ) -> LRESULT;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    pub fn FindWindowExW(
        hWndParent: HWND,
        hWndChildAfter: HWND,
        lpszClass: *const u16,
        lpszWindow: *const u16,
    ) -> HWND;
    pub fn EnumWindows(lpEnumFunc: WNDENUMPROC, lParam: LPARAM) -> BOOL;
    pub fn SetParent(hWndChild: HWND, hWndNewParent: HWND) -> HWND;
    pub fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> BOOL;
    pub fn MoveWindow(hWnd: HWND, X: i32, Y: i32, nWidth: i32, nHeight: i32, bRepaint: BOOL) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn IsWindow(hWnd: HWND) -> BOOL;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    pub fn ShowCursor(bShow: BOOL) -> i32;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> BOOL;
    pub fn LockWorkStation() -> BOOL;
    pub fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: u32, lpTimerFunc: *mut c_void) -> usize;
    pub fn KillTimer(hWnd: HWND, uIDEvent: usize) -> BOOL;
    pub fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
    pub fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    pub fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    pub fn SetProcessDPIAware() -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn ChoosePixelFormat(hdc: HDC, ppfd: *const PIXELFORMATDESCRIPTOR) -> i32;
    pub fn SetPixelFormat(hdc: HDC, format: i32, ppfd: *const PIXELFORMATDESCRIPTOR) -> BOOL;
    pub fn SwapBuffers(hdc: HDC) -> BOOL;
    pub fn GetStockObject(i: i32) -> isize;
}

#[link(name = "opengl32")]
extern "system" {
    pub fn wglCreateContext(hdc: HDC) -> HGLRC;
    pub fn wglDeleteContext(hglrc: HGLRC) -> BOOL;
    pub fn wglMakeCurrent(hdc: HDC, hglrc: HGLRC) -> BOOL;
    pub fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
    pub fn glMatrixMode(mode: c_uint);
    pub fn glLoadIdentity();
    pub fn glFrustum(left: f64, right: f64, bottom: f64, top: f64, zNear: f64, zFar: f64);
    pub fn glOrtho(left: f64, right: f64, bottom: f64, top: f64, zNear: f64, zFar: f64);
    pub fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
    pub fn glClear(mask: c_uint);
    pub fn glEnable(cap: c_uint);
    pub fn glDisable(cap: c_uint);
    pub fn glBlendFunc(sfactor: c_uint, dfactor: c_uint);
    pub fn glGenTextures(n: c_int, textures: *mut c_uint);
    pub fn glBindTexture(target: c_uint, texture: c_uint);
    pub fn glTexImage2D(
        target: c_uint,
        level: c_int,
        internalformat: c_int,
        width: c_int,
        height: c_int,
        border: c_int,
        format: c_uint,
        type_: c_uint,
        pixels: *const c_void,
    );
    pub fn glTexParameteri(target: c_uint, pname: c_uint, param: c_int);
    pub fn glBegin(mode: c_uint);
    pub fn glEnd();
    pub fn glTexCoord2f(s: f32, t: f32);
    pub fn glVertex3f(x: f32, y: f32, z: f32);
    pub fn glVertex2f(x: f32, y: f32);
    pub fn glColor4f(red: f32, green: f32, blue: f32, alpha: f32);
    pub fn glRotatef(angle: f32, x: f32, y: f32, z: f32);
    pub fn glTranslatef(x: f32, y: f32, z: f32);
    pub fn glFogi(pname: c_uint, param: c_int);
    pub fn glFogf(pname: c_uint, param: f32);
    pub fn glFogfv(pname: c_uint, params: *const f32);
    pub fn glPolygonMode(face: c_uint, mode: c_uint);
}

#[link(name = "comctl32")]
extern "system" {
    pub fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> BOOL;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut HKEY,
    ) -> i32;
    pub fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: u32,
    ) -> i32;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: *const u16) -> i32;
    pub fn RegQueryValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    pub fn RegCloseKey(hKey: HKEY) -> i32;
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
