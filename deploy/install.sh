#!/bin/bash
# HunterZero-Rust Installation Script
# Script de instalación para Raspberry Pi Zero W v1.1 con GPi CASE 2W

set -e

echo "=== HunterZero-Rust Installation ==="

# 1. Actualizar sistema y instalar dependencias
echo "Actualizando sistema..."
sudo apt-get update
sudo apt-get upgrade -y

# 2. Instalar dependencias del sistema para módulos
echo "Instalando dependencias del sistema..."
sudo apt-get install -y \
  git \
  curl \
  ca-certificates \
  build-essential \
  pkg-config \
  nmap \
  dnsutils \
  bluez

# 3. Configurar config.txt para GPi CASE 2W
echo "Configurando pantalla GPi CASE 2W..."

CONFIG_SRC="$(dirname "$0")/config.txt"
CONFIG_DST="/boot/firmware/config.txt"
CONFIG_BACKUP="/boot/firmware/config.txt.backup.$(date +%Y%m%d%H%M%S)"

# Verificar si el archivo ya existe
if [ -f "$CONFIG_DST" ]; then
    # Guardar backup de la configuración existente
    sudo cp "$CONFIG_DST" "$CONFIG_BACKUP"
    echo "   - Backup de config.txt existente creado"
    
    # Verificar si ya tiene la configuración de pantalla
    if grep -q "dtoverlay=vc4-kms-dpi-generic" "$CONFIG_DST" 2>/dev/null; then
        echo "   - Configuración de pantalla ya presente, omitiendo..."
    else
        # Agregar configuración de pantalla al final del archivo existente
        sudo tee -a "$CONFIG_DST" >/dev/null << 'EOF'

# Agregado por HunterZero-Rust installer

# --- Retroflag GPi CASE 2W Configuración unificada ---
dtoverlay=vc4-kms-dpi-generic
dtparam=hactive=640,hfp=70,hsync=40,hbp=22
dtparam=vactive=480,vfp=18,vsync=9,vbp=18
dtparam=clock-frequency=24000000,rgb666-padhi
dtparam=width-mm=60,height-mm=45
EOF
        echo "   - Configuración de pantalla agregada a config.txt existente"
    fi
else
    # Crear nuevo archivo config.txt (primera instalación)
    echo "   - Creando nuevo config.txt..."
    if [ -f "$CONFIG_SRC" ]; then
        sudo cp "$CONFIG_SRC" "$CONFIG_DST"
    else
        sudo tee "$CONFIG_DST" >/dev/null << 'EOF'
# HunterZero-Rust - Raspberry Pi Zero W v1.1 + GPi CASE 2W

# --- Retroflag GPi CASE 2W Configuración unificada ---
dtoverlay=vc4-kms-dpi-generic
dtparam=hactive=640,hfp=70,hsync=40,hbp=22
dtparam=vactive=480,vfp=18,vsync=9,vbp=18
dtparam=clock-frequency=24000000,rgb666-padhi
dtparam=width-mm=60,height-mm=45
EOF
    fi
    echo "   - config.txt creado en $CONFIG_DST"
fi

# 4. Configurar GPIO para apagado seguro
echo "Configurando apagado GPIO..."
# NOTA: No se agrega dtoverlay=gpio-shutdown aquí porque esto reclamaría
# las líneas GPIO 26/27 y evitaría que hunterzero-poweroff.service las controle.
# El servicio hunterzero-poweroff.sh ya maneja el botón físico en GPIO 26 adecuadamente.

# 5. Clonar o asegurar repositorio
echo "Configurando directorio de la aplicación..."
APP_DIR="/opt/hunterzero-rust"
if [ ! -d "$APP_DIR" ]; then
    sudo mkdir -p "$APP_DIR"
    sudo chown "$(id -u):$(id -g)" "$APP_DIR"
    git clone https://github.com/gusslayer/HunterZeroR.git "$APP_DIR"
else
    cd "$APP_DIR"
    git pull
fi

# 6. Compilar en modo release
echo "Compilando aplicación en modo release..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo no encontrado. Instalando Rust (rustup)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env"
fi
cd "$APP_DIR"
cargo build --release

# 7. Instalar binario
echo "Instalando binario..."
sudo cp "$APP_DIR/target/release/hunterzero-rust" /usr/local/bin/hunterzero-rust

# 8. Configurar servicio systemd
echo "Configurando servicio systemd..."
sudo bash -c 'cat > /etc/systemd/system/hunterzero.service << EOF
[Unit]
Description=HunterZero-Rust Application
After=network.target

[Service]
ExecStart=/usr/local/bin/hunterzero-rust
Restart=on-failure
WorkingDirectory=/opt/hunterzero-rust
User=root

[Install]
WantedBy=multi-user.target
EOF'

sudo systemctl daemon-reload
sudo systemctl enable hunterzero.service

# 9. Mens finales
echo ""
echo "=== Instalación Completa ==="
echo "1. Reinicia la Raspberry Pi: sudo reboot"
echo "2. Verificar estado: systemctl status hunterzero.service"
echo "3. Acceder a la interfaz mediante la pantalla GPIO"
echo ""
echo "IMPORTANTE:"
echo " - Si es la primera vez que enciendes, el config.txt se creará automáticamente"
echo " - Si ya tenía config.txt, se respaldó y se agregaron las líneas necesarias"
echo " - El apagado seguro usa GPIO 26 con el botón físico"
echo " - Las primeras iniciaciones pueden tardar extra en mostrar video"