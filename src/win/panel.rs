use crate::config::Config;
use crate::win::daemon::{lock_windows_screen, send_command};
use crate::win::ffi::*;
use std::process::{Child, Command};
use std::ptr;
use std::sync::Mutex;

const ID_CHK_WALLPAPER: isize = 101;
const ID_BTN_DAEMON: isize = 102;
const ID_BTN_LOCK: isize = 103;
const ID_TRK_SPEED: isize = 104;
const ID_TRK_DENSITY: isize = 105;
const ID_TRK_DELAY: isize = 106;
const ID_CMB_MODE: isize = 107;
const ID_CHK_FOG: isize = 108;
const ID_CHK_WAVES: isize = 109;
const ID_CHK_ROTATE: isize = 110;
const ID_CHK_TEXTURE: isize = 111;
const ID_CHK_WIREFRAME: isize = 112;
const ID_CHK_CLOCK: isize = 113;
const ID_CHK_FPS: isize = 114;
const ID_CHK_MONO: isize = 115;
const ID_EDT_AUTOLOCK: isize = 116;
const ID_CHK_AUTOSTART: isize = 117;
const ID_BTN_PREVIEW: isize = 118;
const ID_BTN_SCREENSAVER: isize = 119;
const ID_BTN_APPLY: isize = 120;
const ID_BTN_RESET: isize = 121;
const ID_BTN_CLOSE: isize = 122;

const TIMER_STATUS_ID: usize = 1;
const MODES: [&str; 5] = ["matrix", "binary", "hexadecimal", "decimal", "dna"];
const MODE_LABELS: [&str; 5] = [
    "Matrix (Katakana / Default)",
    "Biner (0 & 1)",
    "Heksadesimal (0-F)",
    "Desimal (0-9)",
    "DNA (G, A, T, C)",
];

struct PanelState {
    hwnd: HWND,
    lbl_status: HWND,
    btn_daemon: HWND,
    chk_wallpaper: HWND,
    trk_speed: HWND,
    lbl_speed_val: HWND,
    trk_density: HWND,
    lbl_density_val: HWND,
    trk_delay: HWND,
    lbl_delay_val: HWND,
    cmb_mode: HWND,
    chk_fog: HWND,
    chk_waves: HWND,
    chk_rotate: HWND,
    chk_texture: HWND,
    chk_wireframe: HWND,
    chk_clock: HWND,
    chk_fps: HWND,
    chk_mono: HWND,
    edt_autolock: HWND,
    chk_autostart: HWND,
    lbl_feedback: HWND,
    preview_box: HWND,
    preview_child: Option<Child>,
    popout_child: Option<Child>,
}

static PANEL_STATE: Mutex<Option<PanelState>> = Mutex::new(None);

fn make_lparam_range(min: u16, max: u16) -> LPARAM {
    ((max as isize) << 16) | (min as isize)
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    let w = to_wide(text);
    SetWindowTextW(hwnd, w.as_ptr());
}

unsafe fn get_text(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

unsafe fn is_checked(hwnd: HWND) -> bool {
    SendMessageW(hwnd, BM_GETCHECK, 0, 0) as usize == BST_CHECKED
}

unsafe fn set_checked(hwnd: HWND, checked: bool) {
    SendMessageW(
        hwnd,
        BM_SETCHECK,
        if checked { BST_CHECKED } else { BST_UNCHECKED },
        0,
    );
}

fn is_win_autostart_enabled() -> bool {
    unsafe {
        let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = to_wide("GLMatrixLive");
        let mut hkey: HKEY = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey) != 0 {
            return false;
        }
        let mut typ = 0u32;
        let mut size = 0u32;
        let rc = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            ptr::null_mut(),
            &mut typ,
            ptr::null_mut(),
            &mut size,
        );
        RegCloseKey(hkey);
        rc == 0
    }
}

fn set_win_autostart(enabled: bool) {
    unsafe {
        let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = to_wide("GLMatrixLive");
        let mut hkey: HKEY = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_SET_VALUE, &mut hkey) != 0 {
            return;
        }
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = format!("\"{}\" daemon", exe.to_string_lossy());
                let wcmd = to_wide(&cmd);
                let byte_len = (wcmd.len() * 2) as u32;
                RegSetValueExW(
                    hkey,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    wcmd.as_ptr() as *const u8,
                    byte_len,
                );
            }
        } else {
            RegDeleteValueW(hkey, name.as_ptr());
        }
        RegCloseKey(hkey);
    }
}

unsafe fn create_ctrl(
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: HWND,
    id: isize,
    hinst: HINSTANCE,
    font: HFONT,
) -> HWND {
    let wclass = to_wide(class);
    let wtext = to_wide(text);
    let hwnd = CreateWindowExW(
        0,
        wclass.as_ptr(),
        wtext.as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        parent,
        id,
        hinst,
        ptr::null_mut(),
    );
    if hwnd != 0 && font != 0 {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    }
    hwnd
}

fn load_cfg_to_ui(st: &PanelState, cfg: &Config) {
    unsafe {
        set_checked(st.chk_wallpaper, cfg.general.desktop_enabled);
        let speed_pos = (cfg.desktop.speed * 100.0).round().clamp(10.0, 800.0) as isize;
        SendMessageW(st.trk_speed, TBM_SETPOS, 1, speed_pos);
        SendMessageW(
            st.trk_density,
            TBM_SETPOS,
            1,
            cfg.desktop.density.min(100) as isize,
        );
        let delay_k = (cfg.desktop.delay / 1000).min(100) as isize;
        SendMessageW(st.trk_delay, TBM_SETPOS, 1, delay_k);

        let mode_idx = MODES
            .iter()
            .position(|&m| m == cfg.desktop.effective_mode())
            .unwrap_or(0);
        SendMessageW(st.cmb_mode, CB_SETCURSEL, mode_idx, 0);

        set_checked(st.chk_fog, !cfg.desktop.no_fog);
        set_checked(st.chk_waves, !cfg.desktop.no_waves);
        set_checked(st.chk_rotate, !cfg.desktop.no_rotate);
        set_checked(st.chk_texture, !cfg.desktop.no_texture);
        set_checked(st.chk_wireframe, cfg.desktop.wireframe);
        set_checked(st.chk_clock, cfg.desktop.clock);
        set_checked(st.chk_fps, cfg.desktop.show_fps);
        set_checked(
            st.chk_mono,
            cfg.desktop.extra_args.iter().any(|a| a == "--mono"),
        );

        set_text(st.edt_autolock, &cfg.lockscreen.auto_lock_mins.to_string());
        set_checked(st.chk_autostart, is_win_autostart_enabled());
        update_slider_labels(st);
    }
}

unsafe fn update_slider_labels(st: &PanelState) {
    let spd = SendMessageW(st.trk_speed, TBM_GETPOS, 0, 0) as f32 / 100.0;
    let den = SendMessageW(st.trk_density, TBM_GETPOS, 0, 0);
    let dly_k = SendMessageW(st.trk_delay, TBM_GETPOS, 0, 0);
    set_text(st.lbl_speed_val, &format!("{:.2}x", spd));
    set_text(st.lbl_density_val, &format!("{}%", den));
    if dly_k <= 0 {
        set_text(st.lbl_delay_val, "0 us (Max)");
    } else {
        let fps = 1000.0 / (dly_k as f32);
        set_text(st.lbl_delay_val, &format!("{} ms (~{:.0} FPS)", dly_k, fps));
    }
}

fn collect_cfg_from_ui(st: &PanelState) -> Config {
    let mut cfg = Config::load_or_create();
    unsafe {
        cfg.general.desktop_enabled = is_checked(st.chk_wallpaper);
        cfg.desktop.speed = (SendMessageW(st.trk_speed, TBM_GETPOS, 0, 0) as f32 / 100.0).clamp(0.1, 8.0);
        cfg.desktop.density = SendMessageW(st.trk_density, TBM_GETPOS, 0, 0).clamp(0, 100) as u32;
        cfg.desktop.delay = (SendMessageW(st.trk_delay, TBM_GETPOS, 0, 0).clamp(0, 100) as u32) * 1000;

        let idx = SendMessageW(st.cmb_mode, CB_GETCURSEL, 0, 0);
        let mode = MODES.get(idx as usize).copied().unwrap_or("matrix");
        cfg.desktop.mode = mode.to_string();
        cfg.desktop.binary = mode == "binary";

        cfg.desktop.no_fog = !is_checked(st.chk_fog);
        cfg.desktop.no_waves = !is_checked(st.chk_waves);
        cfg.desktop.no_rotate = !is_checked(st.chk_rotate);
        cfg.desktop.no_texture = !is_checked(st.chk_texture);
        cfg.desktop.wireframe = is_checked(st.chk_wireframe);
        cfg.desktop.clock = is_checked(st.chk_clock);
        cfg.desktop.show_fps = is_checked(st.chk_fps);

        cfg.desktop.extra_args.retain(|a| a != "--mono");
        if is_checked(st.chk_mono) {
            cfg.desktop.extra_args.push("--mono".to_string());
        }

        if let Ok(mins) = get_text(st.edt_autolock).trim().parse::<u32>() {
            cfg.lockscreen.auto_lock_mins = mins;
        }
    }
    cfg
}

fn restart_embedded_preview(st: &mut PanelState) {
    if let Some(mut ch) = st.preview_child.take() {
        let _ = ch.kill();
        let _ = ch.wait();
    }
    let cfg = collect_cfg_from_ui(st);
    if let Ok(exe) = std::env::current_exe() {
        let args = cfg.desktop.to_args(st.preview_box as usize as u64);
        if let Ok(child) = Command::new(exe).arg("render").args(&args).spawn() {
            st.preview_child = Some(child);
        }
    }
}

fn refresh_daemon_status(st: &PanelState) {
    unsafe {
        match send_command("status") {
            Ok(resp) => {
                let active = resp.contains("AKTIF");
                set_text(
                    st.lbl_status,
                    if active {
                        "Status: Daemon AKTIF | Live Wallpaper MENYALA"
                    } else {
                        "Status: Daemon AKTIF | Live Wallpaper MATI"
                    },
                );
                set_text(st.btn_daemon, "Stop Daemon");
            }
            Err(_) => {
                set_text(st.lbl_status, "Status: Daemon NONAKTIF");
                set_text(st.btn_daemon, "Start Daemon");
            }
        }
    }
}

unsafe extern "system" fn panel_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            if wparam == TIMER_STATUS_ID {
                if let Ok(guard) = PANEL_STATE.lock() {
                    if let Some(ref st) = *guard {
                        refresh_daemon_status(st);
                    }
                }
            }
            0
        }
        WM_HSCROLL => {
            if let Ok(mut guard) = PANEL_STATE.lock() {
                if let Some(ref mut st) = *guard {
                    update_slider_labels(st);
                    restart_embedded_preview(st);
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as isize;
            let notif = ((wparam >> 16) & 0xFFFF) as u32;
            match id {
                ID_CHK_WALLPAPER => {
                    if let Ok(guard) = PANEL_STATE.lock() {
                        if let Some(ref st) = *guard {
                            let enabled = is_checked(st.chk_wallpaper);
                            let mut cfg = collect_cfg_from_ui(st);
                            cfg.general.desktop_enabled = enabled;
                            let _ = cfg.save();
                            let cmd = if enabled { "wallpaper-on" } else { "wallpaper-off" };
                            if send_command(cmd).is_err() && enabled {
                                if let Ok(exe) = std::env::current_exe() {
                                    let _ = Command::new(exe).arg("daemon").spawn();
                                }
                            }
                            set_text(
                                st.lbl_feedback,
                                if enabled {
                                    "Live wallpaper diaktifkan."
                                } else {
                                    "Live wallpaper dimatikan."
                                },
                            );
                            refresh_daemon_status(st);
                        }
                    }
                }
                ID_BTN_DAEMON => {
                    if let Ok(guard) = PANEL_STATE.lock() {
                        if let Some(ref st) = *guard {
                            if send_command("status").is_ok() {
                                let _ = send_command("stop");
                                set_text(st.lbl_feedback, "Daemon dihentikan.");
                            } else if let Ok(exe) = std::env::current_exe() {
                                let _ = Command::new(exe).arg("daemon").spawn();
                                set_text(st.lbl_feedback, "Daemon dijalankan.");
                            }
                            refresh_daemon_status(st);
                        }
                    }
                }
                ID_BTN_LOCK => {
                    lock_windows_screen();
                }
                ID_CMB_MODE if notif == 1 => {
                    // CBN_SELCHANGE = 1
                    if let Ok(mut guard) = PANEL_STATE.lock() {
                        if let Some(ref mut st) = *guard {
                            restart_embedded_preview(st);
                        }
                    }
                }
                ID_CHK_FOG | ID_CHK_WAVES | ID_CHK_ROTATE | ID_CHK_TEXTURE | ID_CHK_WIREFRAME
                | ID_CHK_CLOCK | ID_CHK_FPS | ID_CHK_MONO => {
                    if let Ok(mut guard) = PANEL_STATE.lock() {
                        if let Some(ref mut st) = *guard {
                            restart_embedded_preview(st);
                        }
                    }
                }
                ID_CHK_AUTOSTART => {
                    if let Ok(guard) = PANEL_STATE.lock() {
                        if let Some(ref st) = *guard {
                            let en = is_checked(st.chk_autostart);
                            set_win_autostart(en);
                            set_text(
                                st.lbl_feedback,
                                if en {
                                    "Autostart Windows diaktifkan."
                                } else {
                                    "Autostart Windows dinonaktifkan."
                                },
                            );
                        }
                    }
                }
                ID_BTN_PREVIEW => {
                    if let Ok(mut guard) = PANEL_STATE.lock() {
                        if let Some(ref mut st) = *guard {
                            if let Some(mut ch) = st.popout_child.take() {
                                let _ = ch.kill();
                                let _ = ch.wait();
                            }
                            let cfg = collect_cfg_from_ui(st);
                            if let Ok(exe) = std::env::current_exe() {
                                let mut args = cfg.desktop.to_args(0);
                                // Replace -window-id 0x0 with --window
                                if args.len() >= 2 {
                                    args.drain(0..2);
                                }
                                if let Ok(child) =
                                    Command::new(exe).arg("render").arg("--window").args(&args).spawn()
                                {
                                    st.popout_child = Some(child);
                                }
                            }
                        }
                    }
                }
                ID_BTN_SCREENSAVER => {
                    let _ = Command::new("rundll32.exe")
                        .args(["shell32.dll,Control_RunDLL", "desk.cpl,,1"])
                        .spawn();
                }
                ID_BTN_APPLY => {
                    if let Ok(guard) = PANEL_STATE.lock() {
                        if let Some(ref st) = *guard {
                            let cfg = collect_cfg_from_ui(st);
                            match cfg.save() {
                                Ok(()) => {
                                    if send_command("restart").is_err() && cfg.general.desktop_enabled {
                                        if let Ok(exe) = std::env::current_exe() {
                                            let _ = Command::new(exe).arg("daemon").spawn();
                                        }
                                    }
                                    set_text(st.lbl_feedback, "Konfigurasi disimpan & diterapkan.");
                                    refresh_daemon_status(st);
                                }
                                Err(e) => {
                                    set_text(st.lbl_feedback, &format!("Gagal simpan: {}", e));
                                }
                            }
                        }
                    }
                }
                ID_BTN_RESET => {
                    let msg = to_wide("Kembalikan semua pengaturan ke nilai default?");
                    let cap = to_wide("GLMatrix Live");
                    if MessageBoxW(hwnd, msg.as_ptr(), cap.as_ptr(), MB_YESNO | MB_ICONINFORMATION) == IDYES {
                        if let Ok(mut guard) = PANEL_STATE.lock() {
                            if let Some(ref mut st) = *guard {
                                load_cfg_to_ui(st, &Config::default());
                                restart_embedded_preview(st);
                                set_text(st.lbl_feedback, "Dikembalikan ke default (klik Terapkan).");
                            }
                        }
                    }
                }
                ID_BTN_CLOSE => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY | WM_CLOSE => {
            KillTimer(hwnd, TIMER_STATUS_ID);
            if let Ok(mut guard) = PANEL_STATE.lock() {
                if let Some(ref mut st) = *guard {
                    if let Some(mut ch) = st.preview_child.take() {
                        let _ = ch.kill();
                        let _ = ch.wait();
                    }
                    if let Some(mut ch) = st.popout_child.take() {
                        let _ = ch.kill();
                        let _ = ch.wait();
                    }
                }
                *guard = None;
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run_panel() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        SetProcessDPIAware();
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let hinst = GetModuleHandleW(ptr::null());
        let class_name = to_wide("GLMatrixLivePanelClass");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(panel_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: 0,
            hCursor: LoadCursorW(0, IDC_ARROW as *const u16),
            hbrBackground: COLOR_WINDOW + 1,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let title = to_wide("GLMatrix Live — Panel Konfigurasi Windows");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            780,
            520,
            0,
            0,
            hinst,
            ptr::null_mut(),
        );
        if hwnd == 0 {
            return Err("Gagal membuat jendela panel Windows".into());
        }

        let font = GetStockObject(DEFAULT_GUI_FONT);

        // Top controls
        let chk_wallpaper = create_ctrl(
            "BUTTON",
            "Live Wallpaper Aktif (On / Off)",
            BS_AUTOCHECKBOX | WS_TABSTOP,
            16,
            14,
            230,
            24,
            hwnd,
            ID_CHK_WALLPAPER,
            hinst,
            font,
        );
        let lbl_status = create_ctrl("STATIC", "Status: memeriksa...", 0, 16, 44, 410, 20, hwnd, 0, hinst, font);
        let btn_daemon = create_ctrl(
            "BUTTON",
            "Start Daemon",
            BS_PUSHBUTTON | WS_TABSTOP,
            460,
            14,
            135,
            28,
            hwnd,
            ID_BTN_DAEMON,
            hinst,
            font,
        );
        let _btn_lock = create_ctrl(
            "BUTTON",
            "Kunci Windows",
            BS_PUSHBUTTON | WS_TABSTOP,
            605,
            14,
            145,
            28,
            hwnd,
            ID_BTN_LOCK,
            hinst,
            font,
        );

        // Left column: settings
        let _grp = create_ctrl("BUTTON", "Pengaturan GLMatrix", BS_GROUPBOX, 16, 72, 430, 346, hwnd, 0, hinst, font);

        create_ctrl("STATIC", "Kecepatan Glyph:", 0, 30, 100, 120, 20, hwnd, 0, hinst, font);
        let trk_speed = create_ctrl("msctls_trackbar32", "", TBS_HORZ | TBS_AUTOTICKS | WS_TABSTOP, 150, 96, 210, 28, hwnd, ID_TRK_SPEED, hinst, font);
        SendMessageW(trk_speed, TBM_SETRANGE, 1, make_lparam_range(10, 800));
        let lbl_speed_val = create_ctrl("STATIC", "1.00x", 0, 365, 100, 70, 20, hwnd, 0, hinst, font);

        create_ctrl("STATIC", "Kepadatan (%):", 0, 30, 138, 120, 20, hwnd, 0, hinst, font);
        let trk_density = create_ctrl("msctls_trackbar32", "", TBS_HORZ | TBS_AUTOTICKS | WS_TABSTOP, 150, 134, 210, 28, hwnd, ID_TRK_DENSITY, hinst, font);
        SendMessageW(trk_density, TBM_SETRANGE, 1, make_lparam_range(0, 100));
        let lbl_density_val = create_ctrl("STATIC", "20%", 0, 365, 138, 70, 20, hwnd, 0, hinst, font);

        create_ctrl("STATIC", "Frame Delay:", 0, 30, 176, 120, 20, hwnd, 0, hinst, font);
        let trk_delay = create_ctrl("msctls_trackbar32", "", TBS_HORZ | TBS_AUTOTICKS | WS_TABSTOP, 150, 172, 210, 28, hwnd, ID_TRK_DELAY, hinst, font);
        SendMessageW(trk_delay, TBM_SETRANGE, 1, make_lparam_range(0, 100));
        let lbl_delay_val = create_ctrl("STATIC", "35 ms", 0, 365, 176, 75, 20, hwnd, 0, hinst, font);

        create_ctrl("STATIC", "Mode Karakter:", 0, 30, 216, 120, 20, hwnd, 0, hinst, font);
        let cmb_mode = create_ctrl("COMBOBOX", "", CBS_DROPDOWNLIST | WS_TABSTOP, 155, 212, 260, 140, hwnd, ID_CMB_MODE, hinst, font);
        for lbl in MODE_LABELS {
            let wlbl = to_wide(lbl);
            SendMessageW(cmb_mode, CB_ADDSTRING, 0, wlbl.as_ptr() as isize);
        }

        let chk_fog = create_ctrl("BUTTON", "Efek Kabut (Fog)", BS_AUTOCHECKBOX | WS_TABSTOP, 30, 250, 180, 22, hwnd, ID_CHK_FOG, hinst, font);
        let chk_waves = create_ctrl("BUTTON", "Gelombang Warna", BS_AUTOCHECKBOX | WS_TABSTOP, 230, 250, 190, 22, hwnd, ID_CHK_WAVES, hinst, font);
        let chk_rotate = create_ctrl("BUTTON", "Rotasi / Panning 3D", BS_AUTOCHECKBOX | WS_TABSTOP, 30, 276, 180, 22, hwnd, ID_CHK_ROTATE, hinst, font);
        let chk_texture = create_ctrl("BUTTON", "Tekstur Karakter", BS_AUTOCHECKBOX | WS_TABSTOP, 230, 276, 190, 22, hwnd, ID_CHK_TEXTURE, hinst, font);
        let chk_wireframe = create_ctrl("BUTTON", "Mode Wireframe", BS_AUTOCHECKBOX | WS_TABSTOP, 30, 302, 180, 22, hwnd, ID_CHK_WIREFRAME, hinst, font);
        let chk_clock = create_ctrl("BUTTON", "Tampilkan Jam", BS_AUTOCHECKBOX | WS_TABSTOP, 230, 302, 190, 22, hwnd, ID_CHK_CLOCK, hinst, font);
        let chk_fps = create_ctrl("BUTTON", "Tampilkan FPS", BS_AUTOCHECKBOX | WS_TABSTOP, 30, 328, 180, 22, hwnd, ID_CHK_FPS, hinst, font);
        let chk_mono = create_ctrl("BUTTON", "Mode Monokrom", BS_AUTOCHECKBOX | WS_TABSTOP, 230, 328, 190, 22, hwnd, ID_CHK_MONO, hinst, font);

        create_ctrl("STATIC", "Auto-Lock Idle (mnt):", 0, 30, 360, 130, 20, hwnd, 0, hinst, font);
        let edt_autolock = create_ctrl("EDIT", "5", WS_BORDER | ES_NUMBER | ES_AUTOHSCROLL | WS_TABSTOP, 165, 358, 60, 22, hwnd, ID_EDT_AUTOLOCK, hinst, font);
        let chk_autostart = create_ctrl("BUTTON", "Autostart saat Login", BS_AUTOCHECKBOX | WS_TABSTOP, 240, 358, 190, 22, hwnd, ID_CHK_AUTOSTART, hinst, font);

        let _btn_ss = create_ctrl("BUTTON", "Setting Screensaver Windows", BS_PUSHBUTTON | WS_TABSTOP, 30, 386, 200, 24, hwnd, ID_BTN_SCREENSAVER, hinst, font);

        // Right column: Live Preview
        create_ctrl("STATIC", "Live Preview GLMatrix:", 0, 460, 72, 290, 20, hwnd, 0, hinst, font);
        let preview_box = create_ctrl("STATIC", "", WS_BORDER, 460, 96, 290, 280, hwnd, 0, hinst, font);
        create_ctrl("BUTTON", "Preview Jendela Besar", BS_PUSHBUTTON | WS_TABSTOP, 460, 386, 290, 28, hwnd, ID_BTN_PREVIEW, hinst, font);

        // Bottom bar
        create_ctrl("BUTTON", "Reset Default", BS_PUSHBUTTON | WS_TABSTOP, 16, 434, 110, 30, hwnd, ID_BTN_RESET, hinst, font);
        let lbl_feedback = create_ctrl("STATIC", "", 0, 136, 440, 290, 20, hwnd, 0, hinst, font);
        create_ctrl("BUTTON", "Terapkan & Restart", BS_DEFPUSHBUTTON | WS_TABSTOP, 440, 434, 150, 30, hwnd, ID_BTN_APPLY, hinst, font);
        create_ctrl("BUTTON", "Tutup", BS_PUSHBUTTON | WS_TABSTOP, 600, 434, 150, 30, hwnd, ID_BTN_CLOSE, hinst, font);

        let cfg = Config::load_or_create();
        let mut state = PanelState {
            hwnd,
            lbl_status,
            btn_daemon,
            chk_wallpaper,
            trk_speed,
            lbl_speed_val,
            trk_density,
            lbl_density_val,
            trk_delay,
            lbl_delay_val,
            cmb_mode,
            chk_fog,
            chk_waves,
            chk_rotate,
            chk_texture,
            chk_wireframe,
            chk_clock,
            chk_fps,
            chk_mono,
            edt_autolock,
            chk_autostart,
            lbl_feedback,
            preview_box,
            preview_child: None,
            popout_child: None,
        };
        load_cfg_to_ui(&state, &cfg);
        refresh_daemon_status(&state);
        restart_embedded_preview(&mut state);

        if let Ok(mut guard) = PANEL_STATE.lock() {
            *guard = Some(state);
        }

        SetTimer(hwnd, TIMER_STATUS_ID, 1200, ptr::null_mut());
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
