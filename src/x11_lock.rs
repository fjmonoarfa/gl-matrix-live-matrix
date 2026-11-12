use std::process::Child;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use crate::config::Config;
use crate::pam::PamClient;

pub fn run_lockscreen(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let pam = PamClient::new().map_err(|e| format!("Inisialisasi PAM gagal: {}", e))?;
    let username = std::env::var("USER").unwrap_or_else(|_| "fj".to_string());

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num].clone();

    // 1. Dapatkan pemetaan keyboard
    let setup = conn.setup();
    let min_kc = setup.min_keycode;
    let max_kc = setup.max_keycode;
    let kc_count = max_kc - min_kc + 1;
    let kb_mapping = conn.get_keyboard_mapping(min_kc, kc_count)?.reply()?;

    // 2. Buat jendela fullscreen penutup utama
    let lock_win = conn.generate_id()?;
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;

    let values = CreateWindowAux::new()
        .background_pixel(screen.black_pixel)
        .override_redirect(1)
        .event_mask(EventMask::KEY_PRESS | EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY);

    conn.create_window(
        screen.root_depth,
        lock_win,
        screen.root,
        0,
        0,
        width,
        height,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &values,
    )?;

    // Buat jendela dialog prompt di tengah layar
    let box_w = 460;
    let box_h = 160;
    let box_x = (width as i16 - box_w as i16) / 2;
    let box_y = (height as i16 - box_h as i16) / 2;

    let prompt_win = conn.generate_id()?;
    let prompt_values = CreateWindowAux::new()
        .background_pixel(screen.black_pixel)
        .border_pixel(0x00FF41) // Hijau matrix
        .override_redirect(1)
        .event_mask(EventMask::EXPOSURE);

    conn.create_window(
        screen.root_depth,
        prompt_win,
        lock_win,
        box_x,
        box_y,
        box_w,
        box_h,
        2, // Border tebal 2px
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &prompt_values,
    )?;

    // Font dan GC (Graphics Context)
    let font_id = conn.generate_id()?;
    let _ = conn.open_font(font_id, b"fixed");

    let gc_green = conn.generate_id()?;
    let _ = conn.create_gc(
        gc_green,
        prompt_win,
        &CreateGCAux::new()
            .foreground(0x00FF41)
            .background(screen.black_pixel)
            .font(font_id),
    );

    let gc_white = conn.generate_id()?;
    let _ = conn.create_gc(
        gc_white,
        prompt_win,
        &CreateGCAux::new()
            .foreground(0xFFFFFF)
            .background(screen.black_pixel)
            .font(font_id),
    );

    let gc_red = conn.generate_id()?;
    let _ = conn.create_gc(
        gc_red,
        prompt_win,
        &CreateGCAux::new()
            .foreground(0xFF3333)
            .background(screen.black_pixel)
            .font(font_id),
    );

    // Map jendela
    conn.map_window(lock_win)?;
    conn.map_window(prompt_win)?;
    conn.configure_window(
        lock_win,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )?;
    conn.flush()?;

    // 3. Ambil kontrol keyboard & mouse (Grab)
    let mut grabbed_kb = false;
    for _ in 0..50 {
        let reply = conn.grab_keyboard(
            false,
            lock_win,
            x11rb::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?.reply()?;
        if reply.status == GrabStatus::SUCCESS {
            grabbed_kb = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if !grabbed_kb {
        let _ = conn.destroy_window(prompt_win);
        let _ = conn.destroy_window(lock_win);
        let _ = conn.flush();
        return Err("Gagal mengambil kontrol keyboard (keamanan layar kunci)".into());
    }

    let _ = conn.grab_pointer(
        false,
        lock_win,
        EventMask::NO_EVENT,
        GrabMode::ASYNC,
        GrabMode::ASYNC,
        lock_win,
        x11rb::NONE,
        x11rb::CURRENT_TIME,
    )?;

    // 4. Jalankan glmatrix di belakang lock screen (renderer bawaan atau eksternal)
    let glmatrix_child: Option<Child> = cfg.spawn_glmatrix(&cfg.lockscreen.options, lock_win).ok();

    // 5. Loop input password & verifikasi PAM
    let mut password = String::new();
    let mut status_msg = "Ketik password akun lalu tekan Enter".to_string();
    let mut status_is_err = false;

    let redraw = |conn: &RustConnection, pw_len: usize, status: &str, is_err: bool| -> Result<(), Box<dyn std::error::Error>> {
        let _ = conn.clear_area(false, prompt_win, 0, 0, box_w, box_h);

        // Header
        let title = format!("[ {} ]", cfg.lockscreen.banner_text);
        let _ = conn.image_text8(prompt_win, gc_green, 25, 35, title.as_bytes());

        // Password prompt
        let prompt_lbl = format!("Password untuk {}:", username);
        let _ = conn.image_text8(prompt_win, gc_white, 25, 75, prompt_lbl.as_bytes());

        let bullets = if pw_len == 0 {
            "[ _ ]".to_string()
        } else {
            "* ".repeat(pw_len)
        };
        let _ = conn.image_text8(prompt_win, gc_green, 25, 100, bullets.as_bytes());

        // Status
        let gc_status = if is_err { gc_red } else { gc_green };
        let status_text = format!("Status: {}", status);
        let _ = conn.image_text8(prompt_win, gc_status, 25, 135, status_text.as_bytes());

        conn.flush()?;
        Ok(())
    };

    redraw(&conn, password.len(), &status_msg, status_is_err)?;

    loop {
        let event = conn.wait_for_event()?;
        match event {
            x11rb::protocol::Event::Expose(_) => {
                redraw(&conn, password.len(), &status_msg, status_is_err)?;
            }
            x11rb::protocol::Event::KeyPress(kp) => {
                let kc = kp.detail;
                if kc < min_kc || kc > max_kc {
                    continue;
                }
                let per_kc = kb_mapping.keysyms_per_keycode as usize;
                let idx = (kc - min_kc) as usize * per_kc;
                let is_shift = (u16::from(kp.state) & u16::from(KeyButMask::SHIFT)) != 0;

                let keysym = if is_shift && per_kc > 1 {
                    kb_mapping.keysyms[idx + 1]
                } else {
                    kb_mapping.keysyms[idx]
                };

                // Enter / Return
                if keysym == 0xff0d || keysym == 0xff8d {
                    if password.is_empty() {
                        continue;
                    }
                    status_msg = "Memverifikasi kata sandi...".to_string();
                    status_is_err = false;
                    redraw(&conn, password.len(), &status_msg, status_is_err)?;

                    let pam_svc = cfg.general.get_pam_service();
                    match pam.verify_password(&pam_svc, &username, &password) {
                        Ok(()) => {
                            status_msg = "AKSES DITERIMA - Membuka kunci...".to_string();
                            status_is_err = false;
                            redraw(&conn, password.len(), &status_msg, status_is_err)?;
                            std::thread::sleep(Duration::from_millis(300));
                            break;
                        }
                        Err(err) => {
                            password.clear();
                            status_msg = format!("Akses ditolak: {}", err);
                            status_is_err = true;
                            redraw(&conn, password.len(), &status_msg, status_is_err)?;
                        }
                    }
                } else if keysym == 0xff08 {
                    // Backspace
                    password.pop();
                    status_msg = "Ketik password akun lalu tekan Enter".to_string();
                    status_is_err = false;
                    redraw(&conn, password.len(), &status_msg, status_is_err)?;
                } else if keysym == 0xff1b {
                    // Escape
                    password.clear();
                    status_msg = "Input dihapus".to_string();
                    status_is_err = false;
                    redraw(&conn, password.len(), &status_msg, status_is_err)?;
                } else if (0x20..=0x7e).contains(&keysym) {
                    // ASCII karakter
                    if let Some(ch) = char::from_u32(keysym) {
                        password.push(ch);
                        status_msg = "Ketik password akun lalu tekan Enter".to_string();
                        status_is_err = false;
                        redraw(&conn, password.len(), &status_msg, status_is_err)?;
                    }
                }
            }
            _ => {}
        }
    }

    // Bersihkan sebelum keluar
    if let Some(mut child) = glmatrix_child {
        let _ = child.kill();
        let _ = child.wait();
    }

    let _ = conn.ungrab_keyboard(x11rb::CURRENT_TIME);
    let _ = conn.ungrab_pointer(x11rb::CURRENT_TIME);
    let _ = conn.destroy_window(prompt_win);
    let _ = conn.destroy_window(lock_win);
    let _ = conn.flush();

    Ok(())
}
