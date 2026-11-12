use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub desktop: MatrixOptions,
    pub lockscreen: LockscreenConfig,
}

fn default_true() -> bool {
    true
}

fn default_mode() -> String {
    "matrix".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub glmatrix_path: String,
    pub pam_service: String,
    #[serde(default = "default_true")]
    pub desktop_enabled: bool,
}

impl GeneralConfig {
    pub fn get_glmatrix_path(&self) -> String {
        let trimmed = self.glmatrix_path.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("builtin") {
            return "builtin".to_string();
        }
        if std::path::Path::new(trimmed).exists() {
            return trimmed.to_string();
        }
        "builtin".to_string()
    }

    pub fn get_pam_service(&self) -> String {
        #[cfg(unix)]
        {
            if std::path::Path::new(&format!("/etc/pam.d/{}", self.pam_service)).exists() {
                return self.pam_service.clone();
            }
            for svc in &["glmatrix-live", "xscreensaver", "login", "common-auth"] {
                if std::path::Path::new(&format!("/etc/pam.d/{}", svc)).exists() {
                    return svc.to_string();
                }
            }
        }
        self.pam_service.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixOptions {
    pub delay: u32,
    pub speed: f32,
    pub density: u32,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub no_waves: bool,
    pub no_rotate: bool,
    pub no_fog: bool,
    #[serde(default)]
    pub no_texture: bool,
    #[serde(default)]
    pub wireframe: bool,
    #[serde(default)]
    pub binary: bool,
    pub clock: bool,
    #[serde(default)]
    pub timefmt: String,
    #[serde(default)]
    pub show_fps: bool,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockscreenConfig {
    pub auto_lock_mins: u32,
    pub options: MatrixOptions,
    pub banner_text: String,
}

impl Default for Config {
    fn default() -> Self {
        let default_glmatrix = "builtin".to_string();

        #[cfg(unix)]
        let default_pam = if std::path::Path::new("/etc/pam.d/glmatrix-live").exists() {
            "glmatrix-live".to_string()
        } else if std::path::Path::new("/etc/pam.d/xscreensaver").exists() {
            "xscreensaver".to_string()
        } else {
            "glmatrix-live".to_string()
        };
        #[cfg(windows)]
        let default_pam = "windows".to_string();

        Self {
            general: GeneralConfig {
                glmatrix_path: default_glmatrix,
                pam_service: default_pam,
                desktop_enabled: true,
            },
            desktop: MatrixOptions {
                delay: 35000,
                speed: 1.0,
                density: 20,
                mode: "matrix".to_string(),
                no_waves: true,
                no_rotate: true,
                no_fog: false,
                no_texture: false,
                wireframe: false,
                binary: false,
                clock: false,
                timefmt: String::new(),
                show_fps: false,
                extra_args: Vec::new(),
            },
            lockscreen: LockscreenConfig {
                auto_lock_mins: 5,
                options: MatrixOptions {
                    delay: 30000,
                    speed: 1.2,
                    density: 25,
                    mode: "matrix".to_string(),
                    no_waves: true,
                    no_rotate: true,
                    no_fog: false,
                    no_texture: false,
                    wireframe: false,
                    binary: false,
                    clock: false,
                    timefmt: String::new(),
                    show_fps: false,
                    extra_args: Vec::new(),
                },
                banner_text: "MATRIX TERMINAL LOCKED".to_string(),
            },
        }
    }
}

impl MatrixOptions {
    pub fn effective_mode(&self) -> &str {
        if (self.mode.is_empty() || self.mode == "matrix") && self.binary {
            "binary"
        } else if self.mode.is_empty() {
            "matrix"
        } else {
            &self.mode
        }
    }

    pub fn to_args(&self, window_id: u64) -> Vec<String> {
        let mut args = vec![
            "-window-id".to_string(),
            format!("0x{:x}", window_id),
            "--delay".to_string(),
            self.delay.to_string(),
            "--speed".to_string(),
            format!("{:.2}", self.speed),
            "--density".to_string(),
            self.density.to_string(),
        ];

        match self.effective_mode() {
            "binary" => args.push("--binary".to_string()),
            "hexadecimal" | "hex" => args.push("--hexadecimal".to_string()),
            "decimal" => args.push("--decimal".to_string()),
            "dna" => args.push("--dna".to_string()),
            _ => {}
        }

        if self.no_waves {
            args.push("--no-waves".to_string());
        }
        if self.no_rotate {
            args.push("--no-rotate".to_string());
        }
        if self.no_fog {
            args.push("--no-fog".to_string());
        }
        if self.no_texture {
            args.push("--no-texture".to_string());
        }
        if self.wireframe {
            args.push("--wireframe".to_string());
        }
        if self.clock {
            args.push("--clock".to_string());
            if !self.timefmt.trim().is_empty() {
                args.push("--timefmt".to_string());
                args.push(self.timefmt.clone());
            }
        }
        if self.show_fps {
            args.push("--fps".to_string());
        }

        for extra in &self.extra_args {
            if !extra.trim().is_empty() {
                args.push(extra.clone());
            }
        }

        args
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                return Path::new(&appdata).join("glmatrix-live").join("config.toml");
            }
            if let Ok(profile) = std::env::var("USERPROFILE") {
                return Path::new(&profile)
                    .join("AppData")
                    .join("Roaming")
                    .join("glmatrix-live")
                    .join("config.toml");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            Path::new(&home).join(".config").join("glmatrix-live").join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load_or_create() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut cfg) = toml::from_str::<Config>(&content) {
                if cfg.desktop.binary && cfg.desktop.mode == "matrix" {
                    cfg.desktop.mode = "binary".to_string();
                }
                if cfg.lockscreen.options.binary && cfg.lockscreen.options.mode == "matrix" {
                    cfg.lockscreen.options.mode = "binary".to_string();
                }
                return cfg;
            }
        }

        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    pub fn spawn_glmatrix(
        &self,
        opts: &MatrixOptions,
        window_id: u64,
    ) -> Result<std::process::Child, String> {
        let target = self.general.get_glmatrix_path();
        let args = opts.to_args(window_id);
        if target == "builtin" {
            let exe = std::env::current_exe()
                .map_err(|e| format!("Gagal menemukan executable glmatrix-live: {}", e))?;
            std::process::Command::new(&exe)
                .arg("render")
                .args(&args)
                .spawn()
                .map_err(|e| format!("Gagal menjalankan renderer bawaan ({:?}): {}", exe, e))
        } else {
            std::process::Command::new(&target)
                .args(&args)
                .spawn()
                .map_err(|e| format!("Gagal menjalankan glmatrix di {}: {}", target, e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_deserializes_with_defaults() {
        let legacy = r#"
[general]
glmatrix_path = "/usr/libexec/xscreensaver/glmatrix"
pam_service = "xscreensaver"

[desktop]
delay = 35000
speed = 1.0
density = 20
no_waves = true
no_rotate = true
no_fog = false
binary = true
clock = false
extra_args = []

[lockscreen]
auto_lock_mins = 5
banner_text = "MATRIX TERMINAL LOCKED"

[lockscreen.options]
delay = 30000
speed = 1.2
density = 25
no_waves = true
no_rotate = true
no_fog = false
binary = false
clock = false
extra_args = []
"#;
        let cfg: Config = toml::from_str(legacy).expect("legacy config should parse");
        assert!(cfg.general.desktop_enabled);
        assert_eq!(cfg.desktop.effective_mode(), "binary");
        assert_eq!(cfg.lockscreen.options.effective_mode(), "matrix");
        assert!(!cfg.desktop.wireframe);
        assert!(!cfg.desktop.no_texture);
        assert!(!cfg.desktop.show_fps);
        assert_eq!(cfg.desktop.timefmt, "");
    }

    #[test]
    fn to_args_includes_all_glmatrix_options() {
        let mut opts = Config::default().desktop;
        opts.speed = 2.5;
        opts.density = 45;
        opts.delay = 16667;
        opts.mode = "dna".to_string();
        opts.no_fog = true;
        opts.no_waves = false;
        opts.no_rotate = false;
        opts.no_texture = true;
        opts.wireframe = true;
        opts.clock = true;
        opts.timefmt = " %H:%M ".to_string();
        opts.show_fps = true;
        opts.extra_args = vec!["--mono".to_string()];

        let args = opts.to_args(0x1234);
        assert!(args.windows(2).any(|w| w == ["-window-id", "0x1234"]));
        assert!(args.windows(2).any(|w| w == ["--delay", "16667"]));
        assert!(args.windows(2).any(|w| w == ["--speed", "2.50"]));
        assert!(args.windows(2).any(|w| w == ["--density", "45"]));
        assert!(args.contains(&"--dna".to_string()));
        assert!(args.contains(&"--no-fog".to_string()));
        assert!(!args.contains(&"--no-waves".to_string()));
        assert!(!args.contains(&"--no-rotate".to_string()));
        assert!(args.contains(&"--no-texture".to_string()));
        assert!(args.contains(&"--wireframe".to_string()));
        assert!(args.contains(&"--clock".to_string()));
        assert!(args.windows(2).any(|w| w == ["--timefmt", " %H:%M "]));
        assert!(args.contains(&"--fps".to_string()));
        assert!(args.contains(&"--mono".to_string()));
    }
}
