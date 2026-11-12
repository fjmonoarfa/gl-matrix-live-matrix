#!/usr/bin/env python3
"""Panel konfigurasi GUI GTK3 untuk glmatrix-live & screensaver standar glmatrix."""

import json
import os
import re
import shlex
import shutil
import socket
import subprocess
import sys
import tomllib
from pathlib import Path

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk

try:
    gi.require_version("GdkX11", "3.0")
    from gi.repository import GdkX11
except (ValueError, ImportError):
    GdkX11 = None


APP_CSS = b"""
.matrix-banner {
    background-color: #0b1610;
    border: 1px solid #1e4d2b;
    border-radius: 8px;
    padding: 10px 14px;
}
.matrix-title {
    color: #00ff66;
    font-weight: bold;
    font-size: 15px;
}
.matrix-subtitle {
    color: #9ec5ab;
    font-size: 11px;
}
.preview-frame {
    background-color: #000000;
    border: 1px solid #1e4d2b;
    border-radius: 6px;
}
"""

MODE_CHOICES = [
    ("matrix", "Matrix (Katakana / Default)"),
    ("binary", "Biner (0 & 1)"),
    ("hexadecimal", "Heksadesimal (0-F)"),
    ("decimal", "Desimal (0-9)"),
    ("dna", "DNA (G, A, T, C)"),
]

XSCREENSAVER_MODES = [
    ("one", "Satu Screensaver (GLMatrix)"),
    ("random", "Acak (Random)"),
    ("random-same", "Acak Sama di Semua Monitor"),
    ("blank", "Layar Hitam Saja (Blank)"),
    ("off", "Nonaktif (Off)"),
]


def get_config_path() -> Path:
    home = os.environ.get("HOME")
    if home:
        return Path(home) / ".config" / "glmatrix-live" / "config.toml"
    return Path("config.toml")


def get_xscreensaver_path() -> Path:
    home = os.environ.get("HOME")
    if home:
        return Path(home) / ".xscreensaver"
    return Path(".xscreensaver")


def get_socket_path() -> Path:
    base = os.environ.get("XDG_RUNTIME_DIR") or "/tmp"
    return Path(base) / "glmatrix-live.sock"


def get_binary_path() -> str:
    env_bin = os.environ.get("GLMATRIX_LIVE_BIN", "").strip()
    if env_bin and os.path.exists(env_bin):
        return env_bin
    for candidate in (
        "/usr/local/bin/glmatrix-live",
        "/usr/bin/glmatrix-live",
        "/opt/glmatrix-live/glmatrix-live",
        str(Path.home() / ".local" / "bin" / "glmatrix-live"),
    ):
        if os.path.exists(candidate):
            return candidate
    return "glmatrix-live"


def default_glmatrix_path() -> str:
    for p in (
        "/opt/glmatrix-live/libexec/glmatrix",
        "/usr/libexec/xscreensaver/glmatrix",
        "/usr/lib/xscreensaver/glmatrix",
        "/usr/bin/glmatrix",
    ):
        if os.path.exists(p):
            return p
    return "/usr/libexec/xscreensaver/glmatrix"


def default_pam_service() -> str:
    for svc in ("glmatrix-live", "xscreensaver", "login", "common-auth"):
        if os.path.exists(f"/etc/pam.d/{svc}"):
            return svc
    return "xscreensaver"


def default_matrix_options(
    delay=35000,
    speed=1.0,
    density=20,
    no_waves=True,
    no_rotate=True,
) -> dict:
    return {
        "delay": delay,
        "speed": speed,
        "density": density,
        "mode": "matrix",
        "no_waves": no_waves,
        "no_rotate": no_rotate,
        "no_fog": False,
        "no_texture": False,
        "wireframe": False,
        "binary": False,
        "clock": False,
        "timefmt": "",
        "show_fps": False,
        "extra_args": [],
    }


def standard_glmatrix_options() -> dict:
    """Preset standar bawaan glmatrix XScreenSaver."""
    return default_matrix_options(
        delay=30000,
        speed=1.0,
        density=20,
        no_waves=False,
        no_rotate=False,
    )


def default_config() -> dict:
    return {
        "general": {
            "glmatrix_path": default_glmatrix_path(),
            "pam_service": default_pam_service(),
            "desktop_enabled": True,
        },
        "desktop": default_matrix_options(delay=35000, speed=1.0, density=20),
        "lockscreen": {
            "auto_lock_mins": 5,
            "banner_text": "MATRIX TERMINAL LOCKED",
            "options": default_matrix_options(delay=30000, speed=1.2, density=25),
        },
    }


def normalize_matrix_options(raw: dict, fallback: dict) -> dict:
    out = dict(fallback)
    out.update({k: raw[k] for k in fallback if k in raw})
    out["delay"] = int(max(0, min(100000, int(out.get("delay", fallback["delay"])))))
    out["speed"] = float(max(0.1, min(8.0, float(out.get("speed", fallback["speed"])))))
    out["density"] = int(max(0, min(100, int(out.get("density", fallback["density"])))))
    mode = str(out.get("mode", "matrix")).strip().lower()
    if mode == "hex":
        mode = "hexadecimal"
    if (not mode or mode == "matrix") and bool(out.get("binary", False)):
        mode = "binary"
    valid_modes = {m[0] for m in MODE_CHOICES}
    if mode not in valid_modes:
        mode = "matrix"
    out["mode"] = mode
    out["binary"] = mode == "binary"
    out["timefmt"] = str(out.get("timefmt", ""))
    extra = out.get("extra_args", [])
    out["extra_args"] = [str(x) for x in extra] if isinstance(extra, list) else []
    return out


def load_config() -> dict:
    cfg = default_config()
    path = get_config_path()
    if path.exists():
        try:
            with open(path, "rb") as f:
                data = tomllib.load(f)
            gen = data.get("general", {})
            if isinstance(gen, dict):
                cfg["general"]["glmatrix_path"] = str(
                    gen.get("glmatrix_path", cfg["general"]["glmatrix_path"])
                )
                cfg["general"]["pam_service"] = str(
                    gen.get("pam_service", cfg["general"]["pam_service"])
                )
                cfg["general"]["desktop_enabled"] = bool(
                    gen.get("desktop_enabled", True)
                )
            if isinstance(data.get("desktop"), dict):
                cfg["desktop"] = normalize_matrix_options(
                    data["desktop"], cfg["desktop"]
                )
            ls = data.get("lockscreen", {})
            if isinstance(ls, dict):
                cfg["lockscreen"]["auto_lock_mins"] = int(
                    max(0, int(ls.get("auto_lock_mins", 5)))
                )
                cfg["lockscreen"]["banner_text"] = str(
                    ls.get("banner_text", "MATRIX TERMINAL LOCKED")
                )
                if isinstance(ls.get("options"), dict):
                    cfg["lockscreen"]["options"] = normalize_matrix_options(
                        ls["options"], cfg["lockscreen"]["options"]
                    )
        except Exception as exc:
            print(f"Peringatan gagal membaca config: {exc}", file=sys.stderr)
    return cfg


def toml_str(val: str) -> str:
    return json.dumps(str(val), ensure_ascii=False)


def toml_bool(val: bool) -> str:
    return "true" if val else "false"


def format_matrix_section(opts: dict) -> list[str]:
    extra = ", ".join(toml_str(x) for x in opts.get("extra_args", []))
    return [
        f"delay = {int(opts['delay'])}",
        f"speed = {round(float(opts['speed']), 2)}",
        f"density = {int(opts['density'])}",
        f"mode = {toml_str(opts['mode'])}",
        f"no_waves = {toml_bool(opts['no_waves'])}",
        f"no_rotate = {toml_bool(opts['no_rotate'])}",
        f"no_fog = {toml_bool(opts['no_fog'])}",
        f"no_texture = {toml_bool(opts['no_texture'])}",
        f"wireframe = {toml_bool(opts['wireframe'])}",
        f"binary = {toml_bool(opts['mode'] == 'binary')}",
        f"clock = {toml_bool(opts['clock'])}",
        f"timefmt = {toml_str(opts['timefmt'])}",
        f"show_fps = {toml_bool(opts['show_fps'])}",
        f"extra_args = [{extra}]",
    ]


def save_config(cfg: dict) -> None:
    path = get_config_path()
    path.parent.mkdir(parents=True, exist_ok=True)

    lines = [
        "[general]",
        f"glmatrix_path = {toml_str(cfg['general']['glmatrix_path'])}",
        f"pam_service = {toml_str(cfg['general']['pam_service'])}",
        f"desktop_enabled = {toml_bool(cfg['general']['desktop_enabled'])}",
        "",
        "[desktop]",
        *format_matrix_section(cfg["desktop"]),
        "",
        "[lockscreen]",
        f"auto_lock_mins = {int(cfg['lockscreen']['auto_lock_mins'])}",
        f"banner_text = {toml_str(cfg['lockscreen']['banner_text'])}",
        "",
        "[lockscreen.options]",
        *format_matrix_section(cfg["lockscreen"]["options"]),
        "",
    ]

    tmp_path = path.with_suffix(".toml.tmp")
    tmp_path.write_text("\n".join(lines), encoding="utf-8")
    tmp_path.replace(path)


def build_glmatrix_cli_flags(opts: dict, include_root: bool = False) -> list[str]:
    args: list[str] = []
    if include_root:
        args.append("--root")
    args.extend([
        "--delay",
        str(int(opts["delay"])),
        "--speed",
        f"{float(opts['speed']):.2f}",
        "--density",
        str(int(opts["density"])),
    ])
    mode = opts.get("mode", "matrix")
    if mode == "binary":
        args.append("--binary")
    elif mode == "hexadecimal":
        args.append("--hexadecimal")
    elif mode == "decimal":
        args.append("--decimal")
    elif mode == "dna":
        args.append("--dna")

    if opts.get("no_waves"):
        args.append("--no-waves")
    if opts.get("no_rotate"):
        args.append("--no-rotate")
    if opts.get("no_fog"):
        args.append("--no-fog")
    if opts.get("no_texture"):
        args.append("--no-texture")
    if opts.get("wireframe"):
        args.append("--wireframe")
    if opts.get("clock"):
        args.append("--clock")
        tf = opts.get("timefmt", "").strip()
        if tf:
            args.extend(["--timefmt", opts["timefmt"]])
    if opts.get("show_fps"):
        args.append("--fps")

    for extra in opts.get("extra_args", []):
        if extra.strip():
            args.append(extra)
    return args


def parse_glmatrix_cmdline(cmdline: str) -> dict:
    """Parse argumen glmatrix dari baris ~/.xscreensaver menjadi MatrixOptions."""
    opts = standard_glmatrix_options()
    try:
        tokens = shlex.split(cmdline)
    except ValueError:
        tokens = cmdline.split()

    extra: list[str] = []
    i = 0
    while i < len(tokens):
        tok = tokens[i]
        if i == 0 and (tok == "glmatrix" or tok.endswith("/glmatrix")):
            i += 1
            continue
        if tok in ("--root", "-root"):
            i += 1
            continue
        if tok in ("--delay", "-delay") and i + 1 < len(tokens):
            try:
                opts["delay"] = int(tokens[i + 1])
            except ValueError:
                pass
            i += 2
            continue
        if tok in ("--speed", "-speed") and i + 1 < len(tokens):
            try:
                opts["speed"] = float(tokens[i + 1])
            except ValueError:
                pass
            i += 2
            continue
        if tok in ("--density", "-density") and i + 1 < len(tokens):
            try:
                opts["density"] = int(tokens[i + 1])
            except ValueError:
                pass
            i += 2
            continue
        if tok in ("--mode", "-mode") and i + 1 < len(tokens):
            m = tokens[i + 1].lower()
            opts["mode"] = "hexadecimal" if m == "hex" else m
            i += 2
            continue
        if tok in ("--binary", "-binary"):
            opts["mode"] = "binary"
        elif tok in ("--hexadecimal", "-hexadecimal", "--hex", "-hex"):
            opts["mode"] = "hexadecimal"
        elif tok in ("--decimal", "-decimal"):
            opts["mode"] = "decimal"
        elif tok in ("--dna", "-dna"):
            opts["mode"] = "dna"
        elif tok in ("--no-waves", "-no-waves"):
            opts["no_waves"] = True
        elif tok in ("--waves", "-waves"):
            opts["no_waves"] = False
        elif tok in ("--no-rotate", "-no-rotate"):
            opts["no_rotate"] = True
        elif tok in ("--rotate", "-rotate"):
            opts["no_rotate"] = False
        elif tok in ("--no-fog", "-no-fog"):
            opts["no_fog"] = True
        elif tok in ("--fog", "-fog"):
            opts["no_fog"] = False
        elif tok in ("--no-texture", "-no-texture"):
            opts["no_texture"] = True
        elif tok in ("--texture", "-texture"):
            opts["no_texture"] = False
        elif tok in ("--wireframe", "-wireframe"):
            opts["wireframe"] = True
        elif tok in ("--no-wireframe", "-no-wireframe"):
            opts["wireframe"] = False
        elif tok in ("--clock", "-clock"):
            opts["clock"] = True
        elif tok in ("--no-clock", "-no-clock"):
            opts["clock"] = False
        elif tok in ("--timefmt", "-timefmt") and i + 1 < len(tokens):
            opts["timefmt"] = tokens[i + 1]
            i += 2
            continue
        elif tok in ("--fps", "-fps"):
            opts["show_fps"] = True
        elif tok in ("--no-fps", "-no-fps"):
            opts["show_fps"] = False
        else:
            extra.append(tok)
        i += 1

    opts["extra_args"] = extra
    return normalize_matrix_options(opts, standard_glmatrix_options())


def parse_hms_to_mins(val: str, default_mins: int = 5) -> int:
    parts = val.strip().split(":")
    try:
        if len(parts) == 3:
            return max(0, int(parts[0]) * 60 + int(parts[1]))
        if len(parts) == 2:
            return max(0, int(parts[0]))
    except ValueError:
        pass
    return default_mins


def mins_to_hms(mins: int) -> str:
    mins = max(0, int(mins))
    h, m = divmod(mins, 60)
    return f"{h}:{m:02d}:00"


def load_xscreensaver_settings() -> dict:
    data = {
        "exists": False,
        "mode": "one",
        "timeout_mins": 5,
        "lock": False,
        "lock_timeout_mins": 0,
        "fade": True,
        "dpms_enabled": False,
        "selected": 0,
        "glmatrix_options": standard_glmatrix_options(),
    }
    path = get_xscreensaver_path()
    if not path.exists():
        return data

    data["exists"] = True
    text = path.read_text(encoding="utf-8", errors="replace")
    kv: dict[str, str] = {}
    for line in text.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        if line.startswith("programs:"):
            break
        m = re.match(r"^([A-Za-z0-9_]+):\s*(.*)$", line)
        if m:
            kv[m.group(1)] = m.group(2).strip()

    if kv.get("mode") in {c[0] for c in XSCREENSAVER_MODES}:
        data["mode"] = kv["mode"]
    if "timeout" in kv:
        data["timeout_mins"] = max(1, parse_hms_to_mins(kv["timeout"], 5))
    if "lock" in kv:
        data["lock"] = kv["lock"].lower() == "true"
    if "lockTimeout" in kv:
        data["lock_timeout_mins"] = parse_hms_to_mins(kv["lockTimeout"], 0)
    if "fade" in kv:
        data["fade"] = kv["fade"].lower() == "true"
    if "dpmsEnabled" in kv:
        data["dpms_enabled"] = kv["dpmsEnabled"].lower() == "true"
    if "selected" in kv:
        try:
            data["selected"] = int(kv["selected"])
        except ValueError:
            pass

    for line in text.splitlines():
        if "glmatrix" in line and "--root" in line:
            stripped = line.strip()
            if stripped.endswith("\\n\\"):
                stripped = stripped[:-3].strip()
            m = re.search(r"(glmatrix\s+.*)$", stripped)
            if m:
                data["glmatrix_options"] = parse_glmatrix_cmdline(m.group(1))
                break

    return data


def save_xscreensaver_settings(xs: dict) -> None:
    path = get_xscreensaver_path()
    flags = build_glmatrix_cli_flags(xs["glmatrix_options"], include_root=True)
    cmd_str = "glmatrix " + " ".join(shlex.quote(a) for a in flags)
    glmatrix_entry = f'\t\t    "GLMatrix" \t{cmd_str}\t\t    \\n\\'

    if not path.exists():
        lines = [
            "# XScreenSaver Preferences File",
            f"timeout:\t{mins_to_hms(xs['timeout_mins'])}",
            "cycle:\t\t0:05:00",
            f"lock:\t\t{'True' if xs['lock'] else 'False'}",
            f"lockTimeout:\t{mins_to_hms(xs['lock_timeout_mins'])}",
            "passwdTimeout:\t0:00:30",
            f"fade:\t\t{'True' if xs['fade'] else 'False'}",
            f"dpmsEnabled:\t{'True' if xs['dpms_enabled'] else 'False'}",
            f"mode:\t\t{xs['mode']}",
            f"selected:\t{int(xs.get('selected', 0))}",
            "",
            "programs:\t\t\t\t\t\t\t\t      \\",
            glmatrix_entry,
            "",
        ]
        path.write_text("\n".join(lines), encoding="utf-8")
        return

    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    updates = {
        "timeout": f"timeout:\t{mins_to_hms(xs['timeout_mins'])}",
        "lock": f"lock:\t\t{'True' if xs['lock'] else 'False'}",
        "lockTimeout": f"lockTimeout:\t{mins_to_hms(xs['lock_timeout_mins'])}",
        "fade": f"fade:\t\t{'True' if xs['fade'] else 'False'}",
        "dpmsEnabled": f"dpmsEnabled:\t{'True' if xs['dpms_enabled'] else 'False'}",
        "mode": f"mode:\t\t{xs['mode']}",
        "selected": f"selected:\t{int(xs.get('selected', 0))}",
    }

    out_lines: list[str] = []
    seen_keys: set[str] = set()
    replaced_glmatrix = False
    in_programs = False
    programs_index = -1

    for idx, line in enumerate(lines):
        if line.startswith("programs:"):
            in_programs = True
            for k, val_line in updates.items():
                if k not in seen_keys:
                    out_lines.append(val_line)
                    seen_keys.add(k)
            programs_index = len(out_lines)
            out_lines.append(line)
            continue

        if not in_programs:
            matched_key = None
            for k in updates:
                if re.match(rf"^{re.escape(k)}:\s*", line):
                    matched_key = k
                    break
            if matched_key:
                out_lines.append(updates[matched_key])
                seen_keys.add(matched_key)
                continue
        else:
            if not replaced_glmatrix and "glmatrix" in line and "--root" in line:
                # Pertahankan tanda '-' jika dinonaktifkan? Tidak, aktifkan karena user mengedit GLMatrix.
                if line.rstrip().endswith("\\"):
                    out_lines.append(glmatrix_entry)
                else:
                    out_lines.append(f'\t\t    "GLMatrix" \t{cmd_str}')
                replaced_glmatrix = True
                continue

        out_lines.append(line)

    if not replaced_glmatrix:
        if programs_index >= 0:
            out_lines.insert(programs_index + 1, glmatrix_entry)
        else:
            out_lines.extend(["", "programs:\t\t\t\t\t\t\t\t      \\", glmatrix_entry])

    tmp_path = path.with_name(".xscreensaver.tmp")
    tmp_path.write_text("\n".join(out_lines) + "\n", encoding="utf-8")
    tmp_path.replace(path)


def send_daemon_socket(cmd: str, timeout: float = 1.0) -> tuple[bool, str]:
    sock_path = get_socket_path()
    if not sock_path.exists():
        return False, ""
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.settimeout(timeout)
            sock.connect(str(sock_path))
            sock.sendall((cmd + "\n").encode("utf-8"))
            chunks = []
            while True:
                data = sock.recv(4096)
                if not data:
                    break
                chunks.append(data)
            return True, b"".join(chunks).decode("utf-8", errors="replace")
    except OSError:
        return False, ""


class EmbeddedMatrixPreview(Gtk.Box):
    """Widget preview live glmatrix tertanam di dalam panel GTK3 (X11)."""

    def __init__(self, get_glmatrix_bin_cb):
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self._get_glmatrix_bin = get_glmatrix_bin_cb
        self._proc = None
        self._enabled = True
        self._pending_opts: dict | None = None
        self._debounce_id: int | None = None
        self._last_args: list[str] | None = None

        header = Gtk.Box(spacing=8)
        lbl = Gtk.Label(xalign=0)
        lbl.set_markup("<b>Live Preview GLMatrix</b>")
        self.chk_live = Gtk.CheckButton(label="Preview Aktif")
        self.chk_live.set_active(True)
        self.chk_live.connect("toggled", self._on_live_toggled)

        self.btn_popout = Gtk.Button(label="Jendela Terpisah")
        self.btn_popout.get_style_context().add_class("flat")

        header.pack_start(lbl, True, True, 0)
        header.pack_end(self.btn_popout, False, False, 0)
        header.pack_end(self.chk_live, False, False, 0)
        self.pack_start(header, False, False, 0)

        frame = Gtk.Frame()
        frame.get_style_context().add_class("preview-frame")
        self.drawing_area = Gtk.DrawingArea()
        self.drawing_area.set_size_request(280, 190)
        self.drawing_area.connect("realize", self._on_realize)
        self.drawing_area.connect("unrealize", lambda _w: self.stop())
        frame.add(self.drawing_area)
        self.pack_start(frame, True, True, 0)

        self.lbl_info = Gtk.Label(label="Menyiapkan preview...", xalign=0)
        self.lbl_info.get_style_context().add_class("dim-label")
        self.pack_start(self.lbl_info, False, False, 0)

    def _on_realize(self, _widget) -> None:
        if self._pending_opts is not None and self._enabled:
            self._restart_now()

    def _on_live_toggled(self, chk: Gtk.CheckButton) -> None:
        self._enabled = chk.get_active()
        if self._enabled:
            self._last_args = None
            self._restart_now()
        else:
            self.stop()
            self.lbl_info.set_text("Preview dijeda.")

    def update_options(self, opts: dict, immediate: bool = False) -> None:
        self._pending_opts = dict(opts)
        if not self._enabled:
            return
        if self._debounce_id is not None:
            GLib.source_remove(self._debounce_id)
            self._debounce_id = None
        if immediate:
            self._restart_now()
        else:
            self._debounce_id = GLib.timeout_add(220, self._debounced_restart)

    def _debounced_restart(self) -> bool:
        self._debounce_id = None
        self._restart_now()
        return False

    def _get_xid(self) -> int | None:
        gdk_win = self.drawing_area.get_window()
        if gdk_win is None:
            return None
        if GdkX11 is not None and isinstance(gdk_win, GdkX11.X11Window):
            return gdk_win.get_xid()
        if hasattr(gdk_win, "get_xid"):
            return gdk_win.get_xid()
        return None

    def _restart_now(self) -> None:
        if not self._enabled or self._pending_opts is None:
            return
        xid = self._get_xid()
        if xid is None:
            return

        glmatrix_bin = self._get_glmatrix_bin()
        if not os.path.exists(glmatrix_bin):
            self.lbl_info.set_text(f"Binary glmatrix tidak ditemukan: {glmatrix_bin}")
            return

        cli_flags = build_glmatrix_cli_flags(self._pending_opts, include_root=False)
        if cli_flags == self._last_args and self._proc is not None and self._proc.poll() is None:
            return

        self.stop()
        args = [glmatrix_bin, "-window-id", hex(xid)] + cli_flags
        try:
            self._proc = subprocess.Popen(
                args,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            self._last_args = cli_flags
            self.lbl_info.set_text(
                f"Mode: {self._pending_opts.get('mode', 'matrix')} | "
                f"Speed: {self._pending_opts.get('speed', 1.0):.2f}x | "
                f"Density: {self._pending_opts.get('density', 20)}%"
            )
        except Exception as exc:
            self.lbl_info.set_text(f"Gagal memulai preview: {exc}")

    def stop(self) -> None:
        if self._debounce_id is not None:
            GLib.source_remove(self._debounce_id)
            self._debounce_id = None
        if self._proc is not None:
            if self._proc.poll() is None:
                self._proc.terminate()
                try:
                    self._proc.wait(timeout=0.6)
                except subprocess.TimeoutExpired:
                    self._proc.kill()
            self._proc = None


class MatrixOptionsSection(Gtk.Box):
    """Widget reusable untuk mengedit seluruh opsi glmatrix."""

    def __init__(self, on_change=None):
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        self.set_margin_top(10)
        self.set_margin_bottom(10)
        self.set_margin_start(12)
        self.set_margin_end(12)
        self._on_change = on_change
        self._suppress = False

        # --- Section 1: Kecepatan & Performa ---
        perf_frame = Gtk.Frame(label=" Kecepatan, Kepadatan & Frame Rate ")
        perf_grid = Gtk.Grid(column_spacing=10, row_spacing=8)
        perf_grid.set_margin_top(8)
        perf_grid.set_margin_bottom(8)
        perf_grid.set_margin_start(10)
        perf_grid.set_margin_end(10)
        perf_frame.add(perf_grid)
        self.pack_start(perf_frame, False, False, 0)

        # Speed
        lbl_speed = Gtk.Label(label="Kecepatan Glyph:", xalign=0)
        self.speed_adj = Gtk.Adjustment(value=1.0, lower=0.1, upper=8.0, step_increment=0.05, page_increment=0.5)
        self.speed_scale = Gtk.Scale(orientation=Gtk.Orientation.HORIZONTAL, adjustment=self.speed_adj)
        self.speed_scale.set_digits(2)
        self.speed_scale.set_draw_value(False)
        self.speed_scale.set_hexpand(True)
        self.speed_spin = Gtk.SpinButton(adjustment=self.speed_adj, climb_rate=0.05, digits=2)
        self.speed_spin.set_width_chars(6)

        speed_presets = Gtk.Box(spacing=4)
        for label, val in [("0.5x", 0.5), ("1.0x", 1.0), ("2.0x", 2.0), ("4.0x", 4.0)]:
            btn = Gtk.Button(label=label)
            btn.get_style_context().add_class("flat")
            btn.connect("clicked", lambda _b, v=val: self.speed_adj.set_value(v))
            speed_presets.pack_start(btn, False, False, 0)

        perf_grid.attach(lbl_speed, 0, 0, 1, 1)
        perf_grid.attach(self.speed_scale, 1, 0, 1, 1)
        perf_grid.attach(self.speed_spin, 2, 0, 1, 1)
        perf_grid.attach(speed_presets, 1, 1, 2, 1)

        # Density
        lbl_density = Gtk.Label(label="Kepadatan (%):", xalign=0)
        self.density_adj = Gtk.Adjustment(value=20, lower=0, upper=100, step_increment=1, page_increment=5)
        self.density_scale = Gtk.Scale(orientation=Gtk.Orientation.HORIZONTAL, adjustment=self.density_adj)
        self.density_scale.set_digits(0)
        self.density_scale.set_draw_value(False)
        self.density_scale.set_hexpand(True)
        self.density_spin = Gtk.SpinButton(adjustment=self.density_adj, climb_rate=1, digits=0)
        self.density_spin.set_width_chars(6)

        perf_grid.attach(lbl_density, 0, 2, 1, 1)
        perf_grid.attach(self.density_scale, 1, 2, 1, 1)
        perf_grid.attach(self.density_spin, 2, 2, 1, 1)

        # Delay
        lbl_delay = Gtk.Label(label="Frame Delay (µs):", xalign=0)
        self.delay_adj = Gtk.Adjustment(value=35000, lower=0, upper=100000, step_increment=1000, page_increment=5000)
        self.delay_scale = Gtk.Scale(orientation=Gtk.Orientation.HORIZONTAL, adjustment=self.delay_adj)
        self.delay_scale.set_digits(0)
        self.delay_scale.set_draw_value(False)
        self.delay_scale.set_hexpand(True)
        self.delay_spin = Gtk.SpinButton(adjustment=self.delay_adj, climb_rate=1000, digits=0)
        self.delay_spin.set_width_chars(7)

        delay_sub = Gtk.Box(spacing=6)
        self.fps_hint = Gtk.Label(xalign=0)
        self.fps_hint.get_style_context().add_class("dim-label")
        delay_sub.pack_start(self.fps_hint, True, True, 0)
        for label, val in [("60 FPS", 16667), ("30 FPS", 33333), ("Hemat", 55000)]:
            btn = Gtk.Button(label=label)
            btn.get_style_context().add_class("flat")
            btn.connect("clicked", lambda _b, v=val: self.delay_adj.set_value(v))
            delay_sub.pack_start(btn, False, False, 0)

        self.delay_adj.connect("value-changed", self._update_fps_hint)
        self._update_fps_hint()

        perf_grid.attach(lbl_delay, 0, 3, 1, 1)
        perf_grid.attach(self.delay_scale, 1, 3, 1, 1)
        perf_grid.attach(self.delay_spin, 2, 3, 1, 1)
        perf_grid.attach(delay_sub, 1, 4, 2, 1)

        # --- Section 2: Mode & Efek Visual ---
        vis_frame = Gtk.Frame(label=" Mode Karakter & Efek Visual ")
        vis_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        vis_box.set_margin_top(8)
        vis_box.set_margin_bottom(8)
        vis_box.set_margin_start(10)
        vis_box.set_margin_end(10)
        vis_frame.add(vis_box)
        self.pack_start(vis_frame, False, False, 0)

        mode_row = Gtk.Box(spacing=10)
        mode_row.pack_start(Gtk.Label(label="Mode Encoding:", xalign=0), False, False, 0)
        self.mode_combo = Gtk.ComboBoxText()
        for key, label in MODE_CHOICES:
            self.mode_combo.append(key, label)
        self.mode_combo.set_active_id("matrix")
        mode_row.pack_start(self.mode_combo, True, True, 0)
        vis_box.pack_start(mode_row, False, False, 0)

        checks_grid = Gtk.Grid(column_spacing=16, row_spacing=4)
        self.chk_fog = Gtk.CheckButton(label="Efek Kabut (Fog)")
        self.chk_waves = Gtk.CheckButton(label="Gelombang Warna (Waves)")
        self.chk_rotate = Gtk.CheckButton(label="Rotasi / Panning 3D")
        self.chk_texture = Gtk.CheckButton(label="Tekstur Karakter")
        self.chk_wireframe = Gtk.CheckButton(label="Mode Wireframe")
        self.chk_fps = Gtk.CheckButton(label="Tampilkan FPS (--fps)")

        checks_grid.attach(self.chk_fog, 0, 0, 1, 1)
        checks_grid.attach(self.chk_waves, 1, 0, 1, 1)
        checks_grid.attach(self.chk_rotate, 0, 1, 1, 1)
        checks_grid.attach(self.chk_texture, 1, 1, 1, 1)
        checks_grid.attach(self.chk_wireframe, 0, 2, 1, 1)
        checks_grid.attach(self.chk_fps, 1, 2, 1, 1)
        vis_box.pack_start(checks_grid, False, False, 0)

        clock_row = Gtk.Box(spacing=8)
        self.chk_clock = Gtk.CheckButton(label="Tampilkan Jam (--clock)")
        self.chk_clock.connect("toggled", self._on_clock_toggled)
        self.entry_timefmt = Gtk.Entry()
        self.entry_timefmt.set_placeholder_text("Format waktu, mis.  %H:%M:%S ")
        self.entry_timefmt.set_tooltip_text("Format strftime untuk --timefmt (kosong = default glmatrix)")
        clock_row.pack_start(self.chk_clock, False, False, 0)
        clock_row.pack_start(self.entry_timefmt, True, True, 0)
        vis_box.pack_start(clock_row, False, False, 0)

        extra_row = Gtk.Box(spacing=8)
        extra_row.pack_start(Gtk.Label(label="Argumen Tambahan:", xalign=0), False, False, 0)
        self.entry_extra = Gtk.Entry()
        self.entry_extra.set_placeholder_text("Contoh: --mono")
        extra_row.pack_start(self.entry_extra, True, True, 0)
        vis_box.pack_start(extra_row, False, False, 0)

        for adj in (self.speed_adj, self.density_adj, self.delay_adj):
            adj.connect("value-changed", self._emit_change)
        self.mode_combo.connect("changed", self._emit_change)
        for chk in (
            self.chk_fog,
            self.chk_waves,
            self.chk_rotate,
            self.chk_texture,
            self.chk_wireframe,
            self.chk_fps,
            self.chk_clock,
        ):
            chk.connect("toggled", self._emit_change)
        self.entry_timefmt.connect("changed", self._emit_change)
        self.entry_extra.connect("changed", self._emit_change)

    def _emit_change(self, *_args) -> None:
        if not self._suppress and self._on_change is not None:
            self._on_change()

    def _update_fps_hint(self, *_args):
        delay = int(self.delay_adj.get_value())
        if delay <= 0:
            self.fps_hint.set_text("Estimasi: Maksimal (VSync)")
        else:
            fps = 1_000_000.0 / delay
            self.fps_hint.set_text(f"Estimasi: ~{fps:.1f} FPS")

    def _on_clock_toggled(self, *_args):
        self.entry_timefmt.set_sensitive(self.chk_clock.get_active())

    def set_values(self, opts: dict) -> None:
        self._suppress = True
        try:
            self.speed_adj.set_value(float(opts.get("speed", 1.0)))
            self.density_adj.set_value(int(opts.get("density", 20)))
            self.delay_adj.set_value(int(opts.get("delay", 35000)))
            self.mode_combo.set_active_id(opts.get("mode", "matrix"))

            self.chk_fog.set_active(not bool(opts.get("no_fog", False)))
            self.chk_waves.set_active(not bool(opts.get("no_waves", True)))
            self.chk_rotate.set_active(not bool(opts.get("no_rotate", True)))
            self.chk_texture.set_active(not bool(opts.get("no_texture", False)))
            self.chk_wireframe.set_active(bool(opts.get("wireframe", False)))
            self.chk_fps.set_active(bool(opts.get("show_fps", False)))
            self.chk_clock.set_active(bool(opts.get("clock", False)))
            self.entry_timefmt.set_text(str(opts.get("timefmt", "")))
            self._on_clock_toggled()

            extra = opts.get("extra_args", [])
            self.entry_extra.set_text(" ".join(shlex.quote(str(x)) for x in extra))
        finally:
            self._suppress = False
        self._emit_change()

    def get_values(self) -> dict:
        mode = self.mode_combo.get_active_id() or "matrix"
        extra_text = self.entry_extra.get_text().strip()
        try:
            extra_args = shlex.split(extra_text) if extra_text else []
        except ValueError:
            extra_args = extra_text.split()

        return {
            "delay": int(self.delay_adj.get_value()),
            "speed": round(float(self.speed_adj.get_value()), 2),
            "density": int(self.density_adj.get_value()),
            "mode": mode,
            "no_fog": not self.chk_fog.get_active(),
            "no_waves": not self.chk_waves.get_active(),
            "no_rotate": not self.chk_rotate.get_active(),
            "no_texture": not self.chk_texture.get_active(),
            "wireframe": self.chk_wireframe.get_active(),
            "binary": mode == "binary",
            "clock": self.chk_clock.get_active(),
            "timefmt": self.entry_timefmt.get_text(),
            "show_fps": self.chk_fps.get_active(),
            "extra_args": extra_args,
        }


class ConfigPanelWindow(Gtk.Window):
    def __init__(self):
        super().__init__(title="GLMatrix Live — Panel Konfigurasi GUI")
        self.set_default_size(940, 720)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_icon_name("preferences-desktop-wallpaper")

        self._updating_ui = False
        self._popout_proc = None
        self._bin_path = get_binary_path()
        self._cfg = load_config()
        self._xs_cfg = load_xscreensaver_settings()

        provider = Gtk.CssProvider()
        provider.load_from_data(APP_CSS)
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

        root_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        root_box.set_margin_top(12)
        root_box.set_margin_bottom(12)
        root_box.set_margin_start(12)
        root_box.set_margin_end(12)
        self.add(root_box)

        # --- Top Matrix Banner + Live Wallpaper Switch ---
        banner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        banner.get_style_context().add_class("matrix-banner")

        top_row = Gtk.Box(spacing=12)
        title_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        lbl_title = Gtk.Label(label="GLMatrix Live & Screensaver Control Panel", xalign=0)
        lbl_title.get_style_context().add_class("matrix-title")
        lbl_sub = Gtk.Label(
            label="Panel GUI On/Off Live Wallpaper, kecepatan glmatrix, dan setting standar XScreenSaver",
            xalign=0,
        )
        lbl_sub.get_style_context().add_class("matrix-subtitle")
        title_box.pack_start(lbl_title, False, False, 0)
        title_box.pack_start(lbl_sub, False, False, 0)

        switch_box = Gtk.Box(spacing=8)
        switch_box.set_valign(Gtk.Align.CENTER)
        lbl_switch = Gtk.Label(label="Live Wallpaper:")
        lbl_switch.get_style_context().add_class("matrix-subtitle")
        self.wallpaper_switch = Gtk.Switch()
        self.wallpaper_switch.set_valign(Gtk.Align.CENTER)
        self.wallpaper_switch.connect("notify::active", self._on_wallpaper_switch)
        switch_box.pack_start(lbl_switch, False, False, 0)
        switch_box.pack_start(self.wallpaper_switch, False, False, 0)

        top_row.pack_start(title_box, True, True, 0)
        top_row.pack_end(switch_box, False, False, 0)
        banner.pack_start(top_row, False, False, 0)

        status_row = Gtk.Box(spacing=8)
        self.lbl_daemon_status = Gtk.Label(xalign=0)
        self.lbl_wallpaper_status = Gtk.Label(xalign=0)
        status_row.pack_start(self.lbl_daemon_status, False, False, 0)
        status_row.pack_start(Gtk.Label(label="  |  "), False, False, 0)
        status_row.pack_start(self.lbl_wallpaper_status, True, True, 0)

        self.btn_daemon_toggle = Gtk.Button(label="Start Daemon")
        self.btn_daemon_toggle.connect("clicked", self._on_toggle_daemon)
        self.btn_lock = Gtk.Button(label="Kunci Layar")
        self.btn_lock.connect("clicked", self._on_lock_clicked)

        status_row.pack_end(self.btn_lock, False, False, 0)
        status_row.pack_end(self.btn_daemon_toggle, False, False, 0)
        banner.pack_start(status_row, False, False, 0)
        root_box.pack_start(banner, False, False, 0)

        # --- Main Split: Left Notebook Tabs | Right Embedded Live Preview ---
        main_paned = Gtk.Paned(orientation=Gtk.Orientation.HORIZONTAL)
        main_paned.set_position(580)
        root_box.pack_start(main_paned, True, True, 0)

        self.notebook = Gtk.Notebook()
        self.notebook.connect("switch-page", self._on_tab_switched)
        main_paned.pack1(self.notebook, resize=True, shrink=False)

        # Right side: Embedded Live Preview + Quick Info
        preview_panel = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        preview_panel.set_margin_start(10)
        self.preview_widget = EmbeddedMatrixPreview(self._get_glmatrix_bin)
        self.preview_widget.btn_popout.connect("clicked", self._on_popout_preview_clicked)
        preview_panel.pack_start(self.preview_widget, True, True, 0)

        preview_note = Gtk.Label(xalign=0)
        preview_note.set_line_wrap(True)
        preview_note.get_style_context().add_class("dim-label")
        preview_note.set_text(
            "Preview di atas menampilkan efek glmatrix secara langsung sesuai tab yang sedang dibuka."
        )
        preview_panel.pack_start(preview_note, False, False, 0)
        main_paned.pack2(preview_panel, resize=False, shrink=False)

        # === Tab 1: Desktop Live Wallpaper ===
        desktop_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        desktop_presets = Gtk.Box(spacing=6)
        desktop_presets.set_margin_top(8)
        desktop_presets.set_margin_start(12)
        desktop_presets.set_margin_end(12)
        btn_desk_calm = Gtk.Button(label="Preset Wallpaper Tenang")
        btn_desk_calm.connect("clicked", lambda _b: self.desktop_section.set_values(default_config()["desktop"]))
        btn_desk_std = Gtk.Button(label="Preset Standar GLMatrix")
        btn_desk_std.connect("clicked", lambda _b: self.desktop_section.set_values(standard_glmatrix_options()))
        desktop_presets.pack_start(btn_desk_calm, False, False, 0)
        desktop_presets.pack_start(btn_desk_std, False, False, 0)
        desktop_box.pack_start(desktop_presets, False, False, 0)

        self.desktop_section = MatrixOptionsSection(on_change=self._on_active_options_changed)
        desktop_box.pack_start(self.desktop_section, False, False, 0)

        desktop_scroll = Gtk.ScrolledWindow()
        desktop_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        desktop_scroll.add(desktop_box)
        self.notebook.append_page(desktop_scroll, Gtk.Label(label="Desktop Wallpaper"))

        # === Tab 2: Screensaver Standar (XScreenSaver + glmatrix) ===
        xs_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)

        xs_status_frame = Gtk.Frame(label=" XScreenSaver Standar (~/.xscreensaver) ")
        xs_grid = Gtk.Grid(column_spacing=10, row_spacing=8)
        xs_grid.set_margin_top(8)
        xs_grid.set_margin_bottom(8)
        xs_grid.set_margin_start(10)
        xs_grid.set_margin_end(10)
        xs_status_frame.set_margin_top(8)
        xs_status_frame.set_margin_start(12)
        xs_status_frame.set_margin_end(12)
        xs_status_frame.add(xs_grid)

        self.lbl_xs_status = Gtk.Label(xalign=0)
        xs_btn_row = Gtk.Box(spacing=6)
        self.btn_xs_daemon = Gtk.Button(label="Start XScreenSaver")
        self.btn_xs_daemon.connect("clicked", self._on_xs_toggle_daemon)
        self.btn_xs_test = Gtk.Button(label="Test Layar Penuh")
        self.btn_xs_test.connect("clicked", self._on_xs_activate)
        self.btn_xs_native = Gtk.Button(label="Buka xscreensaver-settings")
        self.btn_xs_native.connect("clicked", self._on_xs_open_native)
        xs_btn_row.pack_start(self.btn_xs_daemon, False, False, 0)
        xs_btn_row.pack_start(self.btn_xs_test, False, False, 0)
        xs_btn_row.pack_start(self.btn_xs_native, False, False, 0)

        xs_grid.attach(self.lbl_xs_status, 0, 0, 2, 1)
        xs_grid.attach(xs_btn_row, 0, 1, 2, 1)

        xs_grid.attach(Gtk.Label(label="Mode Screensaver:", xalign=0), 0, 2, 1, 1)
        self.xs_mode_combo = Gtk.ComboBoxText()
        for k, lbl in XSCREENSAVER_MODES:
            self.xs_mode_combo.append(k, lbl)
        self.xs_mode_combo.set_hexpand(True)
        xs_grid.attach(self.xs_mode_combo, 1, 2, 1, 1)

        xs_grid.attach(Gtk.Label(label="Aktif Setelah Idle (menit):", xalign=0), 0, 3, 1, 1)
        self.xs_timeout_spin = Gtk.SpinButton.new_with_range(1, 720, 1)
        xs_grid.attach(self.xs_timeout_spin, 1, 3, 1, 1)

        lock_xs_row = Gtk.Box(spacing=10)
        self.xs_chk_lock = Gtk.CheckButton(label="Kunci Layar saat Screensaver")
        self.xs_lock_timeout_spin = Gtk.SpinButton.new_with_range(0, 120, 1)
        lock_xs_row.pack_start(self.xs_chk_lock, False, False, 0)
        lock_xs_row.pack_start(Gtk.Label(label="Delay Kunci (mnt):"), False, False, 0)
        lock_xs_row.pack_start(self.xs_lock_timeout_spin, False, False, 0)
        xs_grid.attach(lock_xs_row, 0, 4, 2, 1)

        fx_xs_row = Gtk.Box(spacing=16)
        self.xs_chk_fade = Gtk.CheckButton(label="Efek Fade")
        self.xs_chk_dpms = Gtk.CheckButton(label="Hemat Daya Monitor (DPMS)")
        fx_xs_row.pack_start(self.xs_chk_fade, False, False, 0)
        fx_xs_row.pack_start(self.xs_chk_dpms, False, False, 0)
        xs_grid.attach(fx_xs_row, 0, 5, 2, 1)

        xs_action_row = Gtk.Box(spacing=6)
        btn_xs_copy_desk = Gtk.Button(label="Salin dari Desktop")
        btn_xs_copy_desk.connect("clicked", self._on_copy_desktop_to_xs)
        btn_xs_std = Gtk.Button(label="Preset Standar GLMatrix")
        btn_xs_std.connect("clicked", lambda _b: self.xs_matrix_section.set_values(standard_glmatrix_options()))
        btn_xs_apply = Gtk.Button(label="Simpan & Terapkan XScreenSaver")
        btn_xs_apply.get_style_context().add_class("suggested-action")
        btn_xs_apply.connect("clicked", self._on_xs_save_clicked)
        xs_action_row.pack_start(btn_xs_copy_desk, False, False, 0)
        xs_action_row.pack_start(btn_xs_std, False, False, 0)
        xs_action_row.pack_end(btn_xs_apply, False, False, 0)
        xs_grid.attach(xs_action_row, 0, 6, 2, 1)

        xs_box.pack_start(xs_status_frame, False, False, 0)
        self.xs_matrix_section = MatrixOptionsSection(on_change=self._on_active_options_changed)
        xs_box.pack_start(self.xs_matrix_section, False, False, 0)

        xs_scroll = Gtk.ScrolledWindow()
        xs_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        xs_scroll.add(xs_box)
        self.notebook.append_page(xs_scroll, Gtk.Label(label="Screensaver Standar"))

        # === Tab 3: Lockscreen (glmatrix-live) ===
        lock_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        lock_top = Gtk.Frame(label=" Pengaturan Kunci Layar glmatrix-live ")
        lock_grid = Gtk.Grid(column_spacing=10, row_spacing=8)
        lock_grid.set_margin_top(8)
        lock_grid.set_margin_bottom(8)
        lock_grid.set_margin_start(10)
        lock_grid.set_margin_end(10)
        lock_top.add(lock_grid)
        lock_top.set_margin_top(8)
        lock_top.set_margin_start(12)
        lock_top.set_margin_end(12)

        lock_grid.attach(Gtk.Label(label="Auto-Lock Idle (menit, 0 = mati):", xalign=0), 0, 0, 1, 1)
        self.auto_lock_spin = Gtk.SpinButton.new_with_range(0, 240, 1)
        lock_grid.attach(self.auto_lock_spin, 1, 0, 1, 1)

        lock_grid.attach(Gtk.Label(label="Teks Banner:", xalign=0), 0, 1, 1, 1)
        self.entry_banner = Gtk.Entry()
        self.entry_banner.set_hexpand(True)
        lock_grid.attach(self.entry_banner, 1, 1, 1, 1)

        btn_copy_desktop = Gtk.Button(label="Salin Efek Visual dari Tab Desktop")
        btn_copy_desktop.connect("clicked", self._on_copy_desktop_to_lock)
        lock_grid.attach(btn_copy_desktop, 0, 2, 2, 1)

        lock_box.pack_start(lock_top, False, False, 0)
        self.lock_section = MatrixOptionsSection(on_change=self._on_active_options_changed)
        lock_box.pack_start(self.lock_section, False, False, 0)

        lock_scroll = Gtk.ScrolledWindow()
        lock_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        lock_scroll.add(lock_box)
        self.notebook.append_page(lock_scroll, Gtk.Label(label="Lockscreen"))

        # === Tab 4: Sistem ===
        sys_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        sys_box.set_margin_top(10)
        sys_box.set_margin_bottom(10)
        sys_box.set_margin_start(12)
        sys_box.set_margin_end(12)

        sys_frame = Gtk.Frame(label=" Path & Integrasi Sistem ")
        sys_grid = Gtk.Grid(column_spacing=10, row_spacing=10)
        sys_grid.set_margin_top(10)
        sys_grid.set_margin_bottom(10)
        sys_grid.set_margin_start(12)
        sys_grid.set_margin_end(12)
        sys_frame.add(sys_grid)

        sys_grid.attach(Gtk.Label(label="Binary glmatrix:", xalign=0), 0, 0, 1, 1)
        self.entry_glmatrix_path = Gtk.Entry()
        self.entry_glmatrix_path.set_hexpand(True)
        btn_browse = Gtk.Button(label="Pilih...")
        btn_browse.connect("clicked", self._on_browse_glmatrix)
        sys_grid.attach(self.entry_glmatrix_path, 1, 0, 1, 1)
        sys_grid.attach(btn_browse, 2, 0, 1, 1)

        sys_grid.attach(Gtk.Label(label="PAM Service:", xalign=0), 0, 1, 1, 1)
        self.entry_pam = Gtk.Entry()
        sys_grid.attach(self.entry_pam, 1, 1, 2, 1)

        sys_grid.attach(Gtk.Label(label="Config Live Wallpaper:", xalign=0), 0, 2, 1, 1)
        self.entry_cfg_path = Gtk.Entry()
        self.entry_cfg_path.set_text(str(get_config_path()))
        self.entry_cfg_path.set_editable(False)
        sys_grid.attach(self.entry_cfg_path, 1, 2, 2, 1)

        sys_grid.attach(Gtk.Label(label="Config XScreenSaver:", xalign=0), 0, 3, 1, 1)
        self.entry_xs_path = Gtk.Entry()
        self.entry_xs_path.set_text(str(get_xscreensaver_path()))
        self.entry_xs_path.set_editable(False)
        sys_grid.attach(self.entry_xs_path, 1, 3, 2, 1)

        self.chk_autostart = Gtk.CheckButton(label="Aktifkan Autostart User (~/.config/autostart/glmatrix-live.desktop)")
        self.chk_autostart.connect("toggled", self._on_autostart_toggled)
        sys_grid.attach(self.chk_autostart, 0, 4, 3, 1)

        sys_box.pack_start(sys_frame, False, False, 0)
        self.notebook.append_page(sys_box, Gtk.Label(label="Sistem"))

        # --- Bottom Action Bar ---
        bottom_box = Gtk.Box(spacing=8)
        btn_reset = Gtk.Button(label="Reset Default")
        btn_reset.connect("clicked", self._on_reset_clicked)

        self.lbl_feedback = Gtk.Label(xalign=0)
        self.lbl_feedback.set_ellipsize(2)

        btn_save = Gtk.Button(label="Simpan Saja")
        btn_save.connect("clicked", lambda _b: self._save_only())

        btn_apply = Gtk.Button(label="Terapkan Semua")
        btn_apply.get_style_context().add_class("suggested-action")
        btn_apply.connect("clicked", lambda _b: self._apply_and_restart())

        btn_close = Gtk.Button(label="Tutup")
        btn_close.connect("clicked", lambda _b: self.close())

        bottom_box.pack_start(btn_reset, False, False, 0)
        bottom_box.pack_start(self.lbl_feedback, True, True, 6)
        bottom_box.pack_end(btn_close, False, False, 0)
        bottom_box.pack_end(btn_apply, False, False, 0)
        bottom_box.pack_end(btn_save, False, False, 0)
        root_box.pack_start(bottom_box, False, False, 0)

        self.connect("delete-event", self._on_window_close)

        self._load_cfg_into_ui(self._cfg)
        self._load_xs_into_ui(self._xs_cfg)
        self._refresh_status()
        self.preview_widget.update_options(self._get_active_preview_options(), immediate=True)
        GLib.timeout_add(1200, self._poll_status)

    def _get_glmatrix_bin(self) -> str:
        p = self.entry_glmatrix_path.get_text().strip()
        if p and os.path.exists(p):
            return p
        return default_glmatrix_path()

    def _get_active_preview_options(self, page_num: int | None = None) -> dict:
        if page_num is None:
            page_num = self.notebook.get_current_page()
        if page_num == 1:
            return self.xs_matrix_section.get_values()
        if page_num == 2:
            return self.lock_section.get_values()
        return self.desktop_section.get_values()

    def _on_tab_switched(self, _notebook, _page, page_num: int) -> None:
        self.preview_widget.update_options(self._get_active_preview_options(page_num), immediate=True)

    def _on_active_options_changed(self) -> None:
        if not self._updating_ui:
            self.preview_widget.update_options(self._get_active_preview_options(), immediate=False)

    def _load_cfg_into_ui(self, cfg: dict) -> None:
        self._updating_ui = True
        try:
            self.wallpaper_switch.set_active(bool(cfg["general"].get("desktop_enabled", True)))
            self.desktop_section.set_values(cfg["desktop"])
            self.auto_lock_spin.set_value(int(cfg["lockscreen"].get("auto_lock_mins", 5)))
            self.entry_banner.set_text(str(cfg["lockscreen"].get("banner_text", "MATRIX TERMINAL LOCKED")))
            self.lock_section.set_values(cfg["lockscreen"]["options"])
            self.entry_glmatrix_path.set_text(str(cfg["general"].get("glmatrix_path", "")))
            self.entry_pam.set_text(str(cfg["general"].get("pam_service", "xscreensaver")))
            self.chk_autostart.set_active(self._is_user_autostart_enabled())
        finally:
            self._updating_ui = False

    def _load_xs_into_ui(self, xs: dict) -> None:
        self._updating_ui = True
        try:
            self.xs_mode_combo.set_active_id(xs.get("mode", "one"))
            self.xs_timeout_spin.set_value(int(xs.get("timeout_mins", 5)))
            self.xs_chk_lock.set_active(bool(xs.get("lock", False)))
            self.xs_lock_timeout_spin.set_value(int(xs.get("lock_timeout_mins", 0)))
            self.xs_chk_fade.set_active(bool(xs.get("fade", True)))
            self.xs_chk_dpms.set_active(bool(xs.get("dpms_enabled", False)))
            self.xs_matrix_section.set_values(xs.get("glmatrix_options", standard_glmatrix_options()))
        finally:
            self._updating_ui = False

    def _collect_cfg_from_ui(self) -> dict:
        return {
            "general": {
                "glmatrix_path": self.entry_glmatrix_path.get_text().strip() or default_glmatrix_path(),
                "pam_service": self.entry_pam.get_text().strip() or default_pam_service(),
                "desktop_enabled": self.wallpaper_switch.get_active(),
            },
            "desktop": self.desktop_section.get_values(),
            "lockscreen": {
                "auto_lock_mins": int(self.auto_lock_spin.get_value()),
                "banner_text": self.entry_banner.get_text() or "MATRIX TERMINAL LOCKED",
                "options": self.lock_section.get_values(),
            },
        }

    def _collect_xs_from_ui(self) -> dict:
        return {
            "mode": self.xs_mode_combo.get_active_id() or "one",
            "timeout_mins": int(self.xs_timeout_spin.get_value()),
            "lock": self.xs_chk_lock.get_active(),
            "lock_timeout_mins": int(self.xs_lock_timeout_spin.get_value()),
            "fade": self.xs_chk_fade.get_active(),
            "dpms_enabled": self.xs_chk_dpms.get_active(),
            "selected": self._xs_cfg.get("selected", 0),
            "glmatrix_options": self.xs_matrix_section.get_values(),
        }

    def _set_feedback(self, text: str) -> None:
        self.lbl_feedback.set_text(text)

    def _poll_status(self) -> bool:
        self._refresh_status()
        return True

    def _is_xscreensaver_running(self) -> bool:
        if not shutil.which("xscreensaver-command"):
            return False
        res = subprocess.run(
            ["xscreensaver-command", "-version"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=0.5,
        )
        return res.returncode == 0

    def _refresh_status(self) -> None:
        ok, resp = send_daemon_socket("status-json", timeout=0.5)
        daemon_running = False
        wallpaper_active = False
        wallpaper_pid = None
        desktop_enabled = None

        if ok and resp.strip():
            try:
                data = json.loads(resp.strip())
                daemon_running = bool(data.get("daemon_running", False))
                wallpaper_active = bool(data.get("wallpaper_active", False))
                wallpaper_pid = data.get("wallpaper_pid")
                desktop_enabled = data.get("desktop_enabled")
            except json.JSONDecodeError:
                ok_legacy, resp_legacy = send_daemon_socket("status", timeout=0.5)
                if ok_legacy and "BERJALAN" in resp_legacy:
                    daemon_running = True
                    wallpaper_active = "Desktop Background: AKTIF" in resp_legacy

        if daemon_running:
            self.lbl_daemon_status.set_markup("<span foreground='#00e65c'><b>● Daemon: Aktif</b></span>")
            self.btn_daemon_toggle.set_label("Stop Daemon")
        else:
            self.lbl_daemon_status.set_markup("<span foreground='#e06c75'><b>○ Daemon: Nonaktif</b></span>")
            self.btn_daemon_toggle.set_label("Start Daemon")

        if wallpaper_active:
            pid_txt = f" (PID {wallpaper_pid})" if wallpaper_pid else ""
            self.lbl_wallpaper_status.set_markup(f"<span foreground='#00e65c'><b>● Wallpaper: Menyala{pid_txt}</b></span>")
        else:
            self.lbl_wallpaper_status.set_markup("<span foreground='#e5c07b'><b>○ Wallpaper: Mati</b></span>")

        if desktop_enabled is not None and not self._updating_ui:
            if self.wallpaper_switch.get_active() != bool(desktop_enabled):
                self._updating_ui = True
                try:
                    self.wallpaper_switch.set_active(bool(desktop_enabled))
                finally:
                    self._updating_ui = False

        # XScreenSaver status
        has_xs = shutil.which("xscreensaver") is not None
        if not has_xs:
            self.lbl_xs_status.set_markup("<span foreground='#e5c07b'>XScreenSaver tidak terpasang di sistem.</span>")
            self.btn_xs_daemon.set_sensitive(False)
            self.btn_xs_test.set_sensitive(False)
        elif self._is_xscreensaver_running():
            self.lbl_xs_status.set_markup("<span foreground='#00e65c'><b>● XScreenSaver Daemon: Aktif</b></span>")
            self.btn_xs_daemon.set_label("Stop XScreenSaver")
            self.btn_xs_daemon.set_sensitive(True)
            self.btn_xs_test.set_sensitive(True)
        else:
            self.lbl_xs_status.set_markup("<span foreground='#e06c75'><b>○ XScreenSaver Daemon: Nonaktif</b></span>")
            self.btn_xs_daemon.set_label("Start XScreenSaver")
            self.btn_xs_daemon.set_sensitive(True)
            self.btn_xs_test.set_sensitive(False)

        self.btn_xs_native.set_sensitive(shutil.which("xscreensaver-settings") is not None)

        if self._popout_proc is not None and self._popout_proc.poll() is not None:
            self._popout_proc = None
            self.preview_widget.btn_popout.set_label("Jendela Terpisah")

    def _start_daemon_bg(self) -> None:
        subprocess.Popen(
            [self._bin_path, "daemon"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )

    def _on_wallpaper_switch(self, switch: Gtk.Switch, _param) -> None:
        if self._updating_ui:
            return
        enabled = switch.get_active()
        cfg = self._collect_cfg_from_ui()
        cfg["general"]["desktop_enabled"] = enabled
        try:
            save_config(cfg)
        except Exception as exc:
            self._set_feedback(f"Gagal menyimpan: {exc}")
            return

        cmd = "wallpaper-on" if enabled else "wallpaper-off"
        ok, resp = send_daemon_socket(cmd, timeout=2.0)
        if ok:
            self._set_feedback(resp.strip() or ("Live wallpaper ON" if enabled else "Live wallpaper OFF"))
        elif enabled:
            self._start_daemon_bg()
            self._set_feedback("Daemon dijalankan & live wallpaper diaktifkan.")
        else:
            self._set_feedback("Live wallpaper dinonaktifkan (daemon tidak berjalan).")
        GLib.timeout_add(300, lambda: (self._refresh_status(), False)[1])

    def _on_toggle_daemon(self, _btn) -> None:
        ok, _ = send_daemon_socket("status-json", timeout=0.5)
        if ok:
            send_daemon_socket("stop", timeout=1.5)
            self._set_feedback("Daemon dihentikan.")
        else:
            self._start_daemon_bg()
            self._set_feedback("Daemon dijalankan di latar belakang.")
        GLib.timeout_add(400, lambda: (self._refresh_status(), False)[1])

    def _on_lock_clicked(self, _btn) -> None:
        subprocess.Popen([self._bin_path, "lock"], start_new_session=True)

    def _on_popout_preview_clicked(self, _btn) -> None:
        if self._popout_proc is not None and self._popout_proc.poll() is None:
            self._stop_popout()
            self._set_feedback("Jendela preview ditutup.")
            return

        glmatrix_bin = self._get_glmatrix_bin()
        opts = self._get_active_preview_options()
        args = [glmatrix_bin, "--window"] + build_glmatrix_cli_flags(opts, include_root=False)
        try:
            self._popout_proc = subprocess.Popen(args)
            self.preview_widget.btn_popout.set_label("Tutup Jendela")
            self._set_feedback("Preview jendela terpisah dibuka.")
        except Exception as exc:
            self._set_feedback(f"Gagal membuka preview: {exc}")

    def _stop_popout(self) -> None:
        if self._popout_proc is not None:
            if self._popout_proc.poll() is None:
                self._popout_proc.terminate()
                try:
                    self._popout_proc.wait(timeout=0.8)
                except subprocess.TimeoutExpired:
                    self._popout_proc.kill()
            self._popout_proc = None
            self.preview_widget.btn_popout.set_label("Jendela Terpisah")

    def _on_xs_toggle_daemon(self, _btn) -> None:
        if self._is_xscreensaver_running():
            subprocess.Popen(["xscreensaver-command", "-exit"])
            self._set_feedback("XScreenSaver daemon dihentikan.")
        else:
            subprocess.Popen(
                ["xscreensaver", "-no-splash"],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            self._set_feedback("XScreenSaver daemon dimulai.")
        GLib.timeout_add(500, lambda: (self._refresh_status(), False)[1])

    def _on_xs_activate(self, _btn) -> None:
        subprocess.Popen(["xscreensaver-command", "-activate"])

    def _on_xs_open_native(self, _btn) -> None:
        subprocess.Popen(["xscreensaver-settings"], start_new_session=True)

    def _on_copy_desktop_to_xs(self, _btn) -> None:
        self.xs_matrix_section.set_values(self.desktop_section.get_values())
        self._set_feedback("Pengaturan visual Desktop disalin ke XScreenSaver.")

    def _on_xs_save_clicked(self, _btn=None) -> bool:
        xs = self._collect_xs_from_ui()
        try:
            save_xscreensaver_settings(xs)
            if self._is_xscreensaver_running():
                subprocess.Popen(["xscreensaver-command", "-restart"])
            self._set_feedback("✓ Pengaturan standar XScreenSaver disimpan & diterapkan.")
            return True
        except Exception as exc:
            self._set_feedback(f"✗ Gagal menyimpan ~/.xscreensaver: {exc}")
            return False

    def _on_copy_desktop_to_lock(self, _btn) -> None:
        self.lock_section.set_values(self.desktop_section.get_values())
        self._set_feedback("Pengaturan visual Desktop disalin ke Lockscreen.")

    def _on_browse_glmatrix(self, _btn) -> None:
        dialog = Gtk.FileChooserDialog(
            title="Pilih Binary glmatrix",
            parent=self,
            action=Gtk.FileChooserAction.OPEN,
        )
        dialog.add_buttons(
            Gtk.STOCK_CANCEL, Gtk.ResponseType.CANCEL,
            Gtk.STOCK_OPEN, Gtk.ResponseType.OK,
        )
        cur = self.entry_glmatrix_path.get_text().strip()
        if cur and os.path.dirname(cur) and os.path.isdir(os.path.dirname(cur)):
            dialog.set_current_folder(os.path.dirname(cur))
        if dialog.run() == Gtk.ResponseType.OK:
            self.entry_glmatrix_path.set_text(dialog.get_filename())
        dialog.destroy()

    def _autostart_path(self) -> Path:
        return Path.home() / ".config" / "autostart" / "glmatrix-live.desktop"

    def _is_user_autostart_enabled(self) -> bool:
        path = self._autostart_path()
        if not path.exists():
            return False
        text = path.read_text(encoding="utf-8", errors="ignore")
        return "Hidden=true" not in text

    def _on_autostart_toggled(self, chk: Gtk.CheckButton) -> None:
        if self._updating_ui:
            return
        path = self._autostart_path()
        path.parent.mkdir(parents=True, exist_ok=True)
        enabled = chk.get_active()
        content = (
            "[Desktop Entry]\n"
            "Type=Application\n"
            "Name=GLMatrix Live Desktop\n"
            "Comment=Live Matrix Wallpaper & Lockscreen Daemon\n"
            f'Exec=sh -c "sleep 2 && {self._bin_path} daemon"\n'
            f"Hidden={'false' if enabled else 'true'}\n"
            "NoDisplay=false\n"
            f"X-GNOME-Autostart-enabled={'true' if enabled else 'false'}\n"
            "StartupNotify=false\n"
            "Terminal=false\n"
            "Icon=preferences-desktop-wallpaper\n"
            "Categories=Utility;Graphics;\n"
        )
        path.write_text(content, encoding="utf-8")
        self._set_feedback("Autostart user diperbarui.")

    def _on_reset_clicked(self, _btn) -> None:
        dialog = Gtk.MessageDialog(
            transient_for=self,
            flags=0,
            message_type=Gtk.MessageType.QUESTION,
            buttons=Gtk.ButtonsType.YES_NO,
            text="Kembalikan pengaturan ke nilai default?",
        )
        resp = dialog.run()
        dialog.destroy()
        if resp == Gtk.ResponseType.YES:
            self._load_cfg_into_ui(default_config())
            self.xs_matrix_section.set_values(standard_glmatrix_options())
            self.preview_widget.update_options(self._get_active_preview_options(), immediate=True)
            self._set_feedback("Pengaturan dikembalikan ke default (klik Terapkan untuk menyimpan).")

    def _save_only(self) -> bool:
        cfg = self._collect_cfg_from_ui()
        try:
            save_config(cfg)
            self._set_feedback("✓ Konfigurasi disimpan ke config.toml")
            return True
        except Exception as exc:
            self._set_feedback(f"✗ Gagal menyimpan: {exc}")
            return False

    def _apply_and_restart(self) -> None:
        if not self._save_only():
            return
        if get_xscreensaver_path().exists():
            self._on_xs_save_clicked()
        ok, resp = send_daemon_socket("restart", timeout=2.5)
        if ok:
            self._set_feedback("✓ " + (resp.strip() or "Konfigurasi diterapkan."))
        elif self.wallpaper_switch.get_active():
            self._start_daemon_bg()
            self._set_feedback("✓ Konfigurasi disimpan & daemon dijalankan.")
        else:
            self._set_feedback("✓ Konfigurasi disimpan (live wallpaper nonaktif).")
        GLib.timeout_add(350, lambda: (self._refresh_status(), False)[1])

    def _on_window_close(self, *_args) -> bool:
        self.preview_widget.stop()
        self._stop_popout()
        Gtk.main_quit()
        return False


def main() -> None:
    GLib.set_prgname("glmatrix-live-panel")
    win = ConfigPanelWindow()
    win.show_all()
    Gtk.main()


if __name__ == "__main__":
    main()
