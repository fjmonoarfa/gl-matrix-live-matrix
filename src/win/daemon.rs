use crate::config::Config;
use crate::win::desktop::{start_desktop, stop_desktop, DesktopWallpaper};
use crate::win::ffi::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CONTROL_ADDR: &str = "127.0.0.1:47123";

pub fn send_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect_timeout(&CONTROL_ADDR.parse()?, Duration::from_millis(800))?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;
    Ok(resp)
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn lock_windows_screen() {
    unsafe {
        LockWorkStation();
    }
}

fn windows_idle_ms() -> u32 {
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            GetTickCount().wrapping_sub(lii.dwTime)
        } else {
            0
        }
    }
}

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let mutex_name = to_wide("Local\\glmatrix-live-daemon");
    let mutex = unsafe { CreateMutexW(ptr::null_mut(), 1, mutex_name.as_ptr()) };
    if mutex != 0 && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(mutex) };
        println!("glmatrix-live daemon sudah berjalan.");
        return Ok(());
    }

    let listener = TcpListener::bind(CONTROL_ADDR)?;
    listener.set_nonblocking(true)?;
    println!("glmatrix-live Windows daemon aktif pada {}", CONTROL_ADDR);

    let mut cfg = Config::load_or_create();
    let mut desktop_session: Option<DesktopWallpaper> = None;

    if cfg.general.desktop_enabled {
        match start_desktop(&cfg) {
            Ok(sess) => {
                println!("Desktop live wallpaper aktif (PID: {})", sess.child.id());
                desktop_session = Some(sess);
            }
            Err(e) => {
                eprintln!("Gagal memulai desktop live wallpaper: {}", e);
            }
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let mut last_idle_check = Instant::now();
    let mut last_lock_trigger = Instant::now() - Duration::from_secs(120);

    while running.load(Ordering::SeqCst) {
        // Pump Win32 messages for WorkerW host window
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 512];
                if let Ok(n) = stream.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    match cmd.as_str() {
                        "lock" => {
                            let _ = stream.write_all(b"OK: Mengunci layar Windows...\n");
                            lock_windows_screen();
                        }
                        "status" => {
                            let bg_pid = desktop_session.as_mut().and_then(|d| {
                                d.child.try_wait().ok().flatten().is_none().then(|| d.child.id())
                            });
                            let msg = format!(
                                "glmatrix-live daemon: BERJALAN\nLive Wallpaper Enabled: {}\nDesktop Background: {}{}\nConfig: {:?}\n",
                                if cfg.general.desktop_enabled { "YA" } else { "TIDAK" },
                                if bg_pid.is_some() { "AKTIF" } else { "NONAKTIF" },
                                bg_pid.map(|p| format!(" (PID: {})", p)).unwrap_or_default(),
                                Config::config_path()
                            );
                            let _ = stream.write_all(msg.as_bytes());
                        }
                        "status-json" => {
                            let bg_pid = desktop_session.as_mut().and_then(|d| {
                                d.child.try_wait().ok().flatten().is_none().then(|| d.child.id())
                            });
                            let pid_json = bg_pid.map_or_else(|| "null".to_string(), |p| p.to_string());
                            let cfg_path = Config::config_path();
                            let cfg_str = cfg_path.to_string_lossy();
                            let json = format!(
                                "{{\"daemon_running\":true,\"desktop_enabled\":{},\"wallpaper_active\":{},\"wallpaper_pid\":{},\"config_path\":\"{}\"}}\n",
                                cfg.general.desktop_enabled,
                                bg_pid.is_some(),
                                pid_json,
                                json_escape(&cfg_str),
                            );
                            let _ = stream.write_all(json.as_bytes());
                        }
                        "wallpaper-on" => {
                            cfg = Config::load_or_create();
                            cfg.general.desktop_enabled = true;
                            let _ = cfg.save();

                            let alive = desktop_session
                                .as_mut()
                                .and_then(|d| d.child.try_wait().ok())
                                .map_or(false, |st| st.is_none());
                            if !alive {
                                stop_desktop(&mut desktop_session);
                                match start_desktop(&cfg) {
                                    Ok(sess) => {
                                        desktop_session = Some(sess);
                                        let _ = stream.write_all(b"OK: Live wallpaper diaktifkan.\n");
                                    }
                                    Err(e) => {
                                        let _ = stream.write_all(
                                            format!("ERR: Gagal menjalankan live wallpaper: {}\n", e).as_bytes(),
                                        );
                                    }
                                }
                            } else {
                                let _ = stream.write_all(b"OK: Live wallpaper sudah aktif.\n");
                            }
                        }
                        "wallpaper-off" => {
                            cfg = Config::load_or_create();
                            cfg.general.desktop_enabled = false;
                            let _ = cfg.save();
                            stop_desktop(&mut desktop_session);
                            let _ = stream.write_all(b"OK: Live wallpaper dinonaktifkan.\n");
                        }
                        "wallpaper-toggle" => {
                            cfg = Config::load_or_create();
                            cfg.general.desktop_enabled = !cfg.general.desktop_enabled;
                            let _ = cfg.save();
                            if cfg.general.desktop_enabled {
                                stop_desktop(&mut desktop_session);
                                match start_desktop(&cfg) {
                                    Ok(sess) => {
                                        desktop_session = Some(sess);
                                        let _ = stream.write_all(b"OK: Live wallpaper diaktifkan.\n");
                                    }
                                    Err(e) => {
                                        let _ = stream.write_all(
                                            format!("ERR: Gagal menjalankan live wallpaper: {}\n", e).as_bytes(),
                                        );
                                    }
                                }
                            } else {
                                stop_desktop(&mut desktop_session);
                                let _ = stream.write_all(b"OK: Live wallpaper dinonaktifkan.\n");
                            }
                        }
                        "restart" | "reload" => {
                            cfg = Config::load_or_create();
                            stop_desktop(&mut desktop_session);
                            if cfg.general.desktop_enabled {
                                match start_desktop(&cfg) {
                                    Ok(sess) => {
                                        desktop_session = Some(sess);
                                        let _ = stream.write_all(
                                            b"OK: Konfigurasi dimuat ulang dan live wallpaper di-restart.\n",
                                        );
                                    }
                                    Err(e) => {
                                        let _ = stream.write_all(
                                            format!("ERR: Gagal me-restart wallpaper: {}\n", e).as_bytes(),
                                        );
                                    }
                                }
                            } else {
                                let _ = stream.write_all(
                                    b"OK: Konfigurasi dimuat ulang (live wallpaper nonaktif).\n",
                                );
                            }
                        }
                        "stop" => {
                            let _ = stream.write_all(b"OK: Menghentikan daemon...\n");
                            running.store(false, Ordering::SeqCst);
                            break;
                        }
                        _ => {
                            let _ = stream.write_all(b"ERR: Perintah tidak dikenal\n");
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }

        if cfg.lockscreen.auto_lock_mins > 0
            && last_idle_check.elapsed() >= Duration::from_secs(5)
            && last_lock_trigger.elapsed() >= Duration::from_secs(60)
        {
            last_idle_check = Instant::now();
            let threshold_ms = cfg.lockscreen.auto_lock_mins * 60 * 1000;
            if windows_idle_ms() >= threshold_ms {
                last_lock_trigger = Instant::now();
                lock_windows_screen();
            }
        }

        if cfg.general.desktop_enabled {
            let crashed = desktop_session
                .as_mut()
                .and_then(|d| d.child.try_wait().ok())
                .is_some_and(|st| st.is_some());
            if crashed {
                stop_desktop(&mut desktop_session);
                if let Ok(sess) = start_desktop(&cfg) {
                    desktop_session = Some(sess);
                } else {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }

        std::thread::sleep(Duration::from_millis(60));
    }

    stop_desktop(&mut desktop_session);
    if mutex != 0 {
        unsafe { CloseHandle(mutex) };
    }
    Ok(())
}
