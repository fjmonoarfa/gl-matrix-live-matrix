use crate::matrix_sim::{GlFns, MatrixScene, RenderArgs};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static X_ERROR_OCCURRED: AtomicBool = AtomicBool::new(false);

const GLX_RGBA: c_int = 4;
const GLX_DOUBLEBUFFER: c_int = 5;
const GLX_DEPTH_SIZE: c_int = 12;
const GLX_RED_SIZE: c_int = 8;
const GLX_GREEN_SIZE: c_int = 9;
const GLX_BLUE_SIZE: c_int = 10;

const INPUT_OUTPUT: c_uint = 1;
const ALLOC_NONE: c_int = 0;
const CW_BACK_PIXEL: c_ulong = 1 << 1;
const CW_BORDER_PIXEL: c_ulong = 1 << 3;
const CW_EVENT_MASK: c_ulong = 1 << 11;
const CW_COLORMAP: c_ulong = 1 << 13;

const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;
const KEY_PRESS_MASK: c_long = 1 << 0;
const CLIENT_MESSAGE: c_int = 33;
const DESTROY_NOTIFY: c_int = 17;
const KEY_PRESS: c_int = 2;

#[repr(C)]
struct XVisualInfo {
    visual: *mut c_void,
    visualid: c_ulong,
    screen: c_int,
    depth: c_int,
    class: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    colormap_size: c_int,
    bits_per_rgb: c_int,
}

#[repr(C)]
struct XSetWindowAttributes {
    background_pixmap: c_ulong,
    background_pixel: c_ulong,
    border_pixmap: c_ulong,
    border_pixel: c_ulong,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: c_int,
    event_mask: c_long,
    do_not_propagate_mask: c_long,
    override_redirect: c_int,
    colormap: c_ulong,
    cursor: c_ulong,
}

#[repr(C)]
union XEvent {
    ev_type: c_int,
    pad: [c_long; 24],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XClientMessageEvent {
    ev_type: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    message_type: c_ulong,
    format: c_int,
    data_l: [c_long; 5],
}

type XErrorHandlerFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;

unsafe extern "C" fn on_x_error(_dpy: *mut c_void, _err: *mut c_void) -> c_int {
    X_ERROR_OCCURRED.store(true, Ordering::SeqCst);
    0
}

struct X11GlxLib {
    x_open_display: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    x_close_display: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_default_screen: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_root_window: unsafe extern "C" fn(*mut c_void, c_int) -> c_ulong,
    x_create_colormap: unsafe extern "C" fn(*mut c_void, c_ulong, *mut c_void, c_int) -> c_ulong,
    x_create_window: unsafe extern "C" fn(
        *mut c_void,
        c_ulong,
        c_int,
        c_int,
        c_uint,
        c_uint,
        c_uint,
        c_int,
        c_uint,
        *mut c_void,
        c_ulong,
        *mut XSetWindowAttributes,
    ) -> c_ulong,
    x_map_window: unsafe extern "C" fn(*mut c_void, c_ulong) -> c_int,
    x_destroy_window: unsafe extern "C" fn(*mut c_void, c_ulong) -> c_int,
    x_resize_window: unsafe extern "C" fn(*mut c_void, c_ulong, c_uint, c_uint) -> c_int,
    x_store_name: unsafe extern "C" fn(*mut c_void, c_ulong, *const c_char) -> c_int,
    x_intern_atom: unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_ulong,
    x_set_wm_protocols: unsafe extern "C" fn(*mut c_void, c_ulong, *mut c_ulong, c_int) -> c_int,
    x_get_geometry: unsafe extern "C" fn(
        *mut c_void,
        c_ulong,
        *mut c_ulong,
        *mut c_int,
        *mut c_int,
        *mut c_uint,
        *mut c_uint,
        *mut c_uint,
        *mut c_uint,
    ) -> c_int,
    x_pending: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_next_event: unsafe extern "C" fn(*mut c_void, *mut XEvent) -> c_int,
    x_flush: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_free: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_set_error_handler: unsafe extern "C" fn(Option<XErrorHandlerFn>) -> Option<XErrorHandlerFn>,
    x_lookup_keysym: unsafe extern "C" fn(*mut XEvent, c_int) -> c_ulong,
    glx_choose_visual: unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> *mut XVisualInfo,
    glx_create_context: unsafe extern "C" fn(*mut c_void, *mut XVisualInfo, *mut c_void, c_int) -> *mut c_void,
    glx_destroy_context: unsafe extern "C" fn(*mut c_void, *mut c_void),
    glx_make_current: unsafe extern "C" fn(*mut c_void, c_ulong, *mut c_void) -> c_int,
    glx_swap_buffers: unsafe extern "C" fn(*mut c_void, c_ulong),
    gl: GlFns,
}

unsafe fn load_sym<T>(handle: *mut c_void, name: &str) -> Result<T, String> {
    let cname = CString::new(name).map_err(|e| e.to_string())?;
    let sym = libc::dlsym(handle, cname.as_ptr());
    if sym.is_null() {
        Err(format!("Simbol {} tidak ditemukan", name))
    } else {
        Ok(std::mem::transmute_copy(&sym))
    }
}

unsafe fn open_lib(names: &[&str]) -> Result<*mut c_void, String> {
    for name in names {
        let cname = CString::new(*name).map_err(|e| e.to_string())?;
        let h = libc::dlopen(cname.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
        if !h.is_null() {
            return Ok(h);
        }
    }
    Err(format!("Gagal memuat pustaka: {:?}", names))
}

impl X11GlxLib {
    fn load() -> Result<Self, String> {
        unsafe {
            let x11 = open_lib(&["libX11.so.6", "libX11.so"])?;
            let gl = open_lib(&["libGL.so.1", "libGL.so"])?;

            let gl_fns = GlFns {
                viewport: load_sym(gl, "glViewport")?,
                matrix_mode: load_sym(gl, "glMatrixMode")?,
                load_identity: load_sym(gl, "glLoadIdentity")?,
                frustum: load_sym(gl, "glFrustum")?,
                ortho: load_sym(gl, "glOrtho")?,
                clear_color: load_sym(gl, "glClearColor")?,
                clear: load_sym(gl, "glClear")?,
                enable: load_sym(gl, "glEnable")?,
                disable: load_sym(gl, "glDisable")?,
                blend_func: load_sym(gl, "glBlendFunc")?,
                gen_textures: load_sym(gl, "glGenTextures")?,
                bind_texture: load_sym(gl, "glBindTexture")?,
                tex_image_2d: load_sym(gl, "glTexImage2D")?,
                tex_parameteri: load_sym(gl, "glTexParameteri")?,
                begin: load_sym(gl, "glBegin")?,
                end: load_sym(gl, "glEnd")?,
                tex_coord_2f: load_sym(gl, "glTexCoord2f")?,
                vertex_3f: load_sym(gl, "glVertex3f")?,
                vertex_2f: load_sym(gl, "glVertex2f")?,
                color_4f: load_sym(gl, "glColor4f")?,
                rotatef: load_sym(gl, "glRotatef")?,
                translatef: load_sym(gl, "glTranslatef")?,
                fogi: load_sym(gl, "glFogi")?,
                fogf: load_sym(gl, "glFogf")?,
                fogfv: load_sym(gl, "glFogfv")?,
                polygon_mode: load_sym(gl, "glPolygonMode")?,
            };

            Ok(Self {
                x_open_display: load_sym(x11, "XOpenDisplay")?,
                x_close_display: load_sym(x11, "XCloseDisplay")?,
                x_default_screen: load_sym(x11, "XDefaultScreen")?,
                x_root_window: load_sym(x11, "XRootWindow")?,
                x_create_colormap: load_sym(x11, "XCreateColormap")?,
                x_create_window: load_sym(x11, "XCreateWindow")?,
                x_map_window: load_sym(x11, "XMapWindow")?,
                x_destroy_window: load_sym(x11, "XDestroyWindow")?,
                x_resize_window: load_sym(x11, "XResizeWindow")?,
                x_store_name: load_sym(x11, "XStoreName")?,
                x_intern_atom: load_sym(x11, "XInternAtom")?,
                x_set_wm_protocols: load_sym(x11, "XSetWMProtocols")?,
                x_get_geometry: load_sym(x11, "XGetGeometry")?,
                x_pending: load_sym(x11, "XPending")?,
                x_next_event: load_sym(x11, "XNextEvent")?,
                x_flush: load_sym(x11, "XFlush")?,
                x_free: load_sym(x11, "XFree")?,
                x_set_error_handler: load_sym(x11, "XSetErrorHandler")?,
                x_lookup_keysym: load_sym(x11, "XLookupKeysym")?,
                glx_choose_visual: load_sym(gl, "glXChooseVisual")?,
                glx_create_context: load_sym(gl, "glXCreateContext")?,
                glx_destroy_context: load_sym(gl, "glXDestroyContext")?,
                glx_make_current: load_sym(gl, "glXMakeCurrent")?,
                glx_swap_buffers: load_sym(gl, "glXSwapBuffers")?,
                gl: gl_fns,
            })
        }
    }
}

pub fn run_renderer(args: &RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let lib = X11GlxLib::load()?;
    X_ERROR_OCCURRED.store(false, Ordering::SeqCst);

    unsafe {
        (lib.x_set_error_handler)(Some(on_x_error));
        let dpy = (lib.x_open_display)(ptr::null());
        if dpy.is_null() {
            return Err("Gagal membuka koneksi X11 Display untuk GLMatrix".into());
        }

        let screen = (lib.x_default_screen)(dpy);
        let root = (lib.x_root_window)(dpy, screen);

        let mut attrs = [
            GLX_RGBA,
            GLX_DOUBLEBUFFER,
            GLX_RED_SIZE,
            8,
            GLX_GREEN_SIZE,
            8,
            GLX_BLUE_SIZE,
            8,
            GLX_DEPTH_SIZE,
            16,
            0,
        ];
        let mut vi = (lib.glx_choose_visual)(dpy, screen, attrs.as_mut_ptr());
        if vi.is_null() {
            let mut fallback_attrs = [GLX_RGBA, GLX_DOUBLEBUFFER, 0];
            vi = (lib.glx_choose_visual)(dpy, screen, fallback_attrs.as_mut_ptr());
        }
        if vi.is_null() {
            (lib.x_close_display)(dpy);
            return Err("Tidak ditemukan visual OpenGL GLX yang kompatibel".into());
        }

        let mut win_w: c_uint = 800;
        let mut win_h: c_uint = 600;
        let mut own_window = false;
        let mut child_embed_window: Option<c_ulong> = None;

        let target_win: c_ulong = if let Some(parent_id) = args.window_id {
            let mut r_ret: c_ulong = 0;
            let mut x_ret: c_int = 0;
            let mut y_ret: c_int = 0;
            let mut bw_ret: c_uint = 0;
            let mut depth_ret: c_uint = 0;
            (lib.x_get_geometry)(
                dpy,
                parent_id as c_ulong,
                &mut r_ret,
                &mut x_ret,
                &mut y_ret,
                &mut win_w,
                &mut win_h,
                &mut bw_ret,
                &mut depth_ret,
            );

            let cmap = (lib.x_create_colormap)(dpy, root, (*vi).visual, ALLOC_NONE);
            let mut swa: XSetWindowAttributes = std::mem::zeroed();
            swa.background_pixel = 0;
            swa.border_pixel = 0;
            swa.colormap = cmap;
            swa.event_mask = 0;
            let cw = (lib.x_create_window)(
                dpy,
                parent_id as c_ulong,
                0,
                0,
                win_w.max(1),
                win_h.max(1),
                0,
                (*vi).depth,
                INPUT_OUTPUT,
                (*vi).visual,
                CW_BACK_PIXEL | CW_BORDER_PIXEL | CW_COLORMAP | CW_EVENT_MASK,
                &mut swa,
            );
            (lib.x_map_window)(dpy, cw);
            (lib.x_flush)(dpy);
            child_embed_window = Some(cw);
            cw
        } else {
            own_window = true;
            let cmap = (lib.x_create_colormap)(dpy, root, (*vi).visual, ALLOC_NONE);
            let mut swa: XSetWindowAttributes = std::mem::zeroed();
            swa.background_pixel = 0;
            swa.border_pixel = 0;
            swa.colormap = cmap;
            swa.event_mask = STRUCTURE_NOTIFY_MASK | KEY_PRESS_MASK;
            let w = (lib.x_create_window)(
                dpy,
                root,
                0,
                0,
                win_w,
                win_h,
                0,
                (*vi).depth,
                INPUT_OUTPUT,
                (*vi).visual,
                CW_BACK_PIXEL | CW_BORDER_PIXEL | CW_COLORMAP | CW_EVENT_MASK,
                &mut swa,
            );
            let title = CString::new("GLMatrix Preview").unwrap();
            (lib.x_store_name)(dpy, w, title.as_ptr());
            let wm_del = (lib.x_intern_atom)(dpy, CString::new("WM_DELETE_WINDOW").unwrap().as_ptr(), 0);
            let mut protos = [wm_del];
            (lib.x_set_wm_protocols)(dpy, w, protos.as_mut_ptr(), 1);
            (lib.x_map_window)(dpy, w);
            (lib.x_flush)(dpy);
            w
        };

        let ctx = (lib.glx_create_context)(dpy, vi, ptr::null_mut(), 1);
        (lib.x_free)(vi as *mut c_void);
        if ctx.is_null() {
            if let Some(cw) = child_embed_window {
                (lib.x_destroy_window)(dpy, cw);
            }
            if own_window {
                (lib.x_destroy_window)(dpy, target_win);
            }
            (lib.x_close_display)(dpy);
            return Err("Gagal membuat konteks OpenGL GLX".into());
        }

        if (lib.glx_make_current)(dpy, target_win, ctx) == 0 {
            (lib.glx_destroy_context)(dpy, ctx);
            (lib.x_close_display)(dpy);
            return Err("glXMakeCurrent gagal".into());
        }

        let wm_delete_atom =
            (lib.x_intern_atom)(dpy, CString::new("WM_DELETE_WINDOW").unwrap().as_ptr(), 0);

        let mut scene = MatrixScene::new(&lib.gl, args.clone());

        loop {
            if X_ERROR_OCCURRED.load(Ordering::SeqCst) {
                break;
            }

            while (lib.x_pending)(dpy) > 0 {
                let mut ev = XEvent { ev_type: 0 };
                (lib.x_next_event)(dpy, &mut ev);
                match ev.ev_type {
                    DESTROY_NOTIFY => {
                        return cleanup_and_exit(&lib, dpy, ctx, own_window, target_win, child_embed_window);
                    }
                    KEY_PRESS if own_window => {
                        let ks = (lib.x_lookup_keysym)(&mut ev, 0);
                        if ks == 0xFF1B || ks == b'q' as c_ulong {
                            return cleanup_and_exit(&lib, dpy, ctx, own_window, target_win, child_embed_window);
                        }
                    }
                    CLIENT_MESSAGE if own_window => {
                        let cm: XClientMessageEvent = std::mem::transmute_copy(&ev);
                        if cm.data_l[0] as c_ulong == wm_delete_atom {
                            return cleanup_and_exit(&lib, dpy, ctx, own_window, target_win, child_embed_window);
                        }
                    }
                    _ => {}
                }
            }

            let query_win = args.window_id.map(|w| w as c_ulong).unwrap_or(target_win);
            let mut r_ret: c_ulong = 0;
            let mut x_ret: c_int = 0;
            let mut y_ret: c_int = 0;
            let mut new_w: c_uint = win_w;
            let mut new_h: c_uint = win_h;
            let mut bw_ret: c_uint = 0;
            let mut depth_ret: c_uint = 0;
            if (lib.x_get_geometry)(
                dpy,
                query_win,
                &mut r_ret,
                &mut x_ret,
                &mut y_ret,
                &mut new_w,
                &mut new_h,
                &mut bw_ret,
                &mut depth_ret,
            ) == 0
                || X_ERROR_OCCURRED.load(Ordering::SeqCst)
            {
                break;
            }
            if new_w != win_w || new_h != win_h {
                win_w = new_w.max(1);
                win_h = new_h.max(1);
                if let Some(cw) = child_embed_window {
                    (lib.x_resize_window)(dpy, cw, win_w, win_h);
                }
            }

            scene.render_frame(&lib.gl, win_w, win_h);
            (lib.glx_swap_buffers)(dpy, target_win);

            if args.delay_us > 0 {
                std::thread::sleep(Duration::from_micros(args.delay_us as u64));
            }
        }

        cleanup_and_exit(&lib, dpy, ctx, own_window, target_win, child_embed_window)
    }
}

unsafe fn cleanup_and_exit(
    lib: &X11GlxLib,
    dpy: *mut c_void,
    ctx: *mut c_void,
    own_window: bool,
    target_win: c_ulong,
    child_embed_window: Option<c_ulong>,
) -> Result<(), Box<dyn std::error::Error>> {
    (lib.glx_make_current)(dpy, 0, ptr::null_mut());
    (lib.glx_destroy_context)(dpy, ctx);
    if let Some(cw) = child_embed_window {
        if !X_ERROR_OCCURRED.load(Ordering::SeqCst) {
            (lib.x_destroy_window)(dpy, cw);
        }
    }
    if own_window && !X_ERROR_OCCURRED.load(Ordering::SeqCst) {
        (lib.x_destroy_window)(dpy, target_win);
    }
    (lib.x_close_display)(dpy);
    Ok(())
}
