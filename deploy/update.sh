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
git pull --ff-only || warn "git pull falló; se continúa con la versión local."
AFTER="$(git rev-parse HEAD 2>/dev/null || echo none)"

NEED_BUILD=1
if [[ "$BEFORE" == "$AFTER" ]]; then
  log "Sin cambios git ($AFTER). Se verificará configuración y servicio."
  NEED_BUILD=0
else
  log "Cambios detectados: $BEFORE -> $AFTER"
  log "Compilando nueva versión..."
fi

if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  SUDO=""
else
  SUDO="sudo"
fi

BIN_SRC="$ROOT_DIR/target/release/hunterzero-rust"
BIN_DST="/usr/local/bin/hunterzero-rust"
SERVICE_NAME="hunterzero.service"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}"

if [[ "$NEED_BUILD" -eq 0 && ! -f "$BIN_DST" ]]; then
  log "No hay binario instalado; se forzará compilación."
  NEED_BUILD=1
fi

if [[ "$NEED_BUILD" -eq 1 ]]; then
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

  if [[ ! -f "$BIN_SRC" ]]; then
    warn "No se encontró binario compilado en $BIN_SRC"
    exit 1
  fi

  log "Instalando binario..."
  $SUDO cp "$BIN_SRC" "$BIN_DST"
  $SUDO chmod +x "$BIN_DST"
else
  log "Omitiendo compilación; se aplicará configuración/servicio."
fi

# Mantener servicio systemd actualizado (útil en SD nueva o instalaciones antiguas)
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
if [[ -f "$CONFIG_DST" ]]; then
  if ! grep -qE '^[[:space:]]*gpio=27=op,dh' "$CONFIG_DST" 2>/dev/null; then
    printf '\n# HunterZero-Rust safe shutdown GPIO\ngpio=27=op,dh\ngpio=26=ip,pu\n' | $SUDO tee -a "$CONFIG_DST" >/dev/null
    log "Líneas GPIO de apagado seguro agregadas a config.txt"
  fi
  if ! grep -qE '^[[:space:]]*dtparam=audio=on' "$CONFIG_DST" 2>/dev/null; then
    printf '\n# Audio GPi CASE 2W\ndtparam=audio=on\naudio_pwm_mode=2\n' | $SUDO tee -a "$CONFIG_DST" >/dev/null
    log "Configuración de audio agregada a config.txt"
  fi
fi

if ! command -v gpioset >/dev/null 2>&1 || ! command -v gpiomon >/dev/null 2>&1; then
  log "Instalando gpiod para apagado seguro..."
  $SUDO apt-get install -y --no-install-recommends gpiod >/dev/null 2>&1 ||
    warn "No se pudo instalar gpiod; el apagado seguro GPIO puede no funcionar."
fi

$SUDO tee /usr/local/bin/hunterzero-poweroff >/dev/null <<'POWEROFF'
#!/usr/bin/env bash
# Apagado físico de la GPi Case 2W con libgpiod.
# GPIO 27 mantiene el latch de alimentación en alto; GPIO 26 detecta el botón
# de encendido (activo en bajo). Al detectar el flanco descendente se ejecuta
# un apagado limpio.
if [ -z "${BASH_VERSION:-}" ]; then
    exec bash "$0" "$@"
fi
set -euo pipefail

GPIO_CHIP="${HUNTERZERO_GPIO_CHIP:-gpiochip0}"
POWER_BUTTON_LINE="${HUNTERZERO_POWER_BUTTON_LINE:-26}"
POWER_ENABLE_LINE="${HUNTERZERO_POWER_ENABLE_LINE:-27}"
CONSUMER="hunterzero-poweroff"
SHUTDOWN_BIN="${HUNTERZERO_SHUTDOWN_BIN:-/usr/sbin/shutdown}"
shutdown_initiated=0

for tool in gpioset gpiomon; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Falta $tool; instala el paquete gpiod." >&2
        exit 1
    fi
done

if [[ "$GPIO_CHIP" == /* ]]; then
    chip_device="$GPIO_CHIP"
else
    chip_device="/dev/$GPIO_CHIP"
fi
if [[ ! -e "$chip_device" ]]; then
    echo "No existe $chip_device; no se puede vigilar el botón de encendido." >&2
    exit 1
fi
if [[ ! -x "$SHUTDOWN_BIN" ]]; then
    echo "No existe $SHUTDOWN_BIN; no se puede apagar el equipo." >&2
    exit 1
fi

echo "Manteniendo GPIO $POWER_ENABLE_LINE en alto y esperando GPIO $POWER_BUTTON_LINE..."

gpioset -c "$GPIO_CHIP" -C "$CONSUMER" "$POWER_ENABLE_LINE=1" &
enable_pid=$!

initial_level=""
if command -v gpioget >/dev/null 2>&1; then
    initial_level="$(gpioget -c "$GPIO_CHIP" -C "$CONSUMER" "$POWER_BUTTON_LINE" 2>&1 || true)"
    echo "GPIO $POWER_BUTTON_LINE al iniciar: $initial_level"
fi

if [[ "$initial_level" == *"inactive"* ]]; then
    echo "GPIO $POWER_BUTTON_LINE ya en bajo; esperando liberación antes de vigilar apagado..."
    trap 'kill "${enable_pid:-}" 2>/dev/null || true; exit 0' INT TERM
    gpiomon -c "$GPIO_CHIP" -C "$CONSUMER" -e rising -b pull-up -p 100ms -n 1 \
        "$POWER_BUTTON_LINE" >/dev/null || true
    trap - INT TERM
fi

gpiomon -c "$GPIO_CHIP" -C "$CONSUMER" -e falling -b pull-up -p 100ms -n 1 \
    "$POWER_BUTTON_LINE" &
monitor_pid=$!
echo "Vigilando flanco descendente en GPIO $POWER_BUTTON_LINE (chip $GPIO_CHIP)."

is_system_stopping() {
    local state
    state="$(systemctl is-system-running 2>/dev/null || true)"
    case "$state" in
        stopping|offline) return 0 ;;
    esac
    if systemctl list-jobs --no-legend 2>/dev/null | grep -Eq 'shutdown\.target|poweroff\.target|reboot\.target|halt\.target'; then
        return 0
    fi
    [[ -f /run/nologin ]]
}

cleanup() {
    local keep_latch="${1:-0}"
    kill "$monitor_pid" 2>/dev/null || true
    wait "$monitor_pid" 2>/dev/null || true
    if [[ "$keep_latch" -eq 0 && "$shutdown_initiated" -eq 0 ]] && ! is_system_stopping; then
        kill "$enable_pid" 2>/dev/null || true
        wait "$enable_pid" 2>/dev/null || true
    fi
}

on_signal() {
    if [[ "$shutdown_initiated" -ne 0 ]] || is_system_stopping; then
        cleanup 1
        exit 0
    fi
    cleanup 0
    exit 0
}
trap cleanup EXIT
trap on_signal INT TERM

set +e
wait -n -p finished_pid "$enable_pid" "$monitor_pid"
finished_status=$?
set -e

if [[ "${finished_pid:-}" != "$monitor_pid" || "$finished_status" -ne 0 ]]; then
    echo "Falló la vigilancia GPIO (proceso ${finished_pid:-desconocido}, código $finished_status)." >&2
    exit 1
fi

shutdown_initiated=1
echo "Botón de encendido pulsado: apagando el equipo."
sync
"$SHUTDOWN_BIN" -h now || {
    echo "Fallo al ejecutar $SHUTDOWN_BIN; intentando systemctl poweroff." >&2
    systemctl poweroff || true
}
# Mantener GPIO 27 en alto hasta que el sistema corte energía.
wait "$enable_pid" 2>/dev/null || true
POWEROFF
$SUDO chmod +x /usr/local/bin/hunterzero-poweroff

if command -v gpioset >/dev/null 2>&1 && command -v gpiomon >/dev/null 2>&1; then
  $SUDO tee /etc/systemd/system/hunterzero-poweroff.service >/dev/null <<'EOF'
[Unit]
Description=HunterZero GPi physical power button (GPIO 26)
# Permite reintentos infinitos si el chip GPIO aún no está listo al arrancar.
StartLimitIntervalSec=0

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hunterzero-poweroff
# Mantener solo el proceso principal bajo control de systemd; gpioset debe
# sobrevivir durante el apagado para no soltar el latch GPIO 27.
KillMode=process
# Reinicia también tras una salida limpia: si el monitor se detiene por cualquier
# motivo, el botón físico debe seguir operativo sin intervención manual.
Restart=always
RestartSec=2
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
  $SUDO systemctl daemon-reload
  $SUDO systemctl enable hunterzero-poweroff.service >/dev/null 2>&1 || true
  if [[ -e /dev/gpiochip0 ]]; then
    $SUDO systemctl restart hunterzero-poweroff.service || warn "No se pudo reiniciar hunterzero-poweroff.service."
  else
    warn "No existe /dev/gpiochip0; no se reinició el apagado por GPIO."
  fi
else
  warn "Falta gpiod (gpioset/gpiomon); instala gpiod y vuelve a ejecutar la actualización."
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
