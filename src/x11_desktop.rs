use std::process::Child;
use x11rb::connection::Connection;
use x11rb::protocol::shape;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use crate::config::Config;

pub struct DesktopWallpaper {
    #[allow(dead_code)]
    pub window: Window,
    pub child: Child,
}

pub fn start_desktop(cfg: &Config) -> Result<(RustConnection, DesktopWallpaper), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num].clone();

    let win = conn.generate_id()?;
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;

    let values = CreateWindowAux::new()
        .background_pixel(screen.black_pixel)
        .border_pixel(screen.black_pixel)
        .override_redirect(0)
        .event_mask(EventMask::STRUCTURE_NOTIFY);

    conn.create_window(
        screen.root_depth,
        win,
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

    // Atom definitions
    let atom_type = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE")?.reply()?.atom;
    let atom_type_desktop = conn.intern_atom(false, b"_NET_WM_WINDOW_TYPE_DESKTOP")?.reply()?.atom;
    let atom_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let atom_state_below = conn.intern_atom(false, b"_NET_WM_STATE_BELOW")?.reply()?.atom;
    let atom_state_sticky = conn.intern_atom(false, b"_NET_WM_STATE_STICKY")?.reply()?.atom;
    let atom_state_skip_taskbar = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR")?.reply()?.atom;
    let atom_state_skip_pager = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER")?.reply()?.atom;

    // Set _NET_WM_WINDOW_TYPE = DESKTOP
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atom_type,
        AtomEnum::ATOM,
        &[atom_type_desktop],
    )?;

    // Set _NET_WM_STATE = BELOW, STICKY, SKIP_TASKBAR, SKIP_PAGER
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atom_state,
        AtomEnum::ATOM,
        &[
            atom_state_below,
            atom_state_sticky,
            atom_state_skip_taskbar,
            atom_state_skip_pager,
        ],
    )?;

    // Transparent to mouse input
    let _ = shape::rectangles(
        &conn,
        shape::SO::SET,
        shape::SK::INPUT,
        ClipOrdering::UNSORTED,
        win,
        0,
        0,
        &[],
    );

    // Keep below all windows
    conn.configure_window(
        win,
        &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
    )?;

    conn.map_window(win)?;
    conn.flush()?;

    // Spawn glmatrix (renderer bawaan atau eksternal)
    let child = cfg.spawn_glmatrix(&cfg.desktop, win as u64)?;

    Ok((conn, DesktopWallpaper { window: win, child }))
}
