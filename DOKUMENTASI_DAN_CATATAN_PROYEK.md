# Dokumentasi & Riwayat Sesi: glmatrix-live

**Tanggal:** 29 Oktober 2026  
**Penulis/Pengguna:** fj  
**Lokasi Proyek:** `/home/fj/my_projects/desk_apps/glmatrix-live`

---

## 1. Ringkasan Proyek

`glmatrix-live` adalah daemon wallpaper hidup (*live desktop wallpaper*) dan pengunci layar (*lockscreen*) bergaya Matrix GL untuk lingkungan desktop Linux berbasis X11 (seperti Cinnamon, GNOME X11, MATE, XFCE). Aplikasi ini ditulis dalam bahasa pemrograman Rust dengan pustaka native `x11rb`.

### Fitur Utama:
- **Desktop Live Wallpaper:** Menjalankan `glmatrix` di jendela background khusus bertipe `_NET_WM_WINDOW_TYPE_DESKTOP` yang disematkan di bawah seluruh jendela aplikasi (`_NET_WM_STATE_BELOW`) dan transparan terhadap input mouse.
- **Auto-Crash Recovery:** Memantau proses child `glmatrix`. Jika sesi jendela X11 terganggu atau crash, daemon secara otomatis membuat ulang sesi desktop tanpa loop error `BadWindow`.
- **PAM Lockscreen:** Penguncian layar interaktif dengan efek Matrix dan autentikasi PAM Linux (`/etc/pam.d/xscreensaver`).
- **Autostart saat Login:** Terintegrasi penuh dengan XDG Autostart dan systemd user service.

---

## 2. Struktur Instalasi Sistem

| Komponen | Path di Sistem | Keterangan |
| :--- | :--- | :--- |
| **Binary Utama** | `/opt/glmatrix-live/glmatrix-live` | Binary rilis Rust (permissions `755 root:root`) |
| **Global Symlink** | `/usr/local/bin/glmatrix-live` & `/usr/bin/glmatrix-live` | Memastikan perintah `glmatrix-live` dapat diakses langsung dari shell |
| **System Autostart** | `/etc/xdg/autostart/glmatrix-live.desktop` | Menjalankan daemon otomatis dengan delay 2 detik saat login |
| **User Autostart** | `~/.config/autostart/glmatrix-live.desktop` | Cadangan autostart untuk pengguna |
| **Systemd Service** | `~/.config/systemd/user/glmatrix-live.service` | Unit systemd untuk memonitor daemon di tingkat sesi user |
| **Menu Desktop** | `/usr/share/applications/glmatrix-live.desktop` | Pintasan di Application Menu |
| **File Konfigurasi**| `~/.config/glmatrix-live/config.toml` | Mengatur delay, speed, density, rotate, waves, dll. |

---

## 3. Skrip yang Tersedia di Direktori Proyek

1. **`install.sh`**
   - Skrip bash untuk menginstal/memperbarui binary ke `/opt/glmatrix-live` dan mengonfigurasi autostart root.
   - Cara pakai: `sudo ./install.sh`

2. **`build_deb.sh`**
   - Skrip untuk memaketkan ulang aplikasi ke format paket Debian (`.deb`).
   - Output: `glmatrix-live_1.0.0_amd64.deb`
   - Cara pakai: `./build_deb.sh`

---

## 4. Perintah Operasional

```bash
# Cek status daemon dan desktop wallpaper
glmatrix-live status

# Kunci layar secara manual dengan efek Matrix
glmatrix-live lock

# Restart wallpaper & muat ulang konfigurasi
glmatrix-live restart

# Hentikan daemon
glmatrix-live stop

# Jalankan daemon di latar belakang secara manual
glmatrix-live daemon
```

---

## 5. Catatan Porting ke Windows (.exe)

`glmatrix-live` versi ini tidak dapat di-compile langsung menjadi file `.exe` untuk Windows karena:
1. Bergantung pada protokol **X11** (`x11rb`) yang tidak ada di Windows (Windows menggunakan Win32 DWM).
2. Bergantung pada binary eksternal Linux **`/usr/libexec/xscreensaver/glmatrix`**.
3. Bergantung pada **Linux PAM** (`libpam.so`) untuk modul autentikasi password.
4. Bergantung pada POSIX signals dan Unix Domain Sockets.

### Solusi untuk Windows:
- Gunakan arsitektur Win32 hook **`WorkerW`** (`SendMessageTimeout` ke `Progman` dengan pesan `0x052C`) untuk merender di belakang ikon desktop.
- Menggunakan renderer OpenGL/wgpu mandiri tanpa subprocess eksternal.
- Alternatif instan: Gunakan **Lively Wallpaper** (Open Source di Windows) yang sudah memiliki tema Matrix bawaan.
