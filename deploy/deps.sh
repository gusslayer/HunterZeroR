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
  bluez

echo "[HunterZero] Dependencies installed."
