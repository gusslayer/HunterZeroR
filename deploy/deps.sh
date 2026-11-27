#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "Run as root: sudo bash deploy/deps.sh"
  exit 1
fi

apt-get update
apt-get install -y --no-install-recommends \
  git \
  ca-certificates \
  iputils-ping \
  dnsutils \
  nmap \
  bluez \
  alsa-utils \
  iw \
  rfkill

apt-get install -y --no-install-recommends raspi-utils ||
  apt-get install -y --no-install-recommends raspi-gpio ||
  echo "[HunterZero][WARN] Sin pinctrl/raspi-gpio: apagado seguro GPIO deshabilitado." >&2

rfkill unblock wlan bluetooth >/dev/null 2>&1 || true
systemctl enable bluetooth >/dev/null 2>&1 || true
systemctl restart bluetooth >/dev/null 2>&1 || true

echo "[HunterZero] Runtime checks:"
command -v iw >/dev/null 2>&1 && echo "  iw: OK" || echo "  iw: MISSING"
command -v rfkill >/dev/null 2>&1 && echo "  rfkill: OK" || echo "  rfkill: MISSING"
command -v bluetoothctl >/dev/null 2>&1 && echo "  bluetoothctl: OK" || echo "  bluetoothctl: MISSING"
command -v aplay >/dev/null 2>&1 && echo "  aplay: OK" || echo "  aplay: MISSING"
command -v pinctrl >/dev/null 2>&1 && echo "  pinctrl: OK" ||
  { command -v raspi-gpio >/dev/null 2>&1 && echo "  raspi-gpio: OK" || echo "  GPIO tool: MISSING"; }
rfkill list || true
bluetoothctl show >/dev/null 2>&1 || true
iw dev >/dev/null 2>&1 && echo "  iw dev: OK" || echo "  iw dev: no interface"
systemctl status bluetooth --no-pager 2>&1 | tail -n 5 || true

echo "[HunterZero] Dependencies installed."
