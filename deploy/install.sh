#!/bin/bash
# HunterZero-Rust Installation Script
# Script de instalación para Raspberry Pi Zero W v1.1 con GPi CASE 2W

set -euo pipefail

APP_DIR="/opt/hunterzero-rust"
REPO_URL="https://github.com/gusslayer/HunterZeroR.git"
SERVICE_NAME="hunterzero.service"
BIN_NAME="hunterzero-rust"
BIN_DST="/usr/local/bin/${BIN_NAME}"
CONFIG_DST="/boot/firmware/config.txt"
CONFIG_SRC="$(dirname "$0")/config.txt"

log()  { echo "[HunterZero] $*"; }
warn() { echo "[HunterZero][WARN] $*" >&2; }
die()  { echo "[HunterZero][ERROR] $*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1
}

as_root() {
  if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

detect_jobs() {
  local mem_kb=1048576
  if [[ -r /proc/meminfo ]]; then
    mem_kb="$(awk '/MemTotal/ {print $2}' /proc/meminfo)"
  fi
  # Pi Zero W has ~512MB RAM; limit jobs to avoid OOM during rustc/LLVM.
  if [[ "${mem_kb}" -lt 900000 ]]; then
    echo 1
  else
    echo 2
  fi
}

ensure_rust() {
  if require_cmd cargo; then
    log "Rust/Cargo detectado: $(cargo --version)"
    return
  fi

  log "Cargo no encontrado. Instalando Rust (rustup, profile minimal)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal

  # shellcheck source=/dev/null
  if [[ -f "$HOME/.cargo/env" ]]; then
    . "$HOME/.cargo/env"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"

  require_cmd cargo || die "Cargo no quedó disponible tras instalar rustup. Abre una nueva shell o ejecuta: source \"$HOME/.cargo/env\""
  log "Rust instalado: $(cargo --version)"
}

write_display_config() {
  log "Configurando pantalla GPi CASE 2W..."
  local config_backup
  config_backup="/boot/firmware/config.txt.backup.$(date +%Y%m%d%H%M%S)"

  local block
  block=$(cat <<'EOF'

# Agregado por HunterZero-Rust installer

# --- Retroflag GPi CASE 2W Configuración unificada ---
dtoverlay=vc4-kms-dpi-generic
dtparam=hactive=640,hfp=70,hsync=40,hbp=22
dtparam=vactive=480,vfp=18,vsync=9,vbp=18
dtparam=clock-frequency=24000000,rgb666-padhi
dtparam=width-mm=60,height-mm=45
EOF
)

  if [[ -f "$CONFIG_DST" ]]; then
    as_root cp "$CONFIG_DST" "$config_backup"
    log "Backup de config.txt creado: $config_backup"
    if grep -q "dtoverlay=vc4-kms-dpi-generic" "$CONFIG_DST" 2>/dev/null; then
      log "Configuración de pantalla ya presente, omitiendo..."
    else
      printf '%s\n' "$block" | as_root tee -a "$CONFIG_DST" >/dev/null
      log "Configuración de pantalla agregada a config.txt existente"
    fi
  else
    log "Creando nuevo config.txt..."
    if [[ -f "$CONFIG_SRC" ]]; then
      as_root cp "$CONFIG_SRC" "$CONFIG_DST"
    else
      as_root tee "$CONFIG_DST" >/dev/null <<'EOF'
# HunterZero-Rust - Raspberry Pi Zero W v1.1 + GPi CASE 2W

# --- Retroflag GPi CASE 2W Configuración unificada ---
dtoverlay=vc4-kms-dpi-generic
dtparam=hactive=640,hfp=70,hsync=40,hbp=22
dtparam=vactive=480,vfp=18,vsync=9,vbp=18
dtparam=clock-frequency=24000000,rgb666-padhi
dtparam=width-mm=60,height-mm=45
EOF
    fi
    log "config.txt creado en $CONFIG_DST"
  fi
}

ensure_repo() {
  log "Configurando directorio de la aplicación..."
  if [[ ! -d "$APP_DIR/.git" ]]; then
    if [[ -d "$APP_DIR" ]] && [[ -n "$(ls -A "$APP_DIR" 2>/dev/null || true)" ]]; then
      warn "$APP_DIR existe pero no es un repo git. Respaldando..."
      as_root mv "$APP_DIR" "${APP_DIR}.bak.$(date +%Y%m%d%H%M%S)"
    fi
    as_root mkdir -p "$APP_DIR"
    as_root chown "$(id -u):$(id -g)" "$APP_DIR"
    git clone "$REPO_URL" "$APP_DIR"
  else
    log "Repo existente detectado, actualizando..."
    git -C "$APP_DIR" pull --ff-only || warn "No se pudo hacer git pull --ff-only; continuando con el código local"
  fi
}

run_tests() {
  if [[ "${SKIP_TESTS:-0}" == "1" ]]; then
    warn "SKIP_TESTS=1: omitiendo pruebas."
    return
  fi
  log "Ejecutando pruebas (cargo test)..."
  cargo test --all-targets
  log "Pruebas OK."
}

build_release() {
  local jobs
  jobs="$(detect_jobs)"
  log "Compilando en modo release (jobs=${jobs})..."
  # Limitar jobs ayuda a evitar OOM en Pi Zero W.
  CARGO_BUILD_JOBS="${jobs}" cargo build --release -j "${jobs}"
  [[ -f "$APP_DIR/target/release/${BIN_NAME}" ]] || die "No se encontró el binario compilado en target/release/${BIN_NAME}"
  log "Compilación OK."
}

install_binary() {
  log "Instalando binario..."
  as_root systemctl stop "$SERVICE_NAME" 2>/dev/null || true
  as_root cp "$APP_DIR/target/release/${BIN_NAME}" "$BIN_DST"
  as_root chmod +x "$BIN_DST"
  log "Binario instalado en $BIN_DST"
}

install_service() {
  log "Configurando servicio systemd..."
  as_root tee "/etc/systemd/system/${SERVICE_NAME}" >/dev/null <<EOF
[Unit]
Description=HunterZero-Rust Application
After=network.target

[Service]
ExecStart=${BIN_DST}
Restart=on-failure
WorkingDirectory=${APP_DIR}
User=root

[Install]
WantedBy=multi-user.target
EOF
  as_root systemctl daemon-reload
  as_root systemctl enable "$SERVICE_NAME"
  log "Servicio ${SERVICE_NAME} habilitado."
}

main() {
  log "=== HunterZero-Rust Installation ==="

  log "Actualizando índices de paquetes..."
  as_root apt-get update

  log "Instalando dependencias del sistema..."
  as_root apt-get install -y --no-install-recommends \
    git \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    nmap \
    dnsutils \
    bluez \
    iputils-ping

  write_display_config

  # GPIO shutdown note: gpio-shutdown overlay is intentionally not added here.
  log "Apagado GPIO gestionado por hunterzero-poweroff.service (sin dtoverlay=gpio-shutdown)."

  ensure_repo
  ensure_rust

  cd "$APP_DIR"

  # Make sure cargo env is available in this shell.
  if ! require_cmd cargo && [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"
  require_cmd cargo || die "cargo no disponible"

  log "Toolchain: $(rustc --version 2>/dev/null || echo desconocido) / $(cargo --version)"

  run_tests
  build_release
  install_binary
  install_service

  echo ""
  log "=== Instalación Completa ==="
  echo "1. Reinicia la Raspberry Pi: sudo reboot"
  echo "2. Verificar estado: systemctl status ${SERVICE_NAME}"
  echo "3. Ver logs: journalctl -u ${SERVICE_NAME} -f"
  echo ""
  echo "Notas:"
  echo " - Si cambiaste config.txt, el reinicio es necesario para aplicar pantalla."
  echo " - En Pi Zero W la compilación release puede tardar bastante."
  echo " - Para omitir pruebas: SKIP_TESTS=1 sudo -E bash deploy/install.sh"
}

main "$@"
