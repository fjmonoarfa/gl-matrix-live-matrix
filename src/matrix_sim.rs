use std::os::raw::{c_int, c_uint, c_void};
use std::time::Instant;

// OpenGL constants shared across GLX (Linux) and WGL (Windows)
pub const GL_COLOR_BUFFER_BIT: c_uint = 0x0000_4000;
pub const GL_DEPTH_BUFFER_BIT: c_uint = 0x0000_0100;
pub const GL_PROJECTION: c_uint = 0x1701;
pub const GL_MODELVIEW: c_uint = 0x1700;
pub const GL_TEXTURE_2D: c_uint = 0x0DE1;
pub const GL_BLEND: c_uint = 0x0BE2;
pub const GL_DEPTH_TEST: c_uint = 0x0B71;
pub const GL_FOG: c_uint = 0x0B60;
pub const GL_FOG_MODE: c_uint = 0x0B65;
pub const GL_FOG_DENSITY: c_uint = 0x0B62;
pub const GL_FOG_COLOR: c_uint = 0x0B66;
pub const GL_EXP2: c_int = 0x0801;
pub const GL_SRC_ALPHA: c_uint = 0x0302;
pub const GL_ONE_MINUS_SRC_ALPHA: c_uint = 0x0303;
pub const GL_ONE: c_uint = 0x0001;
pub const GL_RGBA: c_int = 0x1908;
pub const GL_UNSIGNED_BYTE: c_uint = 0x1401;
pub const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
pub const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
pub const GL_TEXTURE_WRAP_S: c_uint = 0x2802;
pub const GL_TEXTURE_WRAP_T: c_uint = 0x2803;
pub const GL_LINEAR: c_int = 0x2601;
pub const GL_CLAMP: c_int = 0x2900;
pub const GL_QUADS: c_uint = 0x0007;
pub const GL_FRONT_AND_BACK: c_uint = 0x0408;
pub const GL_LINE: c_uint = 0x1B01;
pub const GL_FILL: c_uint = 0x1B02;

#[derive(Clone, Copy)]
pub struct GlFns {
    pub viewport: unsafe extern "system" fn(c_int, c_int, c_int, c_int),
    pub matrix_mode: unsafe extern "system" fn(c_uint),
    pub load_identity: unsafe extern "system" fn(),
    pub frustum: unsafe extern "system" fn(f64, f64, f64, f64, f64, f64),
    pub ortho: unsafe extern "system" fn(f64, f64, f64, f64, f64, f64),
    pub clear_color: unsafe extern "system" fn(f32, f32, f32, f32),
    pub clear: unsafe extern "system" fn(c_uint),
    pub enable: unsafe extern "system" fn(c_uint),
    pub disable: unsafe extern "system" fn(c_uint),
    pub blend_func: unsafe extern "system" fn(c_uint, c_uint),
    pub gen_textures: unsafe extern "system" fn(c_int, *mut c_uint),
    pub bind_texture: unsafe extern "system" fn(c_uint, c_uint),
    pub tex_image_2d: unsafe extern "system" fn(
        c_uint,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        c_uint,
        c_uint,
        *const c_void,
    ),
    pub tex_parameteri: unsafe extern "system" fn(c_uint, c_uint, c_int),
    pub begin: unsafe extern "system" fn(c_uint),
    pub end: unsafe extern "system" fn(),
    pub tex_coord_2f: unsafe extern "system" fn(f32, f32),
    pub vertex_3f: unsafe extern "system" fn(f32, f32, f32),
    pub vertex_2f: unsafe extern "system" fn(f32, f32),
    pub color_4f: unsafe extern "system" fn(f32, f32, f32, f32),
    pub rotatef: unsafe extern "system" fn(f32, f32, f32, f32),
    pub translatef: unsafe extern "system" fn(f32, f32, f32),
    pub fogi: unsafe extern "system" fn(c_uint, c_int),
    pub fogf: unsafe extern "system" fn(c_uint, f32),
    pub fogfv: unsafe extern "system" fn(c_uint, *const f32),
    pub polygon_mode: unsafe extern "system" fn(c_uint, c_uint),
}

#[derive(Debug, Clone)]
pub struct RenderArgs {
    pub window_id: Option<u64>,
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
    pub screensaver: bool,
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
            screensaver: false,
        }
    }
}

pub fn parse_window_id(s: &str) -> Option<u64> {
    let trimmed = s.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
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
            "--screensaver" | "-screensaver" => out.screensaver = true,
            _ => {}
        }
        i += 1;
    }
    out
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FF_FFFF) as f32 / 16_777_216.0
    }
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
    pub fn range_usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u32() as usize) % n
        }
    }
}

const ATLAS_COLS: usize = 16;
const ATLAS_ROWS: usize = 8;
const CELL_SIZE: usize = 32;
pub const ATLAS_W: usize = ATLAS_COLS * CELL_SIZE;
pub const ATLAS_H: usize = ATLAS_ROWS * CELL_SIZE;

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

pub fn build_glyph_atlas() -> Vec<u8> {
    let mut intensity = vec![0.0f32; ATLAS_W * ATLAS_H];

    for d in 0..10 {
        let ch = char::from_digit(d as u32, 10).unwrap_or('0');
        draw_5x7_in_cell(&mut intensity, d, ch, false);
    }
    for (idx, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
        draw_5x7_in_cell(&mut intensity, 10 + idx, *ch, false);
    }
    for (idx, ch) in ['G', 'T', 'M', 'P', 'S', ':', '-', '/', '.', ' '].iter().enumerate() {
        draw_5x7_in_cell(&mut intensity, 16 + idx, *ch, false);
    }

    let mut rng = Rng::new(0xDEAD_BEEF_1337_CAFE);
    for cell in 32..128 {
        let col = cell % ATLAS_COLS;
        let row = cell / ATLAS_COLS;
        let ox = col * CELL_SIZE;
        let oy = row * CELL_SIZE;

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
        _ => 25,
    }
}

fn random_glyph_for_mode(mode: &str, rng: &mut Rng) -> usize {
    match mode {
        "binary" => rng.range_usize(2),
        "hexadecimal" => rng.range_usize(16),
        "decimal" => rng.range_usize(10),
        "dna" => match rng.range_usize(4) {
            0 => 16,
            1 => 10,
            2 => 17,
            _ => 12,
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

#[cfg(unix)]
fn current_clock_string(fmt: &str) -> String {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

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

#[cfg(windows)]
fn current_clock_string(fmt: &str) -> String {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(lpSystemTime: *mut SystemTime);
    }

    let mut st = SystemTime::default();
    unsafe { GetLocalTime(&mut st) };
    let t = fmt.trim();
    if t.is_empty() {
        format!("{:02}:{:02}:{:02}", st.w_hour, st.w_minute, st.w_second)
    } else {
        let hour12 = match st.w_hour % 12 {
            0 => 12,
            h => h,
        };
        let ampm = if st.w_hour < 12 { "AM" } else { "PM" };
        t.replace("%H", &format!("{:02}", st.w_hour))
            .replace("%I", &format!("{:02}", hour12))
            .replace("%l", &format!("{:2}", hour12))
            .replace("%M", &format!("{:02}", st.w_minute))
            .replace("%S", &format!("{:02}", st.w_second))
            .replace("%p", ampm)
    }
}

unsafe fn draw_fps_overlay(gl: &GlFns, w: f64, h: f64, fps: f32) {
    (gl.matrix_mode)(GL_PROJECTION);
    (gl.load_identity)();
    (gl.ortho)(0.0, w, h, 0.0, -1.0, 1.0);
    (gl.matrix_mode)(GL_MODELVIEW);
    (gl.load_identity)();
    (gl.disable)(GL_TEXTURE_2D);
    (gl.disable)(GL_FOG);
    (gl.polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);

    let text = format!("FPS: {:.1}", fps);
    (gl.color_4f)(0.0, 0.0, 0.0, 0.65);
    (gl.begin)(GL_QUADS);
    (gl.vertex_2f)(8.0, 8.0);
    (gl.vertex_2f)(16.0 + text.len() as f32 * 12.0, 8.0);
    (gl.vertex_2f)(16.0 + text.len() as f32 * 12.0, 28.0);
    (gl.vertex_2f)(8.0, 28.0);
    (gl.end)();

    (gl.color_4f)(0.2, 1.0, 0.4, 0.95);
    let mut cx = 14.0f32;
    for ch in text.chars() {
        let bm = glyph_5x7(ch.to_ascii_uppercase());
        for (row, bits) in bm.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let x = cx + col as f32 * 2.0;
                    let y = 12.0 + row as f32 * 2.0;
                    (gl.begin)(GL_QUADS);
                    (gl.vertex_2f)(x, y);
                    (gl.vertex_2f)(x + 2.0, y);
                    (gl.vertex_2f)(x + 2.0, y + 2.0);
                    (gl.vertex_2f)(x, y + 2.0);
                    (gl.end)();
                }
            }
        }
        cx += 12.0;
    }
}

pub struct MatrixScene {
    args: RenderArgs,
    strips: Vec<Strip>,
    rng: Rng,
    tex_id: c_uint,
    start_time: Instant,
    last_frame: Instant,
    fps_timer: Instant,
    fps_frames: u32,
    current_fps: f32,
    spin_timer: f32,
}

impl MatrixScene {
    pub unsafe fn new(gl: &GlFns, args: RenderArgs) -> Self {
        let atlas = build_glyph_atlas();
        let mut tex_id: c_uint = 0;
        (gl.gen_textures)(1, &mut tex_id);
        (gl.bind_texture)(GL_TEXTURE_2D, tex_id);
        (gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        (gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        (gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP);
        (gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP);
        (gl.tex_image_2d)(
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

        let strip_count = ((args.density as usize) * 240 / 100)
            .max(if args.density == 0 { 0 } else { 12 });
        let mut rng = Rng::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x1234_5678_9ABC_DEF0),
        );
        let strips: Vec<Strip> = (0..strip_count)
            .map(|idx| Strip::respawn(&mut rng, &args.mode, true, args.clock && idx == 0))
            .collect();

        let now = Instant::now();
        Self {
            args,
            strips,
            rng,
            tex_id,
            start_time: now,
            last_frame: now,
            fps_timer: now,
            fps_frames: 0,
            current_fps: 0.0,
            spin_timer: 0.0,
        }
    }

    pub unsafe fn render_frame(&mut self, gl: &GlFns, win_w: u32, win_h: u32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let elapsed = now.duration_since(self.start_time).as_secs_f32();

        self.spin_timer += dt * self.args.speed;
        let mutate_glyphs = self.spin_timer >= 0.08;
        if mutate_glyphs {
            self.spin_timer = 0.0;
        }

        let clock_text = if self.args.clock {
            current_clock_string(&self.args.timefmt)
        } else {
            String::new()
        };

        for strip in &mut self.strips {
            strip.y -= strip.speed * self.args.speed * dt;
            if strip.y - (strip.len as f32) * 1.15 > 36.0 || strip.y < -28.0 {
                let is_clk = strip.is_clock;
                *strip = Strip::respawn(&mut self.rng, &self.args.mode, false, is_clk);
            } else if strip.is_clock && !clock_text.is_empty() {
                let chars: Vec<char> = clock_text.chars().collect();
                for (idx, g) in strip.glyphs.iter_mut().enumerate() {
                    if idx < chars.len() {
                        *g = char_to_glyph_idx(chars[idx]);
                    }
                }
            } else if mutate_glyphs {
                for g in &mut strip.glyphs {
                    if self.rng.next_u32() % 18 == 0 {
                        *g = random_glyph_for_mode(&self.args.mode, &mut self.rng);
                    }
                }
            }
        }

        let w = win_w.max(1);
        let h = win_h.max(1);
        (gl.viewport)(0, 0, w as c_int, h as c_int);
        (gl.clear_color)(0.0, 0.0, 0.0, 1.0);
        (gl.clear)(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        (gl.matrix_mode)(GL_PROJECTION);
        (gl.load_identity)();
        let aspect = (w as f64) / (h as f64);
        let near = 1.0f64;
        let far = 100.0f64;
        let top = near * (45.0f64.to_radians() * 0.5).tan();
        let right = top * aspect;
        (gl.frustum)(-right, right, -top, top, near, far);

        (gl.matrix_mode)(GL_MODELVIEW);
        (gl.load_identity)();
        (gl.disable)(GL_DEPTH_TEST);
        (gl.enable)(GL_BLEND);
        (gl.blend_func)(GL_SRC_ALPHA, GL_ONE);

        if self.args.wireframe {
            (gl.polygon_mode)(GL_FRONT_AND_BACK, GL_LINE);
        } else {
            (gl.polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);
        }

        if !self.args.no_fog {
            let fog_color = [0.0f32, 0.0, 0.0, 1.0];
            (gl.enable)(GL_FOG);
            (gl.fogi)(GL_FOG_MODE, GL_EXP2);
            (gl.fogf)(GL_FOG_DENSITY, 0.028);
            (gl.fogfv)(GL_FOG_COLOR, fog_color.as_ptr());
        } else {
            (gl.disable)(GL_FOG);
        }

        if !self.args.no_rotate {
            (gl.translatef)(0.0, 0.0, -24.0);
            (gl.rotatef)((elapsed * 12.0).sin() * 14.0, 0.0, 1.0, 0.0);
            (gl.rotatef)((elapsed * 8.0).cos() * 8.0, 1.0, 0.0, 0.0);
            (gl.translatef)(0.0, 0.0, 24.0);
        }

        if !self.args.no_texture {
            (gl.enable)(GL_TEXTURE_2D);
            (gl.bind_texture)(GL_TEXTURE_2D, self.tex_id);
        } else {
            (gl.disable)(GL_TEXTURE_2D);
        }

        let cell_w = 0.88f32;
        let cell_h = 1.08f32;
        (gl.begin)(GL_QUADS);
        for strip in &self.strips {
            let wave_boost = if self.args.no_waves {
                1.0
            } else {
                0.65 + 0.35 * (elapsed * 2.4 + strip.wave_phase + strip.x * 0.2).sin()
            };

            for (idx, &glyph) in strip.glyphs.iter().enumerate() {
                let gy = strip.y + (idx as f32) * cell_h;
                let t = 1.0 - (idx as f32 / strip.len.max(1) as f32);
                let alpha = (t * t * wave_boost).clamp(0.05, 1.0);

                if self.args.mono {
                    if idx == 0 {
                        (gl.color_4f)(1.0, 1.0, 1.0, 1.0);
                    } else {
                        (gl.color_4f)(0.75, 0.82, 0.78, alpha);
                    }
                } else if idx == 0 {
                    (gl.color_4f)(0.85, 1.0, 0.88, 1.0);
                } else if strip.is_clock {
                    (gl.color_4f)(0.45, 1.0, 0.65, 0.95);
                } else {
                    (gl.color_4f)(0.05, 0.92 * wave_boost, 0.28, alpha);
                }

                let (u0, v0, u1, v1) = atlas_uv(glyph);
                let x0 = strip.x - cell_w * 0.5;
                let x1 = strip.x + cell_w * 0.5;
                let y0 = gy;
                let y1 = gy + cell_h * 0.95;

                (gl.tex_coord_2f)(u0, v1);
                (gl.vertex_3f)(x0, y0, strip.z);
                (gl.tex_coord_2f)(u1, v1);
                (gl.vertex_3f)(x1, y0, strip.z);
                (gl.tex_coord_2f)(u1, v0);
                (gl.vertex_3f)(x1, y1, strip.z);
                (gl.tex_coord_2f)(u0, v0);
                (gl.vertex_3f)(x0, y1, strip.z);
            }
        }
        (gl.end)();

        self.fps_frames += 1;
        let fps_elapsed = self.fps_timer.elapsed().as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.current_fps = self.fps_frames as f32 / fps_elapsed;
            self.fps_frames = 0;
            self.fps_timer = Instant::now();
        }

        if self.args.show_fps {
            (gl.blend_func)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            draw_fps_overlay(gl, w as f64, h as f64, self.current_fps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_render_args_handles_flags() {
        let raw = vec![
            "-window-id".to_string(),
            "0x1234abcd".to_string(),
            "--speed".to_string(),
            "2.5".to_string(),
            "--density".to_string(),
            "55".to_string(),
            "--dna".to_string(),
            "--no-fog".to_string(),
            "--wireframe".to_string(),
            "--clock".to_string(),
            "--fps".to_string(),
        ];
        let parsed = parse_render_args(&raw);
        assert_eq!(parsed.window_id, Some(0x1234_ABCD));
        assert!((parsed.speed - 2.5).abs() < 0.01);
        assert_eq!(parsed.density, 55);
        assert_eq!(parsed.mode, "dna");
        assert!(parsed.no_fog);
        assert!(parsed.wireframe);
        assert!(parsed.clock);
        assert!(parsed.show_fps);
    }
}
