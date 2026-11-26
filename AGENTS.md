# AGENTS.md — HunterZero-Rust

Reglas de ingeniería vigentes para `hunterzero-rust` v0.2.0 (Rust 2021, `rust-version 1.75`).
Target de despliegue: Raspberry Pi Zero W v1.1, PiOS Lite 32-bit `armhf`, GPi Case 2W.

## 1. Entorno y dependencias

- Mantener dependencias ligeras (`Cargo.toml`): `chrono`, `serde`, `serde_json`; `libc` en Unix; `windows-sys` + `sysinfo` en Windows.
- Linux usa backend nativo: framebuffer `/dev/fb*` + evdev `/dev/input/event*` (`src/platform/linux.rs`). Windows usa GDI (`src/platform/windows.rs`).
- No agregar crates/funcionalidades pesadas innecesarias para Pi Zero W. Release: `opt-level=2`, `lto="thin"`, `codegen-units=1`, `panic="abort"`, `strip=true`.
- Deploy real: `deploy/config.txt`, `deploy/install.sh`, `deploy/update.sh`, `deploy/deps.sh`. Compilar con `cargo build --release`.

## 2. Arquitectura de módulos

- Cada módulo es autocontenido en `src/modules/<nombre>.rs` o `src/modules/<nombre>/mod.rs`.
- Implementar `modules::ToolScreen` (`src/modules/mod.rs`):
  - `title(&self) -> &str`
  - `handle(&mut self, action: Action, processes: &mut ProcessManager) -> Outcome`
  - `sync(&mut self, processes: &mut ProcessManager)` (poll no bloqueante, default noop)
  - `draw(&mut self, canvas: &mut Canvas, processes: &ProcessManager, stats: &SystemStats)`
  - `close(&mut self, processes: &mut ProcessManager)` (cleanup)
- `Outcome::{Stay, Close}`. `Back` vuelve al menú; no salir de la app desde un módulo.
- Registrar sin tocar lógica core: `pub mod <nombre>;` y rama en `modules::create(...)` (`src/modules/mod.rs`) + entrada en `core::plugins::MODULES`.
- Reutilizar `src/gfx.rs`, `src/theme.rs`, `src/core/ui.rs`. Dibujar solo dentro de `CONTENT_TOP..CONTENT_BOTTOM`.
- Header/footer globales fijos los dibuja solo `src/main.rs` con `draw_global_header`/`draw_global_footer`; los módulos no los dibujan ni exponen hints.
- Aislamiento de fallos: `main` limpia content area antes de dibujar módulo, envuelve `handle/sync/draw` en `catch_unwind(AssertUnwindSafe(..))`; ante pánico detiene procesos, loguea `[ERROR]`, cierra módulo y vuelve al menú con alerta breve. (Efectivo con `panic=unwind`; en release `panic=abort` no captura.)
- Para herramientas parametrizadas, reutilizar `modules::parameterized::{ParameterizedScreen, ToolSpec}`.

## 3. Entrada y ayuda

- Los módulos reciben `core::input::Action` normalizada; no leer teclado/GPIO directamente.
- `Action::{Up,Down,Left,Right,Open,Back,Start,Quit}`; `Start.as_open()` se trata como `Open`.
- Debounce global: 80 ms (`core::input::Debouncer`). `Quit` lo consume el loop principal.
- Ayuda estándar: `static HELP: &[core::ui::HelpSection]` y render con `core::ui::draw_help`; no dibujar overlay de ayuda propio.

## 4. Logging y diagnóstico

- Usar `core::logging::{init, log}`. No usar `println!/dbg!` para diagnóstico.
- Log mínimo y accionable: inicio/cierre de sesión y errores/warnings relevantes (fallos de comando, backend, timeout, validación, cancelación).
- No loguear por frame, input, poll, sync, cambios de menú/fase ni salida verbosa de subprocesos.
- Formato objetivo de una línea: `[YYYY-MM-DD HH:MM:SS] [NIVEL] [MODULO] Mensaje`.
- Proteger microSD: append simple, sin dumps; salida de comandos en `logs/commands/latest.log` (tail acotado por `ProcessManager`).

## 5. Procesos y no bloqueo

- Nunca bloquear UI/FPS. Operaciones externas por `core::process_manager::ProcessManager`.
- Usar comandos allowlisted/validados; sin shell `sh -c`; validar input (`core::validation`) antes de spawn.
- Capturar salida a archivo, leer cola acotada, `try_wait`/poll por frame, timeout, cancelación con `Back`/`close`.
- Si se agrega trabajo en background, usar hilo/subproceso separado + canal `std::sync::mpsc`; no bloquear con waits en UI.

## 6. Gatekeeper

Ejecutar antes de finalizar cambios:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

- Prohibido `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` en código de producción; preferir `Result/Option`, `?`, `let ... else`, `unwrap_or*` seguro.
- Sin `unsafe` fuera de `src/platform`/`src/gfx` y con `// SAFETY:`. Sin `static mut`.
- No afirmar "0 errores" sin salida real de los gates en un entorno con toolchain funcional.