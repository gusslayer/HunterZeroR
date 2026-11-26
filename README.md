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

## Assets

- `assets/icons/`: 24 SVG (`add`, `alert`, `battery`, `bleeding`, `bluetooth`, `config`, `exit`, `file`, `folder`, `help`, `info`, `menu`, `nmap`, `nslookup`, `ping`, `proc_monitor`, `search`, `settings`, `success`, `system_monitor`, `update`, `warning`, `wifi`).
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
    ├── modules/     # 8 ToolScreen modules
    ├── platform/    # linux fbdev+evdev, windows gdi
    └── theme.rs     # colores y layout
```

## Build

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```