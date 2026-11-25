# HunterZero-Rust: Full Rust Implementation Status

## Project Overview
HunterZero-Rust is a Rust-based security auditing framework for GPi Case 2W (Raspberry Pi Zero W). All modules have been migrated from Python prototypes to full Rust implementations.

## Modules Status (7/7 - 100% Complete)

| Module | ID | Rust Status | Key Features |
|--------|----|-------------|--------------|
| **Bleeding** | `bleeding` | ✅ Full Rust | Bluetooth LE scanning, deauth operations, mode selection (Scan/Deauth/HandshakeCapture/Monitor) |
| **Nmap** | `nmap` | ✅ Full Rust | TCP/SYN/UDP/Fast scanning, checkbox options (-sV, -O, -A, -F, -sT, timing templates T4/T5), nmap execution via `std::process` |
| **Nslookup** | `nslookup` | ✅ Full Rust | Interactive DNS resolution |
| **Ping** | `ping` | ✅ Full Rust | IP construction octeto a octeto, ICMP packets, 1-4 packets configurable |
| **System Monitor** | `system_monitor` | ✅ Full Rust | CPU %, memory %, temperature, local IP, update statistics |
| **Proc Monitor** | `proc_monitor` | ✅ Full Rust | Process list viewer with navigation |
| **Update** | `update` | ✅ Full Rust | Update checking with progress, version management |

## Architecture

### ToolScreen Trait (src/main.rs)
All modules implement this trait:
- `draw_content(ctx, canvas)` - Draws module UI in the usable area (Y=20 to Y=220)
- `handle_action(action)` - Handles D-pad, A, B, START input
- `footer_hints()` - Returns fixed 4 hints: ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
- `close()` - Cleans up module state

### Plugin Discovery
- Modules discovered by scanning `modules/` for `manifest.yaml`
- Each module has: `id`, `name`, `description`, `icon`, `version`
- `PluginCatalog` loads modules dynamically at runtime

### UI Layout (from theme.rs)
- **Virtual screen**: 320x240 scaled to 640x480
- **Header**: 20px, icons only (⎋, ⎊, ℹ)
- **Footer**: 20px, fixed hints (never changes)
- **Content area**: Y=20 to Y=220 (200px high)
- **Colors**: BG_COLOR (black), TEXT_COLOR (white), ACCENT_COLOR (orange)

## Dependencies (Cargo.toml)
All dependencies are Rust crates. No Python embedury needed.

```toml
[dependencies]
ggez = "0.8"
ggez::input = "0.8"
ggez::graphics = "0.8"
ggez::event = "0.8"
sdl2 = "2.8"
sdl2::ttf = "2.8"
rppal = "1.5"
sys-infos = "0.1"
timeout = "0.2"
```

## Migration History
- **pyembed = "0.4"** was removed from dependencies - all modules now full Rust
- Each module's `screen.rs` was migrated from original Python `screen.py`
- All modules implement the `ToolScreen` trait consistently
- Nmap module includes checkbox-based option selection and `std::process::Command` execution

## Building
```bash
cargo check  # or cargo build
```

No Python dependencies required. All functionality is implemented in pure Rust.