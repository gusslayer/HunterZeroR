# HunterZeroR - Diseño de Interfaz de Usuario

## Esquema de Colores Oficial

| Propósito | Color RGB | Hex | Comentario |
|---|---|---|---|
| Fondo negro (principal) | (0x00, 0x00, 0x00) | #000000 | Pantalla apagada/menú base |
| Fondo de filas seleccionadas | (0x20, 0x20, 0x20) | #202020 | Subtle highlight cuando navega D-pad |
| Texto/letra principal | (0xFF, 0xFF, 0xFF) | #FFFFFF | Blanco puro |
| Texto secundario/deshabilitado | (0xA0, 0xA0, 0xA0) | #A0A0A0 | Cuando no es la opción activa |
| Acento/naranja | (0xFF, 0xA5, 0x00) | #FF8000 | Botones, selecciones, líneas |
| Panel inferior | (0x10, 0x10, 0x10) | #101010 | Barra de hints abajo |

## Escalado de Pantalla (Critical)

```
Raspberry Pi → KMSDRM → SDL2 → HunterZeroR

Resolución virtual:     320 × 240 píxeles
Factor de escala:            × 2 (nearest-neighbor)
Resolución final:          640 × 480 píxeles

⚠️ IMPORTANTE: No usar ningún tipo de antialiasing ni smoothing.
Los glifos de Press Start 2P tienen 8×8 píxeles y deben escalarse
entero (2×) para que sigan siendo nítidos y legibles.
```

## Tipografía

| Uso | Tamaño | Archivo |
|---|---|---|
| Hints de botones (pie de pantalla) | 8 px | `PressStart2P-Regular.ttf` |
| Texto pequeño en resultados | 8 px | `PressStart2P-Regular.ttf` |
| Íconos de sistema (header) | 12 px | `PressStart2P-Regular.ttf` |

**NOTA sobre la tipografía:**
- El header muestra SÓLO 3 íconos pixel art de 12-16 píxeles cada uno
- El footer usa hints de 8px en posiciones X fijas
- NO hay títulos "HUNTERZERO", "CONFIGURAR", "EN CURSO", etc.
- El texto solo aparece en el área usable central (200px alto) cuando un módulo lo requiere

## Distribución Fija de la Pantalla Virtual 320x240

### Alturas Constantes (SIN Títulos)

```
ALTURAS FIXAS:
--------------------
Header:     20 px   (3 íconos pequeños, Y de 0 a 19)
Contenido:  200 px   (área usable Y de 20 a 219)
Footer:     20 px   (hints fijos, Y de 220 a 239)
--------------------
TOTAL:      240 px
```

### Posiciones Y Fijas:

| Elemento | Posición Y | Altura |
|---|---|---|
| Header (íconos) | Y=0 | 20px |
| Conten usable | Y=20 | 200px |
| Footer (hints) | Y=220 | 20px |

### Área usable (200px):

Es el espacio central donde cada módulo dibuja su contenido.
- Comienza en la fila 20 (después del header de íconos)
- Termina en la fila 219 (antes del footer de hints)
- Mide exactamente 200 píxeles de altura horizontal (filas 20-219)

## Interfaz de Usuario: Header y Footer

### 1. Header: SÓLO ÍCONOS DE SISTEMA

El header de 20 píxeles en la parte superior contiene **UNICAMENTE 3 iconos pixel art pequeños**.

**Distribución en X (pantalla virtual 320px):**

| Posición X | Contenido | Descripción |
|---|---|---|
| x=4 | Ícono Izquierda | Señal de red/conectividad (`⎋`) |
| x=160 | Ícono Centro | Estado del sistema (`⎊`) |
| x=316 | Ícono Derecha | Información/ayuda (`ℹ`) |

**Características:**
- ❌ **SIN texto**: No muestra "HUNTERZERO", "CPU", "MEM", "WiFi"
- ❌ **SIN títulos de módulo**: No hay "CONFIGURAR", "EN CURSO", etc.
- ✅ **Sólo gráficos**: 3 íconos de 12-16 píxeles cada uno
- ✅ **Siempre visible**: Fila superior constante en todas las pantallas
- ✅ **Íconos generables en Rust**: Ver sección de iconos para detalles de generación

### 2. Footer: CONJUNTO FIXTO de Hints (Siempre Igual)

El footer de 20 píxeles en la parte inferior muestra SIEMPRE el mismo conjunto de 4 hints, **nunca cambia según el estado** del módulo.

**Distribución en X (pantalla virtual 320px) - COORDENADAS EXACTAS:**

| Posición X | Hint | Significado | Estado Always |
|---|---|---|---|
| x=4 | **D-PAD Mover** | Navegar valores/oxetos | Siempre visible |
| x=116 | **A Abrir** | Confirmar/abrir acción | Siempre visible |
| x=188 | **START Ejecutar** | Lanzar comando/escaneo | Siempre visible |
| x=272 | **B Volver** | Cancelar/retroceso al menú | Siempre visible |

**Características:**
- ❌ **Ya NO cambia por estado**: Antes tenía 5 estados diferentes (Menú, Configurando, EN CURSO, Resultado, Ayuda)
- ❌ **Ya NO hay "dobles opciones"**: Antes mostraba diferentes combinations de 2-3 hints
- ✅ **Siempre los mismos 4 hints**: Mismo conjunto en todas las pantallas
- ✅ **Posiciones X fijas**: Nunca se mueven, siempre en x=4, 116, 188, 272
- ✅ **Misma altura**: Siempre 20px en la fila inferior (Y=220 a Y=239)
- ✅ **Letra 8px**: Todos los hints usan la fuente Press Start 2P de 8 píxeles

### 3. Área Usable Central (200px)

El espacio entre el header y el footer donde cada módulo muestra su contenido.

**Características:**
- Altura: 200 píxeles (filas 20 a 219)
- Los módulos SÓLO dibujan aquí su contenido
- **No dibujan header** (ese se muestra automáticamente arriba)
- **No dibujan footer** (ese se muestra automáticamente abajo)
- Cada módulo tiene libertad de usar las 200px para su contenido
- El contenido puede tener títulos internos, pero esos son parte del módulo, no de la UI global

### 4. Estados de Módulo (Dentro del Área Usable)

Dentro de las 200px del área usable, cada módulo puede tener sus propias vistas, pero **sin afectar al header ni footer globales**.

**Vistas posibles dentro del área usable:**

| Vista | Descripción | Ejemplo |
|---|---|---|
| Contenido básico | Texto e íconos en las 200px | "PING", "IP: 192.168.1.1" |
| Progreso | Barras, resultados en las 200px | `[█░░░] 3/4 paquetes` |
| Lista | Elementos enumerados en las 200px | "Procesos activos: 1. foo 2. bar" |
| Formularios | Entrada de datos en las 200px | "IP: [   ]  Paquetes: [   ]" |

**NOTA:** Cualquier texto o título que aparezca en estas 200px es específico del módulo en ejecución, **no** es parte del diseño UI global (header/footer).

## Iconos de Sistema (Header)

### Formato y Ubicación

Los íconos del header se muestra en la fila superior (Y=0 a Y=19, altura 20px).

**Ubicación de los archivos:**

1. **Ruta recomendada**: `assets/icons/` - directorio del proyecto HunterZeroR
2. **Formato**: PNG transparente con fondo alfa
3. **Tamaños oficiales**:
   - 16×16 píxeles: Mostrado en el header de 320x240 virtual screen
   - 24×24 píxeles: Opcional, para futuras resoluciones o assets de alta densidad
   - 32×32 píxeles: Para futuras actualizaciones de interfaz

**Diseño y Estilo:**

Los íconos deben seguir el estilo **pixel art consistente** con el resto de HunterZero, heredando la estética de los iconos originales (referencia: halfmage/Lucide style). Características técnicas:

- **Profundidad de color**: 24-bit con canal alfa (RGBA)
- **Fondo**: Transparente (ningún fondo sólido)
- **Línea de píxeles**: Mínimo de 1 píxel para mantener nitidez al escalado 2x
- **Paleta recomendada**: Basada en los colores oficiales: negro (#000000), blanco (#FFFFFF), naranja acento (#FFA500)

### Íconos por Módulo (especificaciones detalladas):

| Módulo | Código SVG/HTML sugerido | Descripción visual | Prioridad |
|---|---|---|---|
| **PING** | `⎋` o `|⎓|` | Símbolo de red/Conectividad | Alta |
| **NMAP** | `🔍` o lupa | Lupa de búsqueda/escaneo | Alta |
| **NSLOOKUP** | `DNS` o dominio stylizado | Nombre de dominio/resolución | Media |
| **BLEEDING** | `📡` o señal Bluetooth | Señal/ondas Bluetooth | Media |
| **SYSTEM_MONITOR** | `📊` o gráfico de líneas | Métricas/estadísticas | Media |
| **PROC_MONITOR** | `⎈` o lista | Lista/tabla de procesos | Media |

**NOTA:** Los íconos PING, NMAP, NSLOOKUP, BLEEDING son **obligatorios** para la identificación visual del módulo en el menú principal. SYSTEM_MONITOR y PROC_MONITOR son **recomendados**.

### Generación en Rust (alternativa sin assets PNG)

Para evitar dependencia de archivos externos, los íconos pueden generarse geométricamente usando la biblioteca `ggez` en tiempo de compilación o incluyendo datos embebidos:

```rust
// Ejemplo de generación geométrica de ícono PING
use ggez::graphics::{self, Color};
use std::f32::consts::PI;

fn create_ping_icon(ctx: &mut ggez::Context) -> ggez::graphics::Image {
    // Crear círculo con flecha usando primitivas de graphics
    let mut palette = vec![];
    // ... generación de píxeles manual o usando primitives::circle
    // Retornar Image con los píxeles generados
}
```

### Convención de Nomenclatura:

Si se utilizan archivos PNG, seguir nombre `modulo_tipo.png`:
- `ping.png` - ícono comunicación
- `nmap.png` - ícono búsqueda  
- `nslookup.png` - ícono DNS
- `bleeding.png` - ícono Bluetooth
- `system.png` - ícono monitoreo
- `terminal.png` - ícono procesos

### Integración en el Menú Principal:

El menú principal (vista 320x240) muestra los iconos con esta disposición:

```
+------------------------------------------------------+
|  [PING]     ICMP Echo Request                       |
|  [NMAP]     Escaneo de puertos TCP                  |
|  [NSLOOKUP] Resolución DNS interactiva              |
|  [BLEEDING]   Bluetooth scanning/deauth             |
|  [SYSTEM]     CPU, Memoria, Temperatura             |
|  [PROC]       Lista de procesos activos             |
+------------------------------------------------------+
```

Cada icono ocupa aproximadamente 16×16 píxeles en la cuadrícula del menú, con 2 píxeles de margen entre elementos para legibilidad con el D-pad.

### Guía para Creadores de Íconos:

1. **Diseño en píxel art**: Usar herramientas como Aseprite, Piskel o manualmente en código
2. **Máximo 3 colores** además del fondo transparente para mantener coherencia
3. **Formas geométricas simples**: Círculos, flechas, líneas - evitando detalles muy pequeños
4. **Probar en pantalla real**: Verificar legibilidad en 16×16 píxeles escalado a 32×32 en la pantalla GPi 640x480

**Nota importante:** Los íconos son parte del "skin" visual y no afectan la funcionalidad. Si no se proporcionan, el header mostrará 3 espacios vacíos (aún así funcional), y el menú mostrará el nombre del módulo sin ícono, manteniendo usabilidad completa.

## Transiciones de Estado

Todas las transiciones tienen estos principios:

1. **Instantáneo**: Cambiar de estado es inmediato (no hay animaciones de fade que retrasen la respuesta del D-pad)
2. **Retroalimentación auditiva/visual**: Cuando se presiona A/B, el elemento seleccionado "hace clic" visualmente (cambio de fondo o breve parpadeo)
3. **Descartar eventos de arranque**: Los botones mantenidos al iniciar no se procesan (igual que en Python con `pygame.event.clear()`)
4. **Volver siempre al menú**: Presionar B desde cualquier módulo regresa al menú principal, nunca salga de la aplicación

## Resumen Visual Rápido

```
[PIZARRA 640x480 escalado de 320x240]

--- ARRIBA ---
| ⎋     ⎊     ℹ    |  (3 íconos en header, SIN texto)
|                    |
|                    |  (área usable: 200px de altura, Y=20 a Y=219)
|                    |
--- ABAJO ---
| D-PAD Mover  A Abrir  START Ejecutar  B Volver |  (footer FIXTO, 20px, siempre igual)
--- ABAJO ---
```

**Diferencias clave con el diseño anterior:**

| Aspecto | Diseño Anterior | Diseño Actual |
|---|---|---|
| **Header** | "HUNTERZERO" + CPU/Mem/WiFi + texto | 3 íconos SIN texto |
| **Footer** | Cambiaba por estado (5 estados diferentes) | SIEMPRE los mismos 4 hints |
| **Títulos módulo** | "CONFIGURAR", "EN CURSO", etc. | SIN títulos en header/footer |
| **Área usable** | Variable según vista | Fija: 200px (Y=20 a Y=219) |
| **Íconos** | Opcionales, diseños variados | Obligatorios (PING, NMAP, BLEEDING), especificaciones detalladas |
| **Posiciones X hints** | Cambiaban por estado | Fijas: x=4, 116, 188, 272 |

**Resumen:** El diseño actual elimina completamente los títulos y texto del header y footer, centralizando la comunicación visual en 3 íconos de sistema en el header y 4 hints de navegación fijos en el footer, mientras que el contenido del módulo se restringe al área usable central de 200px de altura.

EOF