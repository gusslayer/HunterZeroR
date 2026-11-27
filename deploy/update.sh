#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log()  { echo "[HunterZero] $*"; }
warn() { echo "[HunterZero][WARN] $*" >&2; }

for cmd in iw rfkill bluetoothctl aplay; do
  command -v "$cmd" >/dev/null 2>&1 || warn "Falta comando runtime: $cmd (ejecuta: sudo bash deploy/deps.sh)"
done

log "Actualizando repositorio..."
git config --global --add safe.directory "$ROOT_DIR" 2>/dev/null || true
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

if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  SUDO=""
else
  SUDO="sudo"
fi

JOBS=1
if [[ -r /proc/meminfo ]]; then
  MEM_KB="$(awk '/MemTotal/ {print $2}' /proc/meminfo)"
  [[ "${MEM_KB}" -lt 900000 ]] && JOBS=1 || JOBS=2
fi

# Pi Zero has 512 MB RAM; a release build needs a temporary swap file.
SWAP_FILE="/var/swap-hunterzero"
cleanup_update_swap() {
  if swapon --show 2>/dev/null | grep -q "$SWAP_FILE"; then
    $SUDO swapoff "$SWAP_FILE" || true
  fi
  [[ -f "$SWAP_FILE" ]] && $SUDO rm -f "$SWAP_FILE" || true
}
if [[ -r /proc/meminfo && "${MEM_KB:-0}" -lt 900000 ]] &&
  ! swapon --show 2>/dev/null | grep -q .; then
  if [[ ! -f "$SWAP_FILE" ]]; then
    $SUDO fallocate -l 1G "$SWAP_FILE" 2>/dev/null ||
      $SUDO dd if=/dev/zero of="$SWAP_FILE" bs=1M count=1024 status=none
    $SUDO chmod 600 "$SWAP_FILE"
    $SUDO mkswap "$SWAP_FILE" >/dev/null
  fi
  if $SUDO swapon "$SWAP_FILE"; then
    trap cleanup_update_swap EXIT
  else
    warn "No se pudo activar swap temporal; la compilación puede fallar por OOM."
  fi
fi

CARGO_BUILD_JOBS="$JOBS" cargo build --release -j "$JOBS"

BIN_SRC="$ROOT_DIR/target/release/hunterzero-rust"
BIN_DST="/usr/local/bin/hunterzero-rust"
if [[ ! -f "$BIN_SRC" ]]; then
  warn "No se encontró binario compilado en $BIN_SRC"
  exit 1
fi

log "Instalando binario..."
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

log "Configurando apagado seguro GPIO (GPi Case 2W)..."
CONFIG_DST="/boot/firmware/config.txt"
if [[ ! -f "$CONFIG_DST" && -f "/boot/config.txt" ]]; then
  CONFIG_DST="/boot/config.txt"
fi
if [[ -f "$CONFIG_DST" ]] && ! grep -qE '^[[:space:]]*gpio=27=op,dh' "$CONFIG_DST" 2>/dev/null; then
  printf '\n# HunterZero-Rust safe shutdown GPIO\ngpio=27=op,dh\ngpio=26=ip,pu\n' | $SUDO tee -a "$CONFIG_DST" >/dev/null
  log "Líneas GPIO de apagado seguro agregadas a config.txt"
fi
$SUDO tee /usr/local/bin/hunterzero-poweroff.sh >/dev/null <<'POWEROFF'
#!/bin/bash
# HunterZero safe shutdown for Retroflag GPi Case 2W.
# GPIO27 = power latch (output, high). GPIO26 = power button (input, active low).
set -eu

if command -v pinctrl >/dev/null 2>&1; then
  pinctrl set 27 op dh
  pinctrl set 26 ip pu
  while true; do
    state="$(pinctrl get 26 2>/dev/null || true)"
    case "$state" in
      *"lo"*|*"dl"*|*"level=0"*)
        /sbin/shutdown -h now "HunterZero safe shutdown"
        exit 0
        ;;
    esac
    sleep 1
  done
elif command -v raspi-gpio >/dev/null 2>&1; then
  raspi-gpio set 27 op dh
  raspi-gpio set 26 ip pu
  while true; do
    state="$(raspi-gpio get 26 2>/dev/null || true)"
    case "$state" in
      *"level=0"*)
        /sbin/shutdown -h now "HunterZero safe shutdown"
        exit 0
        ;;
    esac
    sleep 1
  done
else
  echo "No GPIO tool found (pinctrl/raspi-gpio)" >&2
  exit 1
fi
POWEROFF
$SUDO chmod +x /usr/local/bin/hunterzero-poweroff.sh
if command -v pinctrl >/dev/null 2>&1 || command -v raspi-gpio >/dev/null 2>&1; then
  $SUDO tee /etc/systemd/system/hunterzero-poweroff.service >/dev/null <<'EOF'
[Unit]
Description=HunterZero Safe Shutdown (GPi Case 2W)
After=local-fs.target
Before=multi-user.target

[Service]
Type=simple
ExecStart=/usr/local/bin/hunterzero-poweroff.sh
Restart=on-failure
RestartSec=2
User=root

[Install]
WantedBy=multi-user.target
EOF
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable hunterzero-poweroff.service >/dev/null 2>&1 || true
  $SUDO systemctl restart hunterzero-poweroff.service || warn "No se pudo reiniciar hunterzero-poweroff.service."
else
  warn "No hay pinctrl ni raspi-gpio; instala raspi-utils y vuelve a ejecutar la actualización."
fi

# Restart last: when launched from the UI, this script runs as a child of
# hunterzero.service. A plain systemctl restart would kill this process via the
# service cgroup before it can finish. Schedule the restart in a detached
# transient unit so the update can complete cleanly.
log "Actualización completa. Reiniciando servicio..."
if command -v systemd-run >/dev/null 2>&1; then
  $SUDO systemd-run --quiet --no-block --collect --unit=hunterzero-self-restart \
    /bin/sh -c "sleep 2; exec /bin/systemctl restart ${SERVICE_NAME}" ||
    warn "No se pudo programar reinicio de ${SERVICE_NAME}."
else
  # Fallback: best-effort detached restart (may be killed with service cgroup).
  nohup /bin/sh -c "sleep 2; exec /bin/systemctl restart ${SERVICE_NAME}" >/dev/null 2>&1 </dev/null &
fi
