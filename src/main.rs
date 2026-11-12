mod config;
mod matrix_sim;

#[cfg(unix)]
mod daemon;
#[cfg(unix)]
mod gl_matrix;
#[cfg(unix)]
mod pam;
#[cfg(unix)]
mod x11_desktop;
#[cfg(unix)]
mod x11_lock;

#[cfg(windows)]
mod win;

use clap::{Parser, Subcommand};
use config::Config;
use std::process::Command;

#[cfg(unix)]
const PANEL_PY: &str = include_str!("panel.py");

#[derive(Parser)]
#[command(
    name = "glmatrix-live",
    about = "Native Desktop Live Wallpaper & Lockscreen Manager with Matrix GL effect",
    version = "1.0.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Jalankan daemon di latar belakang (Desktop background + Auto lockscreen)
    Daemon,
    /// Buka panel konfigurasi GUI (On/Off wallpaper, kecepatan, mode & efek glmatrix)
    #[command(visible_aliases = ["gui", "settings", "ui"])]
    Panel,
    /// Nyalakan live wallpaper
    On,
    /// Matikan live wallpaper (daemon tetap berjalan untuk lockscreen)
    Off,
    /// Toggle live wallpaper On/Off
    Toggle,
    /// Kunci layar sekarang
    Lock,
    /// Jalankan hanya desktop live wallpaper
    Desktop,
    /// Hentikan daemon dan wallpaper yang sedang berjalan
    Stop,
    /// Muat ulang konfigurasi dan restart wallpaper
    Restart,
    /// Tampilkan status glmatrix-live
    Status,
    /// Tampilkan lokasi file konfigurasi (gunakan --gui untuk membuka panel)
    Config {
        /// Buka panel konfigurasi GUI
        #[arg(long)]
        gui: bool,
    },
    /// Jalankan renderer OpenGL Matrix bawaan (dipakai internal oleh daemon/preview)
    Render {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[cfg(unix)]
fn launch_panel() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let uid = unsafe { libc::getuid() };
    let script_path = runtime_dir.join(format!("glmatrix-live-panel-{}.py", uid));
    std::fs::write(&script_path, PANEL_PY)?;

    let mut cmd = Command::new("python3");
    cmd.arg(&script_path);
    if let Ok(exe) = std::env::current_exe() {
        cmd.env("GLMATRIX_LIVE_BIN", exe);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Gagal menjalankan python3 untuk panel konfigurasi: {}", e))?;
    if !status.success() {
        return Err(format!("Panel konfigurasi keluar dengan status {}", status).into());
    }
    Ok(())
}

#[cfg(windows)]
fn launch_panel() -> Result<(), Box<dyn std::error::Error>> {
    win::panel::run_panel()
}

#[cfg(unix)]
fn platform_send_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    daemon::send_command(cmd)
}

#[cfg(windows)]
fn platform_send_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    win::daemon::send_command(cmd)
}

#[cfg(unix)]
fn platform_run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    daemon::run_daemon()
}

#[cfg(windows)]
fn platform_run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    win::daemon::run_daemon()
}

#[cfg(unix)]
fn platform_run_renderer(args: &matrix_sim::RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    gl_matrix::run_renderer(args)
}

#[cfg(windows)]
fn platform_run_renderer(args: &matrix_sim::RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    win::gl_render::run_renderer(args)
}

#[cfg(windows)]
fn handle_windows_scr_args() -> Option<Result<(), Box<dyn std::error::Error>>> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let first = raw_args.first()?.trim().to_ascii_lowercase();

    if first == "/s" || first == "-s" {
        let cfg = Config::load_or_create();
        let mut rargs = matrix_sim::parse_render_args(&cfg.lockscreen.options.to_args(0));
        rargs.window_id = None;
        rargs.screensaver = true;
        return Some(win::gl_render::run_renderer(&rargs));
    }

    if first == "/p" || first == "-p" || first.starts_with("/p:") || first.starts_with("-p:") {
        let hwnd_str = if let Some(rest) = first.strip_prefix("/p:").or_else(|| first.strip_prefix("-p:")) {
            rest.to_string()
        } else {
            raw_args.get(1).cloned().unwrap_or_default()
        };
        if let Some(wid) = matrix_sim::parse_window_id(&hwnd_str) {
            let cfg = Config::load_or_create();
            let mut rargs = matrix_sim::parse_render_args(&cfg.lockscreen.options.to_args(wid));
            rargs.window_id = Some(wid);
            return Some(win::gl_render::run_renderer(&rargs));
        }
        return Some(Ok(()));
    }

    if first == "/c" || first == "-c" || first.starts_with("/c:") || first.starts_with("-c:") {
        return Some(launch_panel());
    }

    None
}

fn has_gui_session() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(unix)]
    {
        std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    if let Some(res) = handle_windows_scr_args() {
        return res;
    }

    let cli = Cli::parse();
    let default_cmd = if has_gui_session() {
        Commands::Panel
    } else {
        Commands::Daemon
    };

    match cli.command.unwrap_or(default_cmd) {
        Commands::Daemon => {
            println!("Memulai glmatrix-live daemon...");
            platform_run_daemon()?;
        }
        Commands::Panel => {
            launch_panel()?;
        }
        Commands::On => {
            match platform_send_command("wallpaper-on") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    let mut cfg = Config::load_or_create();
                    cfg.general.desktop_enabled = true;
                    cfg.save()?;
                    println!("Konfigurasi live wallpaper diaktifkan. Memulai daemon di latar belakang...");
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = Command::new(exe).arg("daemon").spawn();
                    }
                }
            }
        }
        Commands::Off => {
            match platform_send_command("wallpaper-off") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    let mut cfg = Config::load_or_create();
                    cfg.general.desktop_enabled = false;
                    cfg.save()?;
                    println!("Live wallpaper dinonaktifkan di konfigurasi (daemon sedang tidak berjalan).");
                }
            }
        }
        Commands::Toggle => {
            match platform_send_command("wallpaper-toggle") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    let mut cfg = Config::load_or_create();
                    cfg.general.desktop_enabled = !cfg.general.desktop_enabled;
                    cfg.save()?;
                    if cfg.general.desktop_enabled {
                        println!("Live wallpaper diaktifkan. Memulai daemon...");
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = Command::new(exe).arg("daemon").spawn();
                        }
                    } else {
                        println!("Live wallpaper dinonaktifkan.");
                    }
                }
            }
        }
        Commands::Lock => {
            match platform_send_command("lock") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    #[cfg(unix)]
                    {
                        println!("Daemon tidak aktif, menjalankan lockscreen langsung...");
                        let cfg = Config::load_or_create();
                        x11_lock::run_lockscreen(&cfg)?;
                    }
                    #[cfg(windows)]
                    {
                        win::daemon::lock_windows_screen();
                    }
                }
            }
        }
        Commands::Desktop => {
            #[cfg(unix)]
            {
                let cfg = Config::load_or_create();
                println!("Menjalankan desktop live wallpaper...");
                let (_conn, mut sess) = x11_desktop::start_desktop(&cfg)?;
                println!("Live wallpaper aktif (PID: {}). Tekan Ctrl+C untuk keluar.", sess.child.id());
                sess.child.wait()?;
            }
            #[cfg(windows)]
            {
                let cfg = Config::load_or_create();
                println!("Menjalankan desktop live wallpaper Windows...");
                let mut sess = win::desktop::start_desktop(&cfg)?;
                println!("Live wallpaper aktif (PID: {}). Tekan Ctrl+C untuk keluar.", sess.child.id());
                sess.child.wait()?;
            }
        }
        Commands::Stop => {
            match platform_send_command("stop") {
                Ok(resp) => print!("{}", resp),
                Err(_) => println!("Daemon glmatrix-live sedang tidak berjalan."),
            }
        }
        Commands::Restart => {
            match platform_send_command("restart") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    println!("Daemon tidak aktif. Menjalankan daemon baru...");
                    platform_run_daemon()?;
                }
            }
        }
        Commands::Status => {
            match platform_send_command("status") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    println!("glmatrix-live daemon: NONAKTIF");
                    println!("Lokasi config: {:?}", Config::config_path());
                }
            }
        }
        Commands::Config { gui } => {
            if gui {
                launch_panel()?;
            } else {
                let path = Config::config_path();
                let _ = Config::load_or_create();
                println!("File konfigurasi glmatrix-live:");
                println!("  Path: {:?}", path);
                println!("\nUntuk membuka panel konfigurasi GUI, jalankan: glmatrix-live panel");
            }
        }
        Commands::Render { args } => {
            let parsed = matrix_sim::parse_render_args(&args);
            platform_run_renderer(&parsed)?;
        }
    }

    Ok(())
}
