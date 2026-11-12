mod config;
mod pam;
mod x11_desktop;
mod x11_lock;
mod daemon;

use clap::{Parser, Subcommand};
use config::Config;
use std::path::PathBuf;
use std::process::Command;

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
    /// Kunci layar sekarang dengan animasi Matrix & autentikasi PAM
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
}

fn launch_panel() -> Result<(), Box<dyn std::error::Error>> {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let default_cmd = if std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()) {
        Commands::Panel
    } else {
        Commands::Daemon
    };

    match cli.command.unwrap_or(default_cmd) {
        Commands::Daemon => {
            println!("Memulai glmatrix-live daemon...");
            daemon::run_daemon()?;
        }
        Commands::Panel => {
            launch_panel()?;
        }
        Commands::On => {
            match daemon::send_command("wallpaper-on") {
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
            match daemon::send_command("wallpaper-off") {
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
            match daemon::send_command("wallpaper-toggle") {
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
            // Coba kirim sinyal ke daemon jika aktif, jika tidak, jalankan langsung
            match daemon::send_command("lock") {
                Ok(resp) => {
                    print!("{}", resp);
                }
                Err(_) => {
                    println!("Daemon tidak aktif, menjalankan lockscreen langsung...");
                    let cfg = Config::load_or_create();
                    x11_lock::run_lockscreen(&cfg)?;
                }
            }
        }
        Commands::Desktop => {
            let cfg = Config::load_or_create();
            println!("Menjalankan desktop live wallpaper...");
            let (_conn, mut sess) = x11_desktop::start_desktop(&cfg)?;
            println!("Live wallpaper aktif (PID: {}). Tekan Ctrl+C untuk keluar.", sess.child.id());
            sess.child.wait()?;
        }
        Commands::Stop => {
            match daemon::send_command("stop") {
                Ok(resp) => print!("{}", resp),
                Err(_) => println!("Daemon glmatrix-live sedang tidak berjalan."),
            }
        }
        Commands::Restart => {
            match daemon::send_command("restart") {
                Ok(resp) => print!("{}", resp),
                Err(_) => {
                    println!("Daemon tidak aktif. Menjalankan daemon baru...");
                    daemon::run_daemon()?;
                }
            }
        }
        Commands::Status => {
            match daemon::send_command("status") {
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
    }

    Ok(())
}
