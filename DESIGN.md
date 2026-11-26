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
| Fondo | `BG_COLOR` | `#000000` | Fondo, limpieza de áreas, footer |
| Tinta | `TEXT_COLOR` | `#FFFFFF` | Texto principal, hints, iconos |
| Acento | `ACCENT_COLOR` | `#FF5500` | Títulos, foco, valor, cursor, progreso |
| Selección | `SELECTED_BG` | `#3A1400` | Fondo de fila seleccionada |
| Muted | `MUTED_COLOR` | `#666666` | Secundario, N/D, contador |
| Panel | `PANEL_COLOR` | `#0C0C0C` | Relleno de marcos/paneles |
| Borde/grid | `GRID_COLOR` | `#262626` | Bordes 1px, separadores, pista |
| OK | `SUCCESS_COLOR` | `#78C878` | Estado correcto/conectado |
| Error | `ERROR_COLOR` | `#FF6E5A` | Error, cancelación, validación |

Usar solo constantes de `theme`; no hardcodear colores en módulos.

## 3. Tipografía (`src/core/font.rs`, `src/gfx.rs`)

- Fuente bitmap 8×8 propia; escala entera (normalmente 1). `draw_text` normaliza a ASCII uppercase.
- Alineación a píxel entero. Truncar con `draw_text_clipped` (`..`). Envolver con `gfx::wrap_text`.
- `text_width = chars * 8 * scale`, `text_height = 8 * scale`. Sin kerning ni subpíxel.

## 4. Layout (`src/theme.rs`)

| Zona | Constante | Valor |
|---|---:|---|
| Canvas | `SCREEN_WIDTH` × `SCREEN_HEIGHT` | 320 × 240 |
| Ventana física | `WINDOW_WIDTH` × `WINDOW_HEIGHT` | 640 × 480 |
| Header | `HEADER_HEIGHT` | 27 px (`y 0..27`) |
| Content top | `CONTENT_TOP` | 28 |
| Content bottom | `CONTENT_BOTTOM` | 211 |
| Content height | `CONTENT_HEIGHT` | 183 px |
| Footer top/alto | `FOOTER_TOP` / `FOOTER_HEIGHT` | 211 / 29 px |
| FPS | `FPS` | 15 |

Los módulos dibujan solo en `CONTENT_TOP..CONTENT_BOTTOM`. No dibujar header/footer propios ni escribir fuera del canvas.

## 5. Componentes reutilizables (`src/core/ui.rs`)

- **Filas/listas**: `ScrollState` + `draw_row`. Selección: `SELECTED_BG`, borde `ACCENT`, barra izquierda 3px, chevron.
- **Scroll**: `draw_scroll_indicator`; `ScrollState::move_by` con wrap-around y clamp.
- **Ayuda**: `HelpSection { heading, paragraphs }` + `draw_help(canvas, sections, &mut ScrollState)`. El módulo solo provee `static HELP`.
- **Marcos/paneles**: `draw_frame(..., Some(PANEL_COLOR))`, borde `GRID_COLOR`, esquinas con ticks.
- **Progreso/métrica**: `draw_meter` (0..100, 4 segmentos). Barras indeterminadas solo con estado propio y tiempo real.
- **Editores**: `Ipv4Editor`, `MacEditor`, `HostnameEditor`; validar con `core::validation`.
- **Iconos**: `draw_icon` usa primitivas 14×14; assets SVG en `assets/icons/` (24 iconos) con fallback por nombre.
- **Footer**: 4 hints `[(&str, i32)]`; formato base menú: `D-PAD Mover`, `A Abrir`, `START Ejecutar`, `B Volver`.

## 6. Entrada y movimiento

- Entrada normalizada: `core::input::Action::{Up,Down,Left,Right,Open,Back,Start,Quit}`.
- `Action::Start.as_open() => Open` dentro de módulos. `Back` vuelve/cancela; no sale de la app. `Quit` lo maneja el loop principal.
- Debounce: 80 ms (`Debouncer`). Animaciones/movimiento: enteros, no bloqueantes, compatibles con 15 FPS; transiciones instantáneas por defecto.

## 7. Reglas Do / Don't

- **Do**: reutilizar `gfx::Canvas`, `theme`, `core::ui`; mantener píxeles enteros; usar `ScrollState`; limpiar área útil; exponer ayuda estática.
- **Don't**: no añadir dependencias gráficas pesadas; no dibujar fuera de content; no hardcodear paleta/layout; no crear widgets duplicados por módulo; no bloquear el loop de render/input.