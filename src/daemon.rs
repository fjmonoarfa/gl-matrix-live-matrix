use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::screensaver;
use x11rb::protocol::xproto::ConnectionExt as _;
use crate::config::Config;
use crate::x11_desktop::{start_desktop, DesktopWallpaper};
use crate::x11_lock::run_lockscreen;

fn stop_desktop_session(
    desktop_session: &mut Option<(x11rb::rust_connection::RustConnection, DesktopWallpaper)>,
) {
    if let Some((conn, mut sess)) = desktop_session.take() {
        let _ = sess.child.kill();
        let _ = sess.child.wait();
        let _ = conn.destroy_window(sess.window);
        let _ = conn.flush();
    }
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

pub fn get_socket_path() -> PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(base).join("glmatrix-live.sock")
}

pub fn send_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    let sock_path = get_socket_path();
    let mut stream = UnixStream::connect(sock_path)?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;
    Ok(resp)
}

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let sock_path = get_socket_path();
    if sock_path.exists() {
        if let Ok(mut stream) = UnixStream::connect(&sock_path) {
            let _ = stream.write_all(b"status\n");
            let mut resp = String::new();
            if stream.read_to_string(&mut resp).is_ok() && !resp.is_empty() {
                println!("glmatrix-live daemon sudah berjalan.");
                return Ok(());
            }
        }
        let _ = std::fs::remove_file(&sock_path);
    }

    let listener = UnixListener::bind(&sock_path)?;
    listener.set_nonblocking(true)?;

    println!("glmatrix-live daemon aktif. Socket: {:?}", sock_path);

    let mut cfg = Config::load_or_create();
    let mut desktop_session: Option<(x11rb::rust_connection::RustConnection, DesktopWallpaper)> = None;

    if cfg.general.desktop_enabled {
        match start_desktop(&cfg) {
            Ok(sess) => {
                println!("✓ Desktop live background dimulai (PID child: {})", sess.1.child.id());
                desktop_session = Some(sess);
            }
            Err(e) => {
                eprintln!("✗ Gagal memulai desktop live background: {}", e);
            }
        }
    } else {
        println!("• Desktop live wallpaper saat ini nonaktif (desktop_enabled = false).");
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Handler sinyal SIGINT / SIGTERM
    ctrlc_handler(r);

    let mut last_idle_check = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        // 1. Cek perintah dari socket
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 512];
                if let Ok(n) = stream.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    match cmd.as_str() {
                        "lock" => {
                            cfg = Config::load_or_create();
                            let _ = stream.write_all(b"OK: Mengunci layar...\n");
                            println!("Perintah lock diterima. Menjalankan Matrix lockscreen...");
                            let _ = run_lockscreen(&cfg);
                            println!("Layar dibuka kembali.");
                        }
                        "status" => {
                            let bg_pid = desktop_session
                                .as_mut()
                                .and_then(|(_, ref mut d)| {
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
                            let bg_pid = desktop_session
                                .as_mut()
                                .and_then(|(_, ref mut d)| {
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

                            let already_alive = desktop_session
                                .as_mut()
                                .and_then(|(_, ref mut d)| d.child.try_wait().ok())
                                .map_or(false, |status| status.is_none());

                            if !already_alive {
                                stop_desktop_session(&mut desktop_session);
                                match start_desktop(&cfg) {
                                    Ok(new_sess) => {
                                        println!("✓ Live wallpaper diaktifkan (PID: {}).", new_sess.1.child.id());
                                        desktop_session = Some(new_sess);
                                        let _ = stream.write_all(b"OK: Live wallpaper diaktifkan.\n");
                                    }
                                    Err(e) => {
                                        let msg = format!("ERR: Gagal menjalankan live wallpaper: {}\n", e);
                                        let _ = stream.write_all(msg.as_bytes());
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
                            stop_desktop_session(&mut desktop_session);
                            println!("• Live wallpaper dinonaktifkan.");
                            let _ = stream.write_all(b"OK: Live wallpaper dinonaktifkan.\n");
                        }
                        "wallpaper-toggle" => {
                            cfg = Config::load_or_create();
                            cfg.general.desktop_enabled = !cfg.general.desktop_enabled;
                            let _ = cfg.save();
                            if cfg.general.desktop_enabled {
                                stop_desktop_session(&mut desktop_session);
                                match start_desktop(&cfg) {
                                    Ok(new_sess) => {
                                        println!("✓ Live wallpaper diaktifkan (PID: {}).", new_sess.1.child.id());
                                        desktop_session = Some(new_sess);
                                        let _ = stream.write_all(b"OK: Live wallpaper diaktifkan.\n");
                                    }
                                    Err(e) => {
                                        let msg = format!("ERR: Gagal menjalankan live wallpaper: {}\n", e);
                                        let _ = stream.write_all(msg.as_bytes());
                                    }
                                }
                            } else {
                                stop_desktop_session(&mut desktop_session);
                                println!("• Live wallpaper dinonaktifkan.");
                                let _ = stream.write_all(b"OK: Live wallpaper dinonaktifkan.\n");
                            }
                        }
                        "restart" | "reload" => {
                            println!("Memuat ulang konfigurasi...");
                            cfg = Config::load_or_create();
                            stop_desktop_session(&mut desktop_session);

                            if cfg.general.desktop_enabled {
                                match start_desktop(&cfg) {
                                    Ok(new_sess) => {
                                        println!("✓ Desktop background berhasil di-restart (PID: {}).", new_sess.1.child.id());
                                        desktop_session = Some(new_sess);
                                        let _ = stream.write_all(b"OK: Konfigurasi dimuat ulang dan live wallpaper di-restart.\n");
                                    }
                                    Err(e) => {
                                        let msg = format!("ERR: Gagal me-restart wallpaper: {}\n", e);
                                        let _ = stream.write_all(msg.as_bytes());
                                    }
                                }
                            } else {
                                println!("• Live wallpaper dinonaktifkan sesuai konfigurasi.");
                                let _ = stream.write_all(b"OK: Konfigurasi dimuat ulang (live wallpaper nonaktif).\n");
                            }
                        }
                        "stop" => {
                            let _ = stream.write_all(b"OK: Menghentikan daemon...\n");
                            println!("Perintah stop diterima.");
                            running.store(false, Ordering::SeqCst);
                            break;
                        }
                        _ => {
                            let _ = stream.write_all(b"ERR: Perintah tidak dikenal\n");
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Tidak ada koneksi baru
            }
            Err(e) => {
                eprintln!("Error listener: {}", e);
            }
        }

        // 2. Cek auto-lock idle jika diaktifkan (> 0)
        if cfg.lockscreen.auto_lock_mins > 0 && last_idle_check.elapsed() >= Duration::from_secs(5) {
            last_idle_check = std::time::Instant::now();
            if let Ok((conn, s_num)) = x11rb::connect(None) {
                let screen = &conn.setup().roots[s_num];
                if let Ok(reply) = screensaver::query_info(&conn, screen.root) {
                    if let Ok(info) = reply.reply() {
                        let idle_ms = info.ms_since_user_input;
                        let threshold_ms = (cfg.lockscreen.auto_lock_mins as u32) * 60 * 1000;
                        if idle_ms >= threshold_ms {
                            println!("Waktu idle mencapai batas ({} detik). Mengunci layar...", idle_ms / 1000);
                            let _ = run_lockscreen(&cfg);
                        }
                    }
                }
            }
        }

        // 3. Pastikan proses background tetap hidup jika crash (saat desktop_enabled = true)
        if cfg.general.desktop_enabled {
            let should_restart = match desktop_session {
                Some((_, ref mut sess)) => match sess.child.try_wait() {
                    Ok(Some(_exit_status)) => true,
                    _ => false,
                },
                None => false,
            };

            if should_restart {
                println!("glmatrix child terhenti, me-restart desktop session...");
                stop_desktop_session(&mut desktop_session);
                match start_desktop(&cfg) {
                    Ok(new_sess) => {
                        println!("✓ Desktop background berhasil direstart (PID: {})", new_sess.1.child.id());
                        desktop_session = Some(new_sess);
                    }
                    Err(e) => {
                        eprintln!("✗ Gagal restart desktop background: {}", e);
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    // Bersihkan saat shutdown
    stop_desktop_session(&mut desktop_session);
    let _ = std::fs::remove_file(&sock_path);
    println!("glmatrix-live daemon telah dimatikan dengan aman.");

    Ok(())
}

fn ctrlc_handler(running: Arc<AtomicBool>) {
    let r = running.clone();
    let _ = std::thread::spawn(move || {
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGINT);
            libc::sigaddset(&mut set, libc::SIGTERM);
            let mut sig: libc::c_int = 0;
            libc::sigwait(&set, &mut sig);
            r.store(false, Ordering::SeqCst);
        }
    });
}
