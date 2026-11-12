use crate::matrix_sim::{GlFns, MatrixScene, RenderArgs};
use crate::win::ffi::*;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Duration;

static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);
static IS_SCREENSAVER: AtomicBool = AtomicBool::new(false);
static INIT_MOUSE_X: AtomicI32 = AtomicI32::new(0);
static INIT_MOUSE_Y: AtomicI32 = AtomicI32::new(0);
static HAS_INIT_MOUSE: AtomicBool = AtomicBool::new(false);

fn win_gl_fns() -> GlFns {
    GlFns {
        viewport: glViewport,
        matrix_mode: glMatrixMode,
        load_identity: glLoadIdentity,
        frustum: glFrustum,
        ortho: glOrtho,
        clear_color: glClearColor,
        clear: glClear,
        enable: glEnable,
        disable: glDisable,
        blend_func: glBlendFunc,
        gen_textures: glGenTextures,
        bind_texture: glBindTexture,
        tex_image_2d: glTexImage2D,
        tex_parameteri: glTexParameteri,
        begin: glBegin,
        end: glEnd,
        tex_coord_2f: glTexCoord2f,
        vertex_3f: glVertex3f,
        vertex_2f: glVertex2f,
        color_4f: glColor4f,
        rotatef: glRotatef,
        translatef: glTranslatef,
        fogi: glFogi,
        fogf: glFogf,
        fogfv: glFogfv,
        polygon_mode: glPolygonMode,
    }
}

unsafe extern "system" fn render_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY | WM_CLOSE => {
            SHOULD_EXIT.store(true, Ordering::SeqCst);
            PostQuitMessage(0);
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if IS_SCREENSAVER.load(Ordering::SeqCst) || wparam == 0x1B || wparam == b'Q' as usize {
                SHOULD_EXIT.store(true, Ordering::SeqCst);
                PostQuitMessage(0);
            }
            0
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            if IS_SCREENSAVER.load(Ordering::SeqCst) {
                SHOULD_EXIT.store(true, Ordering::SeqCst);
                PostQuitMessage(0);
            }
            0
        }
        WM_MOUSEMOVE => {
            if IS_SCREENSAVER.load(Ordering::SeqCst) {
                let mut pt = POINT::default();
                GetCursorPos(&mut pt);
                if !HAS_INIT_MOUSE.load(Ordering::SeqCst) {
                    INIT_MOUSE_X.store(pt.x, Ordering::SeqCst);
                    INIT_MOUSE_Y.store(pt.y, Ordering::SeqCst);
                    HAS_INIT_MOUSE.store(true, Ordering::SeqCst);
                } else {
                    let dx = (pt.x - INIT_MOUSE_X.load(Ordering::SeqCst)).abs();
                    let dy = (pt.y - INIT_MOUSE_Y.load(Ordering::SeqCst)).abs();
                    if dx > 6 || dy > 6 {
                        SHOULD_EXIT.store(true, Ordering::SeqCst);
                        PostQuitMessage(0);
                    }
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run_renderer(args: &RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    SHOULD_EXIT.store(false, Ordering::SeqCst);
    IS_SCREENSAVER.store(args.screensaver, Ordering::SeqCst);

    unsafe {
        SetProcessDPIAware();
        let hinst = GetModuleHandleW(ptr::null());
        let class_name = to_wide("GLMatrixLiveRenderClass");

        let wc = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(render_wnd_proc),
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

        let mut parent_hwnd: Option<HWND> = None;
        let hwnd: HWND = if let Some(wid) = args.window_id {
            let p = wid as HWND;
            if IsWindow(p) == 0 {
                return Err("Parent HWND tidak valid".into());
            }
            parent_hwnd = Some(p);
            let mut rc = RECT::default();
            GetClientRect(p, &mut rc);
            let w = (rc.right - rc.left).max(1);
            let h = (rc.bottom - rc.top).max(1);
            let title = to_wide("GLMatrixChild");
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN | WS_DISABLED,
                0,
                0,
                w,
                h,
                p,
                0,
                hinst,
                ptr::null_mut(),
            )
        } else if args.screensaver {
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(GetSystemMetrics(SM_CXSCREEN));
            let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(GetSystemMetrics(SM_CYSCREEN));
            let title = to_wide("GLMatrix Screensaver");
            GetCursorPos(&mut INIT_MOUSE);
            HAS_INIT_MOUSE = true;
            ShowCursor(0);
            CreateWindowExW(
                WS_EX_TOPMOST,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                vx,
                vy,
                vw,
                vh,
                0,
                0,
                hinst,
                ptr::null_mut(),
            )
        } else {
            let title = to_wide("GLMatrix Preview");
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                820,
                580,
                0,
                0,
                hinst,
                ptr::null_mut(),
            )
        };

        if hwnd == 0 {
            if args.screensaver {
                ShowCursor(1);
            }
            return Err("Gagal membuat jendela OpenGL Windows".into());
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let hdc = GetDC(hwnd);
        if hdc == 0 {
            DestroyWindow(hwnd);
            if args.screensaver {
                ShowCursor(1);
            }
            return Err("GetDC gagal".into());
        }

        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 8,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 16,
            cStencilBits: 0,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };

        let pf = ChoosePixelFormat(hdc, &pfd);
        if pf == 0 || SetPixelFormat(hdc, pf, &pfd) == 0 {
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            if args.screensaver {
                ShowCursor(1);
            }
            return Err("SetPixelFormat OpenGL gagal".into());
        }

        let hglrc = wglCreateContext(hdc);
        if hglrc == 0 || wglMakeCurrent(hdc, hglrc) == 0 {
            if hglrc != 0 {
                wglDeleteContext(hglrc);
            }
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            if args.screensaver {
                ShowCursor(1);
            }
            return Err("wglCreateContext / wglMakeCurrent gagal".into());
        }

        let gl = win_gl_fns();
        let mut scene = MatrixScene::new(&gl, args.clone());
        let mut msg: MSG = std::mem::zeroed();
        let mut last_w = 0u32;
        let mut last_h = 0u32;

        while !SHOULD_EXIT.load(Ordering::SeqCst) {
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    SHOULD_EXIT.store(true, Ordering::SeqCst);
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if SHOULD_EXIT.load(Ordering::SeqCst) {
                break;
            }

            if let Some(p) = parent_hwnd {
                if IsWindow(p) == 0 {
                    break;
                }
                let mut prc = RECT::default();
                GetClientRect(p, &mut prc);
                let pw = (prc.right - prc.left).max(1) as u32;
                let ph = (prc.bottom - prc.top).max(1) as u32;
                if pw != last_w || ph != last_h {
                    MoveWindow(hwnd, 0, 0, pw as i32, ph as i32, 0);
                    last_w = pw;
                    last_h = ph;
                }
            } else {
                let mut rc = RECT::default();
                GetClientRect(hwnd, &mut rc);
                last_w = (rc.right - rc.left).max(1) as u32;
                last_h = (rc.bottom - rc.top).max(1) as u32;
            }

            scene.render_frame(&gl, last_w.max(1), last_h.max(1));
            SwapBuffers(hdc);

            if args.delay_us > 0 {
                std::thread::sleep(Duration::from_micros(args.delay_us as u64));
            }
        }

        wglMakeCurrent(0, 0);
        wglDeleteContext(hglrc);
        ReleaseDC(hwnd, hdc);
        DestroyWindow(hwnd);
        if args.screensaver {
            ShowCursor(1);
        }
    }

    Ok(())
}
