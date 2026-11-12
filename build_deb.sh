#!/usr/bin/env bash
set -e

# Script untuk membuat file installer .deb untuk glmatrix-live

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PACKAGE_NAME="glmatrix-live"
VERSION="1.0.0"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo "amd64")"
DEB_DIR="target/debian"
PACKAGE_ROOT="$DEB_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}"
OUTPUT_DEB="${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"

echo "=================================================="
echo "    Membangun Paket .deb untuk $PACKAGE_NAME     "
echo "=================================================="

# 1. Pastikan binary rilis telah dikompilasi
if [ ! -f "target/release/glmatrix-live" ]; then
    echo "Mengompilasi binary release..."
    cargo build --release
fi

# 2. Siapkan direktori paket
echo "[1/4] Menyiapkan struktur direktori Debian..."
rm -rf "$PACKAGE_ROOT"
mkdir -p "$PACKAGE_ROOT/DEBIAN"
mkdir -p "$PACKAGE_ROOT/opt/glmatrix-live"
mkdir -p "$PACKAGE_ROOT/usr/bin"
mkdir -p "$PACKAGE_ROOT/etc/xdg/autostart"
mkdir -p "$PACKAGE_ROOT/usr/share/applications"
mkdir -p "$PACKAGE_ROOT/usr/share/doc/$PACKAGE_NAME"

# 3. Salin binary & symlink
echo "[2/4] Menyalin binary dan aset desktop..."
cp "target/release/glmatrix-live" "$PACKAGE_ROOT/opt/glmatrix-live/glmatrix-live"
chmod 755 "$PACKAGE_ROOT/opt/glmatrix-live/glmatrix-live"

# Symlink di /usr/bin/glmatrix-live
ln -sf "/opt/glmatrix-live/glmatrix-live" "$PACKAGE_ROOT/usr/bin/glmatrix-live"

# Autostart saat login
cat << 'EOF' > "$PACKAGE_ROOT/etc/xdg/autostart/glmatrix-live.desktop"
[Desktop Entry]
Type=Application
Name=GLMatrix Live Desktop
Comment=Live Matrix Wallpaper & Lockscreen Daemon
Exec=sh -c "sleep 2 && /usr/bin/glmatrix-live daemon"
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
StartupNotify=false
Terminal=false
Icon=preferences-desktop-wallpaper
Categories=Utility;Graphics;
EOF
chmod 644 "$PACKAGE_ROOT/etc/xdg/autostart/glmatrix-live.desktop"

# Desktop menu entry (membuka Panel Konfigurasi)
cat << 'EOF' > "$PACKAGE_ROOT/usr/share/applications/glmatrix-live.desktop"
[Desktop Entry]
Type=Application
Name=GLMatrix Live Settings
GenericName=Live Wallpaper Configuration
Comment=Panel Konfigurasi Live Matrix Wallpaper & Lockscreen
Exec=/usr/bin/glmatrix-live panel
Icon=preferences-desktop-wallpaper
Terminal=false
Categories=Utility;DesktopSettings;Settings;
Keywords=matrix;wallpaper;screensaver;lockscreen;settings;config;
EOF
chmod 644 "$PACKAGE_ROOT/usr/share/applications/glmatrix-live.desktop"

# Dokumentasi singkat
cat << EOF > "$PACKAGE_ROOT/usr/share/doc/$PACKAGE_NAME/copyright"
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: glmatrix-live
Source: local

Files: *
Copyright: 2026 FJ <fj@local>
License: MIT
EOF

# 4. Metadata kontrol DEBIAN
echo "[3/4] Menulis file DEBIAN control & scripts..."
INSTALLED_SIZE=$(du -sk "$PACKAGE_ROOT" | cut -f1)

cat << EOF > "$PACKAGE_ROOT/DEBIAN/control"
Package: $PACKAGE_NAME
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_SIZE
Depends: libc6, libpam0g, xscreensaver-gl, python3, python3-gi, gir1.2-gtk-3.0
Maintainer: FJ <fj@local>
Description: Native Matrix GL Live Wallpaper and Lockscreen Manager
 Lightweight native desktop live wallpaper and lockscreen manager for X11
 featuring OpenGL Matrix rain effect with PAM authentication and daemon control.
 Automatically starts on user login.
EOF

# DEBIAN/postinst script
cat << 'EOF' > "$PACKAGE_ROOT/DEBIAN/postinst"
#!/bin/sh
set -e

if [ -x /usr/bin/update-desktop-database ]; then
    /usr/bin/update-desktop-database -q || true
fi

echo "=========================================================="
echo "  glmatrix-live berhasil diinstall!"
echo "  - Binary: /opt/glmatrix-live/glmatrix-live"
echo "  - Command: glmatrix-live"
echo "  - Autostart login telah diaktifkan di /etc/xdg/autostart/"
echo "=========================================================="

exit 0
EOF
chmod 755 "$PACKAGE_ROOT/DEBIAN/postinst"

# DEBIAN/prerm script
cat << 'EOF' > "$PACKAGE_ROOT/DEBIAN/prerm"
#!/bin/sh
set -e

# Hentikan daemon jika sedang berjalan saat uninstall/upgrade
if [ -x /usr/bin/glmatrix-live ]; then
    /usr/bin/glmatrix-live stop 2>/dev/null || true
fi
killall -q glmatrix-live 2>/dev/null || true

exit 0
EOF
chmod 755 "$PACKAGE_ROOT/DEBIAN/prerm"

# DEBIAN/postrm script
cat << 'EOF' > "$PACKAGE_ROOT/DEBIAN/postrm"
#!/bin/sh
set -e

if [ -x /usr/bin/update-desktop-database ]; then
    /usr/bin/update-desktop-database -q || true
fi

exit 0
EOF
chmod 755 "$PACKAGE_ROOT/DEBIAN/postrm"

# 5. Bangun file .deb
echo "[4/4] Memaketkan menjadi $OUTPUT_DEB..."
dpkg-deb --build --root-owner-group "$PACKAGE_ROOT" "$OUTPUT_DEB"

echo "=================================================="
echo "✓ Berhasil dibuat: $OUTPUT_DEB"
echo "  Ukuran file : $(ls -lh "$OUTPUT_DEB" | awk '{print $5}')"
echo ""
echo "Cara install di sistem lain / fresh install:"
echo "  sudo apt install ./$OUTPUT_DEB"
echo "  # atau:"
echo "  sudo dpkg -i $OUTPUT_DEB && sudo apt-get install -f"
echo "=================================================="
