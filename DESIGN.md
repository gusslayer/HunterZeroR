# DESIGN.md — HunterZero-Rust

Sistema visual vigente para `hunterzero-rust` v0.2.0 (GPi Case 2W / Pi Zero W, `armhf`).
Fuente de verdad: `src/theme.rs`, `src/gfx.rs`, `src/core/ui.rs`, `src/modules/`.

## 1. Visual

- Estética retro-hacker monocromática/bicolor, alto contraste, cero ruido visual.
- Render por software en `gfx::Canvas` (ARGB8888, 320×240), escalado 2× nearest-neighbor a 640×480 por el backend.
- Sin degradados, sombras, blur, antialiasing ni transparencia. Coordenadas en píxeles enteros.

## 2. Paleta (`src/theme.rs`)

| Rol | Constante | HEX | Uso |
|---|---|---|---|
| Fondo | `BG_COLOR` | `#000000` | Fondo, limpieza de áreas, HUD |
| Tinta | `TEXT_COLOR` | `#FFFFFF` | Texto principal, hints, iconos |
| Acento | `ACCENT_COLOR` | `#FF5500` | Títulos, foco, valor, cursor, BT activo |
| Selección | `SELECTED_BG` | `#3A1400` | Fondo de fila seleccionada |
| Muted | `MUTED_COLOR` | `#666666` | Secundario, N/D, desconectado |
| Panel | `PANEL_COLOR` | `#0C0C0C` | Relleno de marcos/paneles |
| Borde/grid | `GRID_COLOR` | `#262626` | Bordes 1px, separadores, pista |
| OK | `SUCCESS_COLOR` | `#78C878` | OK, Wi-Fi conectado, batería cargando |
| Error | `ERROR_COLOR` | `#FF6E5A` | Error, batería <20% descargando |

Usar solo constantes de `theme`; no hardcodear colores en módulos.

## 3. Tipografía y layout base

- Fuente bitmap 8×8 (`src/core/font.rs`), escala entera (normalmente 1), ASCII uppercase. `text_width = chars*8*scale`.
- Truncar con `draw_text_clipped` (`..`); envolver con `gfx::wrap_text`. Píxeles enteros, sin kerning/subpíxel.

| Zona | Constante | Valor |
|---|---:|---|
| Canvas | `SCREEN_WIDTH` × `SCREEN_HEIGHT` | 320 × 240 |
| Ventana | `WINDOW_WIDTH` × `WINDOW_HEIGHT` | 640 × 480 |
| Header global | `HEADER_HEIGHT` | 27 px (`y 0..27`) |
| Content | `CONTENT_TOP..CONTENT_BOTTOM` | `28..211` (183 px) |
| Footer global | `FOOTER_TOP` / `FOOTER_HEIGHT` | `211 / 29 px` (`y 211..240`) |
| FPS | `FPS` | 15 |

## 4. Header global fijo (`src/core/ui.rs::draw_global_header`)

Dibujado solo por `src/main.rs`. Limpia `0..320, 0..27` con `BG_COLOR`; línea inferior 1px en `y=26` con `GRID_COLOR`.
Indicadores en `y=6` (icono 14×14 + texto 8px centrado verticalmente):

| X | Icono | Texto / color |
|---:|---|---|
| 4 | `cpu` | `{cpu_percent}%` |
| 60 | `memory` | `{ram_percent}%` |
| 116 | `disk` | `{disk_percent}%` |
| 176 | `wifi` | `SUCCESS_COLOR` si conectado, `MUTED_COLOR` si no |
| 200 | `bluetooth` | `ACCENT_COLOR` si activo, `MUTED_COLOR` si apagado |
| 228 | `battery` | `+` si `battery_charging`; `{battery_percent}%` o `N/D` `MUTED_COLOR`; `SUCCESS_COLOR` cargando, `ERROR_COLOR` si `<20%` descargando, si no `TEXT_COLOR` |

`SystemStats` provee valores cacheados/no bloqueantes: `cpu_percent:u8`, `ram_percent:u8`, `disk_percent:u8`, `wifi_connected:bool`, `bluetooth_active:bool`, `battery_percent:Option<u8>`, `battery_charging:bool`.

## 5. Footer global fijo (`src/core/ui.rs::draw_global_footer`)

Dibujado solo por `src/main.rs`, `y 211..240`, fondo `BG_COLOR`, línea superior `GRID_COLOR`. Hints fijos:
`D-PAD Mover` (x=4), `A Abrir` (x=116), `START Ejecutar` (x=188), `B Volver` (x=272). Los módulos no dibujan footer ni exponen `footer_hints`.

## 6. Componentes reutilizables (`src/core/ui.rs`)

- **Listas**: `ScrollState` + `draw_row` (selección `SELECTED_BG`, borde `ACCENT`, barra 3px, chevron).
- **Ayuda**: `HelpSection` + `draw_help`; módulo solo provee `static HELP`.
- **Marcos/métrica**: `draw_frame(..., Some(PANEL_COLOR))`, `draw_meter` 0..100 (4 segmentos).
- **Editores**: `Ipv4Editor`, `MacEditor`, `HostnameEditor`; validar con `core::validation`.
- **Iconos**: `draw_icon` primitivas 14×14; fallback por nombre; assets `assets/icons/` (24 SVG).
- **Logs**: `LogsScreen` (`id: logs`, icono `file`) lee `logs/app.log` y `logs/commands/latest.log`; `A` alterna, `START` vacía log actual, `D-PAD` scroll, `B` vuelve.

## 7. Entrada y reglas

- `Action::{Up,Down,Left,Right,Open,Back,Start,Quit}`; `Start.as_open() => Open`; `Back` vuelve; `Quit` solo en main loop. Debounce 80 ms.
- **Do**: reutilizar `gfx`, `theme`, `core::ui`; píxeles enteros; dibujar solo en content; exponer ayuda estática.
- **Don't**: no dibujar header/footer en módulos; no hardcodear paleta/layout; no deps gráficas pesadas; no bloquear render/input.