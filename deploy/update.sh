#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log()  { echo "[HunterZero] $*"; }
warn() { echo "[HunterZero][WARN] $*" >&2; }

log "Actualizando repositorio..."
BEFORE="$(git rev-parse HEAD 2>/dev/null || echo none)"
git pull --ff-only
AFTER="$(git rev-parse HEAD 2>/dev/null || echo none)"

if [[ "$BEFORE" == "$AFTER" ]]; then
  log "Ya estás en la última versión ($AFTER)."
  exit 0
fi

log "Cambios detectados: $BEFORE -> $AFTER"
log "Compilando nueva versión..."

# Cargar cargo si hace falta
if ! command -v cargo >/dev/null 2>&1 && [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null 2>&1 || { warn "cargo no encontrado; no se puede compilar."; exit 1; }

JOBS=1
if [[ -r /proc/meminfo ]]; then
  MEM_KB="$(awk '/MemTotal/ {print $2}' /proc/meminfo)"
  [[ "${MEM_KB}" -lt 900000 ]] && JOBS=1 || JOBS=2
fi

CARGO_BUILD_JOBS="$JOBS" cargo build --release -j "$JOBS"

BIN_SRC="$ROOT_DIR/target/release/hunterzero-rust"
BIN_DST="/usr/local/bin/hunterzero-rust"
if [[ ! -f "$BIN_SRC" ]]; then
  warn "No se encontró binario compilado en $BIN_SRC"
  exit 1
fi

log "Instalando binario..."
if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  SUDO=""
else
  SUDO="sudo"
fi

$SUDO cp "$BIN_SRC" "$BIN_DST"
$SUDO chmod +x "$BIN_DST"

# Mantener servicio systemd actualizado (útil en SD nueva o instalaciones antiguas)
SERVICE_NAME="hunterzero.service"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}"
$SUDO tee "$SERVICE_FILE" >/dev/null <<EOF
[Unit]
Description=HunterZero-Rust Application
After=network.target

[Service]
ExecStartPre=-/bin/sh -c 'echo 0 > /sys/class/graphics/fbcon/cursor_blink 2>/dev/null; printf "\\033[?25l\\033[?1c" > /dev/tty1 2>/dev/null; printf "\\033[?25l\\033[?1c" > /dev/tty0 2>/dev/null; true'
ExecStart=${BIN_DST}
Restart=on-failure
WorkingDirectory=${ROOT_DIR}
User=root

[Install]
WantedBy=multi-user.target
EOF
$SUDO systemctl daemon-reload
$SUDO systemctl enable "$SERVICE_NAME" >/dev/null 2>&1 || true
$SUDO systemctl restart "$SERVICE_NAME" || warn "No se pudo reiniciar ${SERVICE_NAME} automáticamente."

log "Actualización completa."