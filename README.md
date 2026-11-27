# HunterZero-Rust

Portable diagnostics UI for **GPi Case 2W + Raspberry Pi Zero W v1.1** (PiOS Lite 32-bit, `armhf`), v0.2.0.

## Architecture

- **Software render**: `gfx::Canvas` 320×240, escalado 2× nearest-neighbor a 640×480.
- **Linux backend**: `/dev/fb0` (fbdev/KMS-DPI) + evdev (`/dev/input/event*`).
- **Windows backend**: GDI, solo para desarrollo.
- UI y módulos 100% Rust, dependencias ligeras.

## Native Modules (`src/modules/`)

| ID | Función |
|---|---|
| `bleeding` | Bluetooth LE / BR-EDR scan, enum, DeAuth acotado |
| `logs` | Visor on-device de `logs/app.log` y `logs/commands/latest.log` |
| `nmap` | TCP connect scan acotado con NSE opcional |
| `nslookup` | DNS lookup interactivo (A/AAAA) |
| `ping` | ICMP echo manual |
| `proc_monitor` | Lista de procesos por CPU |
| `system_monitor` | CPU/RAM/temp/IP/Wi-Fi/batería/disco/reloj |
| `update` | Ejecuta `deploy/update.sh` y muestra salida |
| `wifi_survey` | Descubre SSID, selecciona uno y mide señal/heatmap en tiempo real |

## Assets

- `assets/icons/`: iconos PNG (`add`, `alert`, `battery`, `bleeding`, `bluetooth`, `config`, `exit`, `file`, `folder`, `help`, `info`, `menu`, `nmap`, `nslookup`, `ping`, `proc_monitor`, `search`, `settings`, `success`, `system_monitor`, `update`, `warning`, `wifi`, entre otros).
- `assets/fonts/PressStart2P-Regular.ttf`.

## Docs

- [`DESIGN.md`](DESIGN.md): sistema visual, layout, componentes y reglas UI.
- [`AGENTS.md`](AGENTS.md): contrato de módulos, input, logging, procesos no bloqueantes, gatekeeper.

## Structure

```text
.
├── .gitignore
├── AGENTS.md
├── Cargo.lock
├── Cargo.toml
├── DESIGN.md
├── README.md
├── assets/
│   ├── fonts/
│   └── icons/
├── deploy/
│   ├── config.txt
│   ├── deps.sh
│   ├── install.sh
│   └── update.sh
└── src/
    ├── core/        # input, font, logging, plugins, process_manager, state, system_stats, ui, validation
    ├── gfx.rs       # canvas 320x240
    ├── main.rs      # loop, menú, backend
    ├── modules/     # 9 ToolScreen modules
    ├── platform/    # linux fbdev+evdev, windows gdi
    └── theme.rs     # colores y layout
```

## Toolchain

Requisitos mínimos: Rust stable 1.75+ con `rustfmt` y `clippy`.

```bash
# Linux / Raspberry Pi OS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
rustup component add rustfmt clippy

# Dependencias de compilación y runtime en PiOS
sudo apt-get update
sudo apt-get install -y build-essential pkg-config file \
  bluez iw rfkill alsa-utils raspi-utils
```

En Windows (desarrollo) instala **Visual Studio Build Tools** con la carga de trabajo
“Desarrollo para el escritorio con C++” (aporta `link.exe`), o usa WSL/Pi para el
gatekeeper completo.

## Build

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```