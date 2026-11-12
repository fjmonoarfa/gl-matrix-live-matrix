use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static X_ERROR_OCCURRED: AtomicBool = AtomicBool::new(false);

// X11 / GLX constants
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

// OpenGL constants
const GL_COLOR_BUFFER_BIT: c_uint = 0x0000_4000;
const GL_DEPTH_BUFFER_BIT: c_uint = 0x0000_0100;
const GL_PROJECTION: c_uint = 0x1701;
const GL_MODELVIEW: c_uint = 0x1700;
const GL_TEXTURE_2D: c_uint = 0x0DE1;
const GL_BLEND: c_uint = 0x0BE2;
const GL_DEPTH_TEST: c_uint = 0x0B71;
const GL_FOG: c_uint = 0x0B60;
const GL_FOG_MODE: c_uint = 0x0B65;
const GL_FOG_DENSITY: c_uint = 0x0B62;
const GL_FOG_COLOR: c_uint = 0x0B66;
const GL_EXP2: c_int = 0x0801;
const GL_SRC_ALPHA: c_uint = 0x0302;
const GL_ONE_MINUS_SRC_ALPHA: c_uint = 0x0303;
const GL_ONE: c_uint = 0x0001;
const GL_RGBA: c_int = 0x1908;
const GL_UNSIGNED_BYTE: c_uint = 0x1401;
const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
const GL_TEXTURE_WRAP_S: c_uint = 0x2802;
const GL_TEXTURE_WRAP_T: c_uint = 0x2803;
const GL_LINEAR: c_int = 0x2601;
const GL_CLAMP_TO_EDGE: c_int = 0x812F;
const GL_QUADS: c_uint = 0x0007;
const GL_FRONT_AND_BACK: c_uint = 0x0408;
const GL_LINE: c_uint = 0x1B01;
const GL_FILL: c_uint = 0x1B02;

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

struct X11GlLib {
    // X11
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

    // GLX
    glx_choose_visual: unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> *mut XVisualInfo,
    glx_create_context: unsafe extern "C" fn(*mut c_void, *mut XVisualInfo, *mut c_void, c_int) -> *mut c_void,
    glx_destroy_context: unsafe extern "C" fn(*mut c_void, *mut c_void),
    glx_make_current: unsafe extern "C" fn(*mut c_void, c_ulong, *mut c_void) -> c_int,
    glx_swap_buffers: unsafe extern "C" fn(*mut c_void, c_ulong),

    // OpenGL
    gl_viewport: unsafe extern "C" fn(c_int, c_int, c_int, c_int),
    gl_matrix_mode: unsafe extern "C" fn(c_uint),
    gl_load_identity: unsafe extern "C" fn(),
    gl_frustum: unsafe extern "C" fn(f64, f64, f64, f64, f64, f64),
    gl_ortho: unsafe extern "C" fn(f64, f64, f64, f64, f64, f64),
    gl_clear_color: unsafe extern "C" fn(f32, f32, f32, f32),
    gl_clear: unsafe extern "C" fn(c_uint),
    gl_enable: unsafe extern "C" fn(c_uint),
    gl_disable: unsafe extern "C" fn(c_uint),
    gl_blend_func: unsafe extern "C" fn(c_uint, c_uint),
    gl_gen_textures: unsafe extern "C" fn(c_int, *mut c_uint),
    gl_bind_texture: unsafe extern "C" fn(c_uint, c_uint),
    gl_tex_image_2d: unsafe extern "C" fn(c_uint, c_int, c_int, c_int, c_int, c_int, c_uint, c_uint, *const c_void),
    gl_tex_parameteri: unsafe extern "C" fn(c_uint, c_uint, c_int),
    gl_begin: unsafe extern "C" fn(c_uint),
    gl_end: unsafe extern "C" fn(),
    gl_tex_coord_2f: unsafe extern "C" fn(f32, f32),
    gl_vertex_3f: unsafe extern "C" fn(f32, f32, f32),
    gl_vertex_2f: unsafe extern "C" fn(f32, f32),
    gl_color_4f: unsafe extern "C" fn(f32, f32, f32, f32),
    gl_rotatef: unsafe extern "C" fn(f32, f32, f32, f32),
    gl_translatef: unsafe extern "C" fn(f32, f32, f32),
    gl_fogi: unsafe extern "C" fn(c_uint, c_int),
    gl_fogf: unsafe extern "C" fn(c_uint, f32),
    gl_fogfv: unsafe extern "C" fn(c_uint, *const f32),
    gl_polygon_mode: unsafe extern "C" fn(c_uint, c_uint),
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

impl X11GlLib {
    fn load() -> Result<Self, String> {
        unsafe {
            let x11 = open_lib(&["libX11.so.6", "libX11.so"])?;
            let gl = open_lib(&["libGL.so.1", "libGL.so"])?;

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

                gl_viewport: load_sym(gl, "glViewport")?,
                gl_matrix_mode: load_sym(gl, "glMatrixMode")?,
                gl_load_identity: load_sym(gl, "glLoadIdentity")?,
                gl_frustum: load_sym(gl, "glFrustum")?,
                gl_ortho: load_sym(gl, "glOrtho")?,
                gl_clear_color: load_sym(gl, "glClearColor")?,
                gl_clear: load_sym(gl, "glClear")?,
                gl_enable: load_sym(gl, "glEnable")?,
                gl_disable: load_sym(gl, "glDisable")?,
                gl_blend_func: load_sym(gl, "glBlendFunc")?,
                gl_gen_textures: load_sym(gl, "glGenTextures")?,
                gl_bind_texture: load_sym(gl, "glBindTexture")?,
                gl_tex_image_2d: load_sym(gl, "glTexImage2D")?,
                gl_tex_parameteri: load_sym(gl, "glTexParameteri")?,
                gl_begin: load_sym(gl, "glBegin")?,
                gl_end: load_sym(gl, "glEnd")?,
                gl_tex_coord_2f: load_sym(gl, "glTexCoord2f")?,
                gl_vertex_3f: load_sym(gl, "glVertex3f")?,
                gl_vertex_2f: load_sym(gl, "glVertex2f")?,
                gl_color_4f: load_sym(gl, "glColor4f")?,
                gl_rotatef: load_sym(gl, "glRotatef")?,
                gl_translatef: load_sym(gl, "glTranslatef")?,
                gl_fogi: load_sym(gl, "glFogi")?,
                gl_fogf: load_sym(gl, "glFogf")?,
                gl_fogfv: load_sym(gl, "glFogfv")?,
                gl_polygon_mode: load_sym(gl, "glPolygonMode")?,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderArgs {
    pub window_id: Option<u32>,
    pub delay_us: u32,
    pub speed: f32,
    pub density: u32,
    pub mode: String,
    pub no_fog: bool,
    pub no_waves: bool,
    pub no_rotate: bool,
    pub no_texture: bool,
    pub wireframe: bool,
    pub clock: bool,
    pub timefmt: String,
    pub show_fps: bool,
    pub mono: bool,
}

impl Default for RenderArgs {
    fn default() -> Self {
        Self {
            window_id: None,
            delay_us: 30000,
            speed: 1.0,
            density: 20,
            mode: "matrix".to_string(),
            no_fog: false,
            no_waves: false,
            no_rotate: false,
            no_texture: false,
            wireframe: false,
            clock: false,
            timefmt: String::new(),
            show_fps: false,
            mono: false,
        }
    }
}

fn parse_window_id(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u32>().ok()
    }
}

pub fn parse_render_args(args: &[String]) -> RenderArgs {
    let mut out = RenderArgs::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-window-id" | "--window-id" => {
                if i + 1 < args.len() {
                    out.window_id = parse_window_id(&args[i + 1]);
                    i += 2;
                    continue;
                }
            }
            "--delay" | "-delay" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u32>() {
                        out.delay_us = v.min(500_000);
                    }
                    i += 2;
                    continue;
                }
            }
            "--speed" | "-speed" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<f32>() {
                        out.speed = v.clamp(0.05, 16.0);
                    }
                    i += 2;
                    continue;
                }
            }
            "--density" | "-density" => {
                if i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<u32>() {
                        out.density = v.min(100);
                    }
                    i += 2;
                    continue;
                }
            }
            "--mode" | "-mode" => {
                if i + 1 < args.len() {
                    let m = args[i + 1].to_lowercase();
                    out.mode = if m == "hex" { "hexadecimal".to_string() } else { m };
                    i += 2;
                    continue;
                }
            }
            "--binary" | "-binary" => out.mode = "binary".to_string(),
            "--hexadecimal" | "-hexadecimal" | "--hex" | "-hex" => {
                out.mode = "hexadecimal".to_string();
            }
            "--decimal" | "-decimal" => out.mode = "decimal".to_string(),
            "--dna" | "-dna" => out.mode = "dna".to_string(),
            "--fog" | "-fog" => out.no_fog = false,
            "--no-fog" | "-no-fog" => out.no_fog = true,
            "--waves" | "-waves" => out.no_waves = false,
            "--no-waves" | "-no-waves" => out.no_waves = true,
            "--rotate" | "-rotate" => out.no_rotate = false,
            "--no-rotate" | "-no-rotate" => out.no_rotate = true,
            "--texture" | "-texture" => out.no_texture = false,
            "--no-texture" | "-no-texture" => out.no_texture = true,
            "--wireframe" | "-wireframe" => out.wireframe = true,
            "--no-wireframe" | "-no-wireframe" => out.wireframe = false,
            "--clock" | "-clock" => out.clock = true,
            "--no-clock" | "-no-clock" => out.clock = false,
            "--timefmt" | "-timefmt" => {
                if i + 1 < args.len() {
                    out.timefmt = args[i + 1].clone();
                    i += 2;
                    continue;
                }
            }
            "--fps" | "-fps" => out.show_fps = true,
            "--no-fps" | "-no-fps" => out.show_fps = false,
            "--mono" | "-mono" => out.mono = true,
            _ => {}
        }
        i += 1;
    }
    out
}

// Fast deterministic PRNG (xorshift64)
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
    fn range_usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u32() as usize) % n
        }
    }
}

// Glyph atlas layout: 16 cols x 8 rows = 128 cells of 32x32 RGBA = 512x256
const ATLAS_COLS: usize = 16;
const ATLAS_ROWS: usize = 8;
const CELL_SIZE: usize = 32;
const ATLAS_W: usize = ATLAS_COLS * CELL_SIZE;
const ATLAS_H: usize = ATLAS_ROWS * CELL_SIZE;

// 5x7 font bitmaps for 0-9, A-Z, punctuation (used both in atlas & FPS overlay)
fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        _ => [0; 7],
    }
}

fn draw_pixel_glow(buf: &mut [f32], x: i32, y: i32, strength: f32) {
    for dy in -2..=2 {
        for dx in -2..=2 {
            let px = x + dx;
            let py = y + dy;
            if px >= 0 && px < ATLAS_W as i32 && py >= 0 && py < ATLAS_H as i32 {
                let d2 = (dx * dx + dy * dy) as f32;
                let w = if d2 == 0.0 {
                    1.0
                } else if d2 <= 2.0 {
                    0.55
                } else {
                    0.18
                };
                let idx = (py as usize) * ATLAS_W + (px as usize);
                buf[idx] = (buf[idx] + strength * w).min(1.0);
            }
        }
    }
}

fn draw_line_cell(buf: &mut [f32], ox: usize, oy: usize, x0: f32, y0: f32, x1: f32, y1: f32) {
    let steps = 28;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let px = ox as f32 + (x0 + (x1 - x0) * t) * (CELL_SIZE as f32 - 8.0) + 4.0;
        let py = oy as f32 + (y0 + (y1 - y0) * t) * (CELL_SIZE as f32 - 8.0) + 4.0;
        draw_pixel_glow(buf, px.round() as i32, py.round() as i32, 0.9);
    }
}

fn draw_5x7_in_cell(buf: &mut [f32], cell_idx: usize, ch: char, mirror_x: bool) {
    let col = cell_idx % ATLAS_COLS;
    let row = cell_idx / ATLAS_COLS;
    let ox = col * CELL_SIZE;
    let oy = row * CELL_SIZE;
    let bm = glyph_5x7(ch);
    for (r, bits) in bm.iter().enumerate() {
        for c in 0..5 {
            let bit_idx = if mirror_x { c } else { 4 - c };
            if (bits >> bit_idx) & 1 == 1 {
                for sy in 0..3 {
                    for sx in 0..3 {
                        let px = (ox + 6 + c * 4 + sx) as i32;
                        let py = (oy + 4 + r * 3 + sy) as i32;
                        draw_pixel_glow(buf, px, py, 0.85);
                    }
                }
            }
        }
    }
}

fn build_glyph_atlas() -> Vec<u8> {
    let mut intensity = vec![0.0f32; ATLAS_W * ATLAS_H];

    // 0..9: digits '0'..'9'
    for d in 0..10 {
        let ch = char::from_digit(d as u32, 10).unwrap_or('0');
        draw_5x7_in_cell(&mut intensity, d, ch, false);
    }
    // 10..15: hex 'A'..'F'
    for (idx, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
        draw_5x7_in_cell(&mut intensity, 10 + idx, *ch, false);
    }
    // 16..26: 'G', 'T', 'M', 'P', 'S', ':', '-', '/', '.', ' '
    for (idx, ch) in ['G', 'T', 'M', 'P', 'S', ':', '-', '/', '.', ' '].iter().enumerate() {
        draw_5x7_in_cell(&mut intensity, 16 + idx, *ch, false);
    }

    // 32..127: 96 procedural Matrix/Katakana-style crisp geometric glyphs (mirrored Matrix aesthetic)
    let mut rng = Rng::new(0xDEAD_BEEF_1337_CAFE);
    for cell in 32..128 {
        let col = cell % ATLAS_COLS;
        let row = cell / ATLAS_COLS;
        let ox = col * CELL_SIZE;
        let oy = row * CELL_SIZE;

        // If first 10 katakana slots, mix in mirrored digits like authentic glmatrix
        if cell < 42 {
            let ch = char::from_digit((cell - 32) as u32, 10).unwrap_or('0');
            draw_5x7_in_cell(&mut intensity, cell, ch, true);
            continue;
        }

        let strokes = 3 + rng.range_usize(4);
        let grid_pts = [0.15f32, 0.38, 0.62, 0.85];
        for _ in 0..strokes {
            let horizontal = rng.next_u32() % 3 != 0;
            let x0 = grid_pts[rng.range_usize(4)];
            let y0 = grid_pts[rng.range_usize(4)];
            let (x1, y1) = if horizontal {
                let nx = grid_pts[rng.range_usize(4)];
                (if (nx - x0).abs() < 0.1 { 0.85 } else { nx }, y0)
            } else {
                let ny = grid_pts[rng.range_usize(4)];
                (x0, if (ny - y0).abs() < 0.1 { 0.85 } else { ny })
            };
            draw_line_cell(&mut intensity, ox, oy, x0, y0, x1, y1);
        }
        // Occasional diagonal Katakana hook (like ノ, レ, ツ)
        if rng.next_u32() % 2 == 0 {
            draw_line_cell(&mut intensity, ox, oy, 0.78, 0.25, 0.25, 0.85);
        }
    }

    let mut rgba = Vec::with_capacity(ATLAS_W * ATLAS_H * 4);
    for &v in &intensity {
        let a = (v.clamp(0.0, 1.0) * 255.0) as u8;
        rgba.push(255);
        rgba.push(255);
        rgba.push(255);
        rgba.push(a);
    }
    rgba
}

fn atlas_uv(glyph_idx: usize) -> (f32, f32, f32, f32) {
    let idx = glyph_idx % (ATLAS_COLS * ATLAS_ROWS);
    let c = idx % ATLAS_COLS;
    let r = idx / ATLAS_COLS;
    let pad_u = 1.0 / (ATLAS_W as f32);
    let pad_v = 1.0 / (ATLAS_H as f32);
    let u0 = (c as f32 / ATLAS_COLS as f32) + pad_u;
    let u1 = ((c + 1) as f32 / ATLAS_COLS as f32) - pad_u;
    let v0 = (r as f32 / ATLAS_ROWS as f32) + pad_v;
    let v1 = ((r + 1) as f32 / ATLAS_ROWS as f32) - pad_v;
    (u0, v0, u1, v1)
}

fn char_to_glyph_idx(ch: char) -> usize {
    match ch.to_ascii_uppercase() {
        '0'..='9' => (ch as u8 - b'0') as usize,
        'A'..='F' => 10 + (ch.to_ascii_uppercase() as u8 - b'A') as usize,
        'G' => 16,
        'T' => 17,
        'M' => 18,
        'P' => 19,
        'S' => 20,
        ':' => 21,
        '-' => 22,
        '/' => 23,
        '.' => 24,
        _ => 25, // space
    }
}

fn random_glyph_for_mode(mode: &str, rng: &mut Rng) -> usize {
    match mode {
        "binary" => rng.range_usize(2), // '0' or '1'
        "hexadecimal" => rng.range_usize(16), // '0'..'F'
        "decimal" => rng.range_usize(10), // '0'..'9'
        "dna" => match rng.range_usize(4) {
            0 => 16, // G
            1 => 10, // A
            2 => 17, // T
            _ => 12, // C
        },
        _ => {
            if rng.next_u32() % 5 == 0 {
                rng.range_usize(10)
            } else {
                32 + rng.range_usize(96)
            }
        }
    }
}

struct Strip {
    x: f32,
    y: f32,
    z: f32,
    speed: f32,
    len: usize,
    glyphs: Vec<usize>,
    wave_phase: f32,
    is_clock: bool,
}

impl Strip {
    fn respawn(rng: &mut Rng, mode: &str, initial: bool, is_clock: bool) -> Self {
        let z = if is_clock {
            rng.range_f32(-14.0, -9.0)
        } else {
            rng.range_f32(-52.0, -6.0)
        };
        let x = if is_clock {
            rng.range_f32(-5.5, 5.5)
        } else {
            rng.range_f32(-28.0, 28.0)
        };
        let len = if is_clock { 12 } else { 10 + rng.range_usize(18) };
        let y = if initial {
            rng.range_f32(-18.0, 32.0)
        } else {
            rng.range_f32(18.0, 36.0)
        };
        let speed = rng.range_f32(3.5, 9.5);
        let glyphs = (0..len)
            .map(|_| random_glyph_for_mode(mode, rng))
            .collect();
        Self {
            x,
            y,
            z,
            speed,
            len,
            glyphs,
            wave_phase: rng.range_f32(0.0, std::f32::consts::TAU),
            is_clock,
        }
    }
}

fn current_clock_string(fmt: &str) -> String {
    let fmt_str = if fmt.trim().is_empty() { "%H:%M:%S" } else { fmt.trim() };
    let Ok(cfmt) = CString::new(fmt_str) else {
        return "00:00:00".to_string();
    };
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let mut buf = [0i8; 64];
        let n = libc::strftime(buf.as_mut_ptr() as *mut c_char, buf.len(), cfmt.as_ptr(), &tm);
        if n == 0 {
            return "00:00:00".to_string();
        }
        CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn draw_fps_overlay(lib: &X11GlLib, w: f64, h: f64, fps: f32) {
    (lib.gl_matrix_mode)(GL_PROJECTION);
    (lib.gl_load_identity)();
    (lib.gl_ortho)(0.0, w, h, 0.0, -1.0, 1.0);
    (lib.gl_matrix_mode)(GL_MODELVIEW);
    (lib.gl_load_identity)();
    (lib.gl_disable)(GL_TEXTURE_2D);
    (lib.gl_disable)(GL_FOG);
    (lib.gl_polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);

    let text = format!("FPS: {:.1}", fps);
    (lib.gl_color_4f)(0.0, 0.0, 0.0, 0.65);
    (lib.gl_begin)(GL_QUADS);
    (lib.gl_vertex_2f)(8.0, 8.0);
    (lib.gl_vertex_2f)(16.0 + text.len() as f32 * 12.0, 8.0);
    (lib.gl_vertex_2f)(16.0 + text.len() as f32 * 12.0, 28.0);
    (lib.gl_vertex_2f)(8.0, 28.0);
    (lib.gl_end)();

    (lib.gl_color_4f)(0.2, 1.0, 0.4, 0.95);
    let mut cx = 14.0f32;
    for ch in text.chars() {
        let bm = glyph_5x7(ch.to_ascii_uppercase());
        for (row, bits) in bm.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let x = cx + col as f32 * 2.0;
                    let y = 12.0 + row as f32 * 2.0;
                    (lib.gl_begin)(GL_QUADS);
                    (lib.gl_vertex_2f)(x, y);
                    (lib.gl_vertex_2f)(x + 2.0, y);
                    (lib.gl_vertex_2f)(x + 2.0, y + 2.0);
                    (lib.gl_vertex_2f)(x, y + 2.0);
                    (lib.gl_end)();
                }
            }
        }
        cx += 12.0;
    }
}

pub fn run_renderer(args: &RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let lib = X11GlLib::load()?;
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

            // Buat child GLX window agar visual/depth selalu cocok 100% dengan GLX
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

        // Upload glyph texture atlas
        let atlas = build_glyph_atlas();
        let mut tex_id: c_uint = 0;
        (lib.gl_gen_textures)(1, &mut tex_id);
        (lib.gl_bind_texture)(GL_TEXTURE_2D, tex_id);
        (lib.gl_tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        (lib.gl_tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        (lib.gl_tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        (lib.gl_tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        (lib.gl_tex_image_2d)(
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            ATLAS_W as c_int,
            ATLAS_H as c_int,
            0,
            GL_RGBA as c_uint,
            GL_UNSIGNED_BYTE,
            atlas.as_ptr() as *const c_void,
        );

        let wm_delete_atom = (lib.x_intern_atom)(dpy, CString::new("WM_DELETE_WINDOW").unwrap().as_ptr(), 0);

        let strip_count = ((args.density as usize) * 240 / 100).max(if args.density == 0 { 0 } else { 12 });
        let mut rng = Rng::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x1234_5678_9ABC_DEF0),
        );
        let mut strips: Vec<Strip> = (0..strip_count)
            .map(|idx| Strip::respawn(&mut rng, &args.mode, true, args.clock && idx == 0))
            .collect();

        let start_time = Instant::now();
        let mut last_frame = Instant::now();
        let mut fps_timer = Instant::now();
        let mut fps_frames = 0u32;
        let mut current_fps = 0.0f32;
        let mut spin_timer = 0.0f32;

        loop {
            if X_ERROR_OCCURRED.load(Ordering::SeqCst) {
                break;
            }

            // Process X11 events
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

            // Sync parent/window geometry
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

            let now = Instant::now();
            let dt = now.duration_since(last_frame).as_secs_f32().min(0.1);
            last_frame = now;
            let elapsed = now.duration_since(start_time).as_secs_f32();

            spin_timer += dt * args.speed;
            let mutate_glyphs = spin_timer >= 0.08;
            if mutate_glyphs {
                spin_timer = 0.0;
            }

            let clock_text = if args.clock {
                current_clock_string(&args.timefmt)
            } else {
                String::new()
            };

            for strip in &mut strips {
                strip.y -= strip.speed * args.speed * dt;
                if strip.y - (strip.len as f32) * 1.15 > 36.0 || strip.y < -28.0 {
                    let is_clk = strip.is_clock;
                    *strip = Strip::respawn(&mut rng, &args.mode, false, is_clk);
                } else if strip.is_clock && !clock_text.is_empty() {
                    let chars: Vec<char> = clock_text.chars().collect();
                    for (idx, g) in strip.glyphs.iter_mut().enumerate() {
                        if idx < chars.len() {
                            *g = char_to_glyph_idx(chars[idx]);
                        }
                    }
                } else if mutate_glyphs {
                    for g in &mut strip.glyphs {
                        if rng.next_u32() % 18 == 0 {
                            *g = random_glyph_for_mode(&args.mode, &mut rng);
                        }
                    }
                }
            }

            // Render frame
            (lib.gl_viewport)(0, 0, win_w as c_int, win_h as c_int);
            (lib.gl_clear_color)(0.0, 0.0, 0.0, 1.0);
            (lib.gl_clear)(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

            (lib.gl_matrix_mode)(GL_PROJECTION);
            (lib.gl_load_identity)();
            let aspect = (win_w.max(1) as f64) / (win_h.max(1) as f64);
            let near = 1.0f64;
            let far = 100.0f64;
            let top = near * (45.0f64.to_radians() * 0.5).tan();
            let right = top * aspect;
            (lib.gl_frustum)(-right, right, -top, top, near, far);

            (lib.gl_matrix_mode)(GL_MODELVIEW);
            (lib.gl_load_identity)();
            (lib.gl_disable)(GL_DEPTH_TEST);
            (lib.gl_enable)(GL_BLEND);
            (lib.gl_blend_func)(GL_SRC_ALPHA, GL_ONE);

            if args.wireframe {
                (lib.gl_polygon_mode)(GL_FRONT_AND_BACK, GL_LINE);
            } else {
                (lib.gl_polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);
            }

            if !args.no_fog {
                let fog_color = [0.0f32, 0.0, 0.0, 1.0];
                (lib.gl_enable)(GL_FOG);
                (lib.gl_fogi)(GL_FOG_MODE, GL_EXP2);
                (lib.gl_fogf)(GL_FOG_DENSITY, 0.028);
                (lib.gl_fogfv)(GL_FOG_COLOR, fog_color.as_ptr());
            } else {
                (lib.gl_disable)(GL_FOG);
            }

            if !args.no_rotate {
                (lib.gl_translatef)(0.0, 0.0, -24.0);
                (lib.gl_rotatef)((elapsed * 12.0).sin() * 14.0, 0.0, 1.0, 0.0);
                (lib.gl_rotatef)((elapsed * 8.0).cos() * 8.0, 1.0, 0.0, 0.0);
                (lib.gl_translatef)(0.0, 0.0, 24.0);
            }

            if !args.no_texture {
                (lib.gl_enable)(GL_TEXTURE_2D);
                (lib.gl_bind_texture)(GL_TEXTURE_2D, tex_id);
            } else {
                (lib.gl_disable)(GL_TEXTURE_2D);
            }

            let cell_w = 0.88f32;
            let cell_h = 1.08f32;
            (lib.gl_begin)(GL_QUADS);
            for strip in &strips {
                let wave_boost = if args.no_waves {
                    1.0
                } else {
                    0.65 + 0.35 * (elapsed * 2.4 + strip.wave_phase + strip.x * 0.2).sin()
                };

                for (idx, &glyph) in strip.glyphs.iter().enumerate() {
                    let gy = strip.y + (idx as f32) * cell_h;
                    let t = 1.0 - (idx as f32 / strip.len.max(1) as f32);
                    let alpha = (t * t * wave_boost).clamp(0.05, 1.0);

                    if args.mono {
                        if idx == 0 {
                            (lib.gl_color_4f)(1.0, 1.0, 1.0, 1.0);
                        } else {
                            (lib.gl_color_4f)(0.75, 0.82, 0.78, alpha);
                        }
                    } else if idx == 0 {
                        (lib.gl_color_4f)(0.85, 1.0, 0.88, 1.0);
                    } else if strip.is_clock {
                        (lib.gl_color_4f)(0.45, 1.0, 0.65, 0.95);
                    } else {
                        (lib.gl_color_4f)(0.05, 0.92 * wave_boost, 0.28, alpha);
                    }

                    let (u0, v0, u1, v1) = atlas_uv(glyph);
                    let x0 = strip.x - cell_w * 0.5;
                    let x1 = strip.x + cell_w * 0.5;
                    let y0 = gy;
                    let y1 = gy + cell_h * 0.95;

                    (lib.gl_tex_coord_2f)(u0, v1);
                    (lib.gl_vertex_3f)(x0, y0, strip.z);
                    (lib.gl_tex_coord_2f)(u1, v1);
                    (lib.gl_vertex_3f)(x1, y0, strip.z);
                    (lib.gl_tex_coord_2f)(u1, v0);
                    (lib.gl_vertex_3f)(x1, y1, strip.z);
                    (lib.gl_tex_coord_2f)(u0, v0);
                    (lib.gl_vertex_3f)(x0, y1, strip.z);
                }
            }
            (lib.gl_end)();

            fps_frames += 1;
            let fps_elapsed = fps_timer.elapsed().as_secs_f32();
            if fps_elapsed >= 0.5 {
                current_fps = fps_frames as f32 / fps_elapsed;
                fps_frames = 0;
                fps_timer = Instant::now();
            }

            if args.show_fps {
                (lib.gl_blend_func)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                draw_fps_overlay(&lib, win_w as f64, win_h as f64, current_fps);
            }

            (lib.glx_swap_buffers)(dpy, target_win);

            if args.delay_us > 0 {
                std::thread::sleep(Duration::from_micros(args.delay_us as u64));
            }
        }

        cleanup_and_exit(&lib, dpy, ctx, own_window, target_win, child_embed_window)
    }
}

unsafe fn cleanup_and_exit(
    lib: &X11GlLib,
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
