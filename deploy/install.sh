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
SWAP_FILE="/var/swap-hunterzero"

log()  { echo "[HunterZero] $*"; }
warn() { echo "[HunterZero][WARN] $*" >&2; }
die()  { echo "[HunterZero][ERROR] $*" >&2; exit 1; }

require_cmd() { command -v "$1" >/dev/null 2>&1; }

as_root() {
  if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

mem_total_kb() {
  if [[ -r /proc/meminfo ]]; then
    awk '/MemTotal/ {print $2}' /proc/meminfo
  else
    echo 1048576
  fi
}

detect_jobs() {
  local mem_kb
  mem_kb="$(mem_total_kb)"
  if [[ "${mem_kb}" -lt 900000 ]]; then
    echo 1
  else
    echo 2
  fi
}

ensure_swap_if_low_mem() {
  local mem_kb
  mem_kb="$(mem_total_kb)"
  [[ "${mem_kb}" -lt 900000 ]] || return 0

  if swapon --show 2>/dev/null | grep -q .; then
    log "Swap ya activo."
    return 0
  fi
  if [[ -f "$SWAP_FILE" ]]; then
    log "Activando swap existente en $SWAP_FILE..."
    as_root chmod 600 "$SWAP_FILE" || true
    as_root mkswap "$SWAP_FILE" >/dev/null 2>&1 || true
    as_root swapon "$SWAP_FILE" || warn "No se pudo activar swap existente."
    return 0
  fi

  log "Memoria baja detectada ($((${mem_kb}/1024))MB). Creando swap temporal de 1G para compilar..."
  if as_root fallocate -l 1G "$SWAP_FILE" 2>/dev/null || as_root dd if=/dev/zero of="$SWAP_FILE" bs=1M count=1024 status=none; then
    as_root chmod 600 "$SWAP_FILE"
    as_root mkswap "$SWAP_FILE" >/dev/null
    as_root swapon "$SWAP_FILE" || warn "No se pudo activar swap; la compilación puede fallar por OOM."
  else
    warn "No se pudo crear swap temporal; la compilación puede fallar por OOM."
  fi
}

cleanup_temp_swap() {
  if swapon --show 2>/dev/null | grep -q "$SWAP_FILE"; then
    log "Desactivando swap temporal..."
    as_root swapoff "$SWAP_FILE" || true
  fi
  [[ -f "$SWAP_FILE" ]] && as_root rm -f "$SWAP_FILE" || true
}

ensure_rust() {
  if ! require_cmd cargo && [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
  fi

  if require_cmd cargo; then
    log "Rust/Cargo detectado: $(cargo --version)"
  else
    log "Cargo no encontrado. Instalando Rust (rustup, profile minimal)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck source=/dev/null
    [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
  fi

  require_cmd cargo || die "Cargo no quedó disponible. Ejecuta: source \"$HOME/.cargo/env\""
  require_cmd rustc || die "rustc no quedó disponible tras instalar Rust."
  if require_cmd rustup; then
    rustup default stable >/dev/null 2>&1 || warn "No se pudo fijar toolchain stable (continuando)."
  fi
  log "Toolchain: $(rustc --version) / $(cargo --version)"
}

write_display_config() {
  log "Configurando pantalla GPi CASE 2W..."
  local config_backup="/boot/firmware/config.txt.backup.$(date +%Y%m%d%H%M%S)"
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

cargo_with_jobs() {
  local jobs="$1"; shift
  CARGO_BUILD_JOBS="$jobs" cargo "$@" -j "$jobs"
}

run_preflight_checks() {
  local jobs
  jobs="$(detect_jobs)"
  log "Preflight: verificando código antes de compilar release..."

  # Comprobación rápida de compilación (debug/check) para detectar errores temprano.
  if cargo_with_jobs "$jobs" check --all-targets; then
    log "Preflight check OK."
  else
    warn "cargo check falló. Intentando autocorrección (git pull --ff-only) y reintento..."
    git -C "$APP_DIR" pull --ff-only || true
    cargo_with_jobs "$jobs" check --all-targets || die "El código no compila (cargo check). Corrige los errores antes de continuar."
    log "Preflight check OK tras reintento."
  fi

  # Gates opcionales si las herramientas están instaladas.
  if cargo fmt --version >/dev/null 2>&1; then
    log "Preflight: cargo fmt --check..."
    cargo fmt --all -- --check || warn "Formato no cumple cargo fmt (no bloqueante en Pi, pero conviene corregirlo)."
  else
    warn "rustfmt/cargo-fmt no disponible; omitiendo cargo fmt --check."
  fi

  if cargo clippy --version >/dev/null 2>&1; then
    log "Preflight: cargo clippy..."
    cargo_with_jobs "$jobs" clippy --all-targets -- -D warnings || warn "Clippy reportó warnings/errores (no bloqueante en instalador automático)."
  else
    warn "clippy no disponible; omitiendo cargo clippy."
  fi
}

run_tests() {
  if [[ "${SKIP_TESTS:-0}" == "1" ]]; then
    warn "SKIP_TESTS=1: omitiendo pruebas."
    return 0
  fi

  local jobs
  jobs="$(detect_jobs)"
  log "Ejecutando pruebas (cargo test, jobs=${jobs})..."
  if cargo_with_jobs "$jobs" test --all-targets; then
    log "Pruebas OK."
    return 0
  fi

  warn "Las pruebas fallaron. Intentando autocorrección: git pull --ff-only y reintento..."
  git -C "$APP_DIR" pull --ff-only || true
  if cargo_with_jobs "$jobs" test --all-targets; then
    log "Pruebas OK tras reintento."
    return 0
  fi

  die "Las pruebas fallaron. Revisa el error arriba. Usa SKIP_TESTS=1 para omitir temporalmente si estás depurando."
}

build_release() {
  local jobs
  jobs="$(detect_jobs)"
  ensure_swap_if_low_mem

  log "Compilando en modo release (jobs=${jobs})..."
  if cargo_with_jobs "$jobs" build --release; then
    :
  else
    warn "Fallo compilación release. Intentando recuperación automática..."
    warn "Reintento 1: build release con -j 1..."
    if ! CARGO_BUILD_JOBS=1 cargo build --release -j 1; then
      warn "Reintento 2: git pull --ff-only + cargo clean -p ${BIN_NAME} + build -j 1..."
      git -C "$APP_DIR" pull --ff-only || true
      cargo clean -p "$BIN_NAME" || true
      CARGO_BUILD_JOBS=1 cargo build --release -j 1 || die "Compilación fallida tras reintentos. Revisa errores arriba."
    fi
  fi

  local built_bin="$APP_DIR/target/release/${BIN_NAME}"
  [[ -f "$built_bin" ]] || die "No se encontró el binario compilado en $built_bin"
  [[ -s "$built_bin" ]] || die "El binario compilado está vacío: $built_bin"
  log "Compilación OK: $built_bin"
}

verify_binary() {
  local bin="$1"
  log "Verificando binario instalado..."
  [[ -f "$bin" ]] || die "Binario no encontrado: $bin"
  [[ -x "$bin" ]] || die "Binario no ejecutable: $bin"
  # Verificación ligera: en Linux, comprobar que es ELF ejecutable si file está disponible.
  if require_cmd file; then
    local info
    info="$(file -b "$bin" || true)"
    log "file: $info"
    if [[ "$info" != *"ELF"* ]] && [[ "$info" != *"executable"* ]]; then
      warn "El binario no parece ELF ejecutable según 'file': $info"
    fi
  fi
  log "Verificación de binario OK."
}

install_binary() {
  log "Instalando binario..."
  as_root systemctl stop "$SERVICE_NAME" 2>/dev/null || true
  as_root cp "$APP_DIR/target/release/${BIN_NAME}" "$BIN_DST"
  as_root chmod +x "$BIN_DST"
  verify_binary "$BIN_DST"
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
  trap cleanup_temp_swap EXIT

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
    file \
    nmap \
    dnsutils \
    bluez \
    iputils-ping

  write_display_config
  log "Apagado GPIO gestionado por hunterzero-poweroff.service (sin dtoverlay=gpio-shutdown)."

  ensure_repo
  ensure_rust

  cd "$APP_DIR"
  if ! require_cmd cargo && [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"
  require_cmd cargo || die "cargo no disponible"

  run_preflight_checks
  run_tests
  build_release
  install_binary
  install_service

  echo ""
  log "=== Instalación Completa ==="
  log "Verificaciones: preflight OK, pruebas OK (o SKIP_TESTS=1), build release OK, binario verificado."
  echo "1. Reinicia la Raspberry Pi: sudo reboot"
  echo "2. Verificar estado: systemctl status ${SERVICE_NAME}"
  echo "3. Ver logs: journalctl -u ${SERVICE_NAME} -f"
  echo ""
  echo "Notas:"
  echo " - Si cambiaste config.txt, el reinicio es necesario para aplicar pantalla."
  echo " - En Pi Zero W la compilación release puede tardar bastante."
  echo " - Para omitir pruebas: SKIP_TESTS=1 sudo -E bash deploy/install.sh"
  echo " - El instalador reintenta automáticamente ante fallos comunes y usa swap temporal en poca RAM."
}

main "$@"
