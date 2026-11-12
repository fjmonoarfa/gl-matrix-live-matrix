#!/usr/bin/env bash
set -e

# Installer script for glmatrix-live
# Installs binary to /opt/glmatrix-live and symlinks to /usr/local/bin

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_SRC="$SCRIPT_DIR/target/release/glmatrix-live"
OPT_DIR="/opt/glmatrix-live"
BIN_TARGET="/usr/local/bin/glmatrix-live"
XDG_AUTOSTART="/etc/xdg/autostart/glmatrix-live.desktop"
APP_DESKTOP="/usr/share/applications/glmatrix-live.desktop"

if [ "$EUID" -ne 0 ]; then
    echo "Installer ini membutuhkan akses root. Menjalankan sudo..."
    exec sudo "$0" "$@"
fi

echo "=================================================="
echo "    Menginstall glmatrix-live ke Root System     "
echo "=================================================="

if [ ! -f "$BINARY_SRC" ]; then
    echo "Error: Binary tidak ditemukan di $BINARY_SRC" >&2
    echo "Silakan jalankan 'cargo build --release' terlebih dahulu." >&2
    exit 1
fi

echo "[1/4] Membuat direktori $OPT_DIR..."
mkdir -p "$OPT_DIR"

echo "[2/4] Menyalin binary glmatrix-live ke $OPT_DIR/glmatrix-live..."
cp -f "$BINARY_SRC" "$OPT_DIR/glmatrix-live"
chmod 755 "$OPT_DIR/glmatrix-live"
chown root:root "$OPT_DIR/glmatrix-live"

echo "[3/4] Membuat symbolic link di $BIN_TARGET..."
ln -sf "$OPT_DIR/glmatrix-live" "$BIN_TARGET"

echo "[4/4] Memasang autostart saat login ke $XDG_AUTOSTART..."
mkdir -p /etc/xdg/autostart
cat << 'EOF' > "$XDG_AUTOSTART"
[Desktop Entry]
Type=Application
Name=GLMatrix Live Desktop
Comment=Live Matrix Wallpaper & Lockscreen Daemon
Exec=sh -c "sleep 2 && /usr/local/bin/glmatrix-live daemon"
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
StartupNotify=false
Terminal=false
Icon=preferences-desktop-wallpaper
Categories=Utility;Graphics;
EOF
chmod 644 "$XDG_AUTOSTART"

# Menu aplikasi desktop (membuka Panel Konfigurasi)
mkdir -p /usr/share/applications
cat << 'EOF' > "$APP_DESKTOP"
[Desktop Entry]
Type=Application
Name=GLMatrix Live Settings
GenericName=Live Wallpaper Configuration
Comment=Panel Konfigurasi Live Matrix Wallpaper & Lockscreen
Exec=/usr/local/bin/glmatrix-live panel
Icon=preferences-desktop-wallpaper
Terminal=false
Categories=Utility;DesktopSettings;Settings;
Keywords=matrix;wallpaper;screensaver;lockscreen;settings;config;
EOF
chmod 644 "$APP_DESKTOP"

echo "=================================================="
echo "✓ Instalasi Berhasil!"
echo "  - Binary: $OPT_DIR/glmatrix-live"
echo "  - Symlink: $BIN_TARGET"
echo "  - Autostart Login: $XDG_AUTOSTART"
echo "  - Application Entry: $APP_DESKTOP"
echo "=================================================="
