# HunterZeroR - Documentación de la Interfaz de Módulos

## Versión 1.0 - Arquitectura Rust

Este documento describe la interfaz estándar que deben implementar todos los módulos del sistema HunterZeroR para ser compatibles con la arquitectura Rust.

---

## 📊 Tabla de Contenido

1. [Arquitectura General](#arqu-general)
2. [Estructura del Módulo](#estructura-del-módulo)
3. [Trait ToolScreen](#traitscreent)
4. [Fases de Configuración](#fases-de-configuración)
5. [Manejo de Entrada](#manejo-de-entrada)
6. [Footer Hints](#footer-hints)
7. [Cierre de Pantalla](#cierre-de-pantalla)
8. [Registro de Módulos](#registro-de-módulos)
9. [Agregar un Nuevo Módulo](#agregar-un-nuevo-módulo)
10. [Compatibilidad con Python](#compatibilidad-con-python)

---

## 🏗️ 1. Arquitectura General

La arquitectura de módulos sigue un patrón consistente:

```
┌─────────────────────────────────────────────────────────────┐
│                    HUNTERZERO-RUST                           │
├─────────────────────────────────────────────────────────────┤
│  Capa 1: Rust GUI (hunterzero-rust/)                        │
│  - Ventana 640x480 escalada desde 320x240 virtual           │
│  - Manejo de entrada D-pad/A/B/START/SELECT                │
│  - Renderizado pixel-art (Press Start 2P)                   │
│  - Estados: menú, módulo, ayuda, configuración, resultados │
│                                                             │
│  Capa 2: Módulos (Python o Rust)                            │
│  - Ejecutándose como subprocessos o modules puros Rust      │
│  - Comunicación: stdin/stdout con timeout                   │
│                                                             │
│  Capa 3: Bridge (Comunicación)                              │
│  - Rust lanza `python módulos/ping/screen.py` como subprocess│
│  - La salida del módulo Python se muestra en pantalla Rust   │
│  - Eventos del módulo reportan de vuelta a Rust              │
└─────────────────────────────────────────────────────────────┘
```

---

## 📦 2. Estructura del Módulo

Cada módulo debe existir en su propio directorio dentro de `modules/`:

```
modules/
├── ping/
│   ├── manifest.yaml    # Metadatos del módulo
│   └── screen.rs        # Lógica y UI del módulo (OBLIGATORIO)
└── bleeding/
    ├── manifest.yaml
    └── screen.rs
```

### Requisitos mínimos:
- **Directorio nombre debe coincidir con el `id` del módulo** en el manifest
- **Debe existir `screen.rs`** con la implementación del trait `ToolScreen`
- **Archivo `manifest.yaml`** opcional (si no existe, se usa identificador por defecto)

---

## 🎯 3. Trait ToolScreen

Todos los módulos deben implementar el trait `ToolScreen` que define la interfaz estándar:

```rust
trait ToolScreen: Send + 'static {
    /// Dibujar el contenido del módulo SOLO en el área usable
    /// ¡NO dibuja header ni footer! Eso se hace en main.rs para consistencia.
    fn draw_content(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()>;

    /// Manejar acciones de entrada del D-pad, A, B, START
    fn handle_action(&mut self, action: &Action) -> bool;

    /// Obtener hints para el footer (4 strings fijos consistentes en todas las vistas)
    fn footer_hints(&self) -> [&'static str; 4];

    /// Limpiar recursos al cerrar esta pantalla
    fn close(&mut self);
}
```

### Métodos detallados:

#### `draw_content(ctx, canvas)`
- **Ubicación**: Área usable (Y desde CONTENT_START_Y=20 hasta CONTENT_END_Y=220)
- **Responsabilidad**: Dibujar únicamente el contenido del módulo
- **Restricción**: No dibujar header ni footer (ellos se dibujan automáticamente)
- **Retorno**: `GameResult<()>` - Ok si el dibujo fue exitoso

#### `handle_action(action)`
- **Entrada**: `core::input::Action` - D-pad/A/B/START
- **Retorno**: `bool` - `true` si la acción fue consumida y no se necesita más procesamiento
- **Acciones soportadas**:
  - `Left`/`Right` - Navegar o cambiar modos
  - `Up`/`Down` - Aumentar/disminuir valor o cambiar fase
  - `Open` (A) - Confirmar/guardar/ejecutar
  - `Back` (B) - Cancelar, volver al menú

#### `footer_hints()`
- **Retorno**: `[&'static str; 4]` - Siempre los mismos 4 hints
- **Consistencia**: Deben ser idénticas en todas las vistas del mismo módulo
- **Ejemplo estándar**: `["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]`

#### `close()`
- **Responsabilidad**: Limpiar estado del módulo
- **Llamado automático**: Cuando el usuario presiona B para cancelar
- **Limpieza típica**: Resetear IP, contador, fase a valores por defecto

---

## ⚙️ 3. Fases de Configuración

Muchos módulos (especialmente ping y bleeding) usan un sistema de fases para guiar al usuario through configuración:

### Ejemplo de enum de fases:

```rust
enum ConfigPhase {
    /// Estado inicial - esperando input
    SelectingOctet,
    /// Modificando valor actual
    ModifyingValue,
    /// Listo para ejecutar
    Ready,
}
```

### Transiciones de fase típicas:

| Acción | Desde | Hasta | Comentario |
|--------|-------|-------|------------|
| A (Open) | SelectingOctet | Configuring | Avanzar a configuración |
| A (Open) | Configuring | Ready | Lista para ejecutar |
| B (Back) | Cualquier | Seleccionar inicial | Resetear todo |
| D-pad | Cualquier | Cualquier | Navegar valores |

---

## 🎮 4. Manejo de Entrada

El sistema normaliza todas las entradas a través de `core::input::Action`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Open,   // A (abrir/confirmar)
    Back,   // B (cancelar/volver)
    Start,  // START
    Quit,   // SDL_QUIT / Escape
    None,   // Sin acción
}
```

### Mapeo de hardware a actions:

| Tecla Physical | Action Rust | Comentario |
|---------------|-------------|------------|
| D-pad Up      | Up          | Mover cursor/valor arriba |
| D-pad Down    | Down        | Mover cursor/valor abajo |
| D-pad Left    | Left        | Anterior octeto/valor |
| D-pad Right   | Right       | Siguiente octeto/valor |
| A (Button)    | Open        | Confirmar/guardar/ejecutar |
| B (Button)    | Back        | Cancelar/volver al menú |
| START         | Start       | A veces mapeado a Open |
| Escape        | Quit        | Cerrar aplicación |

---

## 📋 5. Footer Hints

### Importancia:
Los footer hints son **fijos y consistentes** en todas las vistas de un módulo. El usuario siempre debe saber qué hacer presionando cada botón, sin importar en qué fase esté.

### Estándar actual:
```rust
fn footer_hints(&self) -> [&'static str; 4] {
    ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
}
```

### Restricciones:
- **Siempre 4 elementos** en el array
- **Strings literales** (`'static str`) - no Strings dinámicas
- **Consistencia visual** - Mismas posiciones siempre:
  - x=4: "D-PAD Mover"
  - x=116: "A Abrir"  
  - x=188: "START Ejecutar"
  - x=272: "B Volver"

### Ejemplo módulo Ping:
```rust
fn footer_hints(&self) -> [&'static str; 4] {
    ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
}
```

---

## 🚪 6. Cierre de Pantalla

### Cuando se llama:
- Usuario presiona **B** (Back) en cualquier vista del módulo
- El sistema está a punto de regresar al menú principal

### Responsabilidades de `close()`:

```rust
fn close(&mut self) {
    // 1. Limpiar IP o configuración temporal
    self.target_ip.clear();
    
    // 2. Resetear contador de paquetes
    self.packet_count = 2;
    
    // 3. Restablecer fase a inicial
    self.config_phase = ConfigPhase::SelectingOctet;
    
    // 4. Cerrar cualquier recurso abierto
    // (archivos, subprocessos, conexiones GPIO, etc.)
}
```

### Ejemplo módulo Bleeding:
```rust
fn close(&mut self) {
    self.target_device = String::new();
    self.mode = BleedingMode::Scan;
    self.packet_count = 1;
    self.phase = BleedingPhase::Scanning;
}
```

---

## 📋 6. Registro de Módulos

Los módulos se descubren automáticamente a través de `PluginCatalog`, que escanea directorios en `modules/`.

### Arquivo `manifest.yaml` (opcional pero recomendado):

```yaml
# Identificador, pantalla y metadatos registrados por PluginCatalog.
id: ping
name: Ping
description: Envía paquetes ICMP a una IPv4 privada introducida con el D-pad.
icon: ping
version: "0.1.0"
screen: screen.py
deps:
  - iputils-ping
```

### Campos del manifest:
- `id`: Identificador único (debe coincidir con nombre de directorio)
- `name`: Nombre mostrado en el menú
- `description`: Descripción corta (mostrada en ayuda)
- `icon`: Nombre de archivo SVG en `assets/icons/` (sin extensión)
- `version`: Versión del módulo
- `screen`: Nombre del archivo de pantalla (para compatibilidad Python)
- `deps`: Dependencias del sistema (paquetes APT)

### Si NO hay manifest.yaml:
El `PluginCatalog` escaneará los directorios y usará el nombre de la carpeta como `id`, intentando leer el `screen.rs` existente.

### Descubrimiento automático en `main.rs`:

```rust
let catalog = core::plugins::PluginCatalog::new(Path::new("modules"));
```

Esto escaneará automáticamente todos los subdirectorios de `modules/` y cargará aquellos que tengan un archivo `screen.rs` implementando el trait `ToolScreen`.

---

## ➕ 7. Agregar un Nuevo Módulo

### Paso a paso:

1. **Crear directorio**:
   ```bash
   mkdir modules/mi_modulo
   ```

2. **Crear `screen.rs`**:
   Copiar el patrón de un módulo existente (ping, bleeding, etc.) y modificar:
   - Nombre de la estructura
   - Enum de fases
   - Lógica de dibujo
   - Manejo de acciones
   - Footer hints

3. **Opcional: Crear `manifest.yaml`**:
   ```yaml
   id: mi_modulo
   name: Mi Módulo
   description: Descripción del módulo
   icon: modulo
   version: "1.0.0"
   ```

4. **Probar compilación**:
   ```bash
   cargo build --release
   ```

### Patrón mínimo viable (`screen.rs`):

```rust
use core::input::Action;
use ggez::graphics;
use ggez::Context;
use crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR};

enum Phase {
    Starting,
    Ready,
}

pub struct MiModuloScreen {
    variable1: String,
    variable2: u32,
    phase: Phase,
}

impl MiModuloScreen {
    pub fn new() -> Self {
        Self {
            variable1: String::new(),
            variable2: 0,
            phase: Phase::Starting,
        }
    }
}

impl ToolScreen for MiModuloScreen {
    fn draw_content(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        canvas.set_color(BG_COLOR);
        canvas.clear();
        
        let title = graphics::Text::new(ctx, "MI MÓDULO", 8)?;
        title.draw(ctx, graphics::Point::new(10.0, 25.0), ACCENT_COLOR)?;
        
        // ... más dibujo
        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> bool {
        match action {
            Left => { /* cambiar fase anterior */ true },
            Right => { /* cambiar fase siguiente */ true },
            Up => { /* aumentar valor */ true },
            Down => { /* disminuir valor */ true },
            Open => { /* ejecutar/confirmar */ false },
            Back => { /* resetear y volver */ true },
            _ => true,
        }
    }

    fn footer_hints(&self) -> [&'static str; 4] {
        ["D-PAD Mover", "A Abrir", "START Ejecutar", "B Volver"]
    }

    fn close(&mut self) {
        self.variable1 = String::new();
        self.variable2 = 0;
        self.phase = Phase::Starting;
    }
}
```

---

## 🐍 8. Compatibilidad con Python

### Opción A: Módulos puros Rust (recomendado)
- Ventajas: Mejor rendimiento, sin dependencias Python
- Desventaja: Requiere reescribir la lógica

### Opción B: Módulos Python (puente híbrido)
- El sistema sigue lanzando subprocessos Python
- La UI Rust muestra la salida del módulo Python
- Sin reescritura necesaria

### Ejemplo lanzando módulo Python desde Rust:

```rust
fn start_module_python(module_path: &str) {
    let child = std::process::Command::new("python")
        .args(["-u", module_path])
        .stdout(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start module");
    
    // Leer salida mientras el proceso corre
    // Actualizar pantalla Rust con los resultados
    // Cuando child.wait() termine, regresar al menú
}
```

### Flujo híbrido typical:

1. Usuario selecciona módulo en menú Rust
2. Presiona **A** (Open)
3. Rust lanza: `python modules/ping/screen.py`
4. Se oculta la interfaz Rust principal
5. Se muestra la salida del Python (o una pantalla de "ejecutando...")
6. Cuando Python termina, se regresa al menú Rust

---

## 📦 9. Dependencias del Sistema

Cada módulo puede declarar sus dependencias en `manifest.yaml`:

```yaml
deps:
  - iputils-ping    # Para módulo ping
  - nmap            # Para módulo nmap
  - dnsutils        # Para módulo nslookup
  - bluetooth-tools # Para módulo bleeding
```

Estas dependencias se instalan automáticamente en el script `deploy/install.sh` basándose en los campos `deps:` de los manifiestos de los módulos.

---

## 🔧 10. Convenios de Código

### Nombres de archivos:
- Siempre `screen.rs` en el directorio del módulo
- Nombre de estructura: `NombreModuloScreen` (PascalCase)
- Enum de fases: `Phase` o `ConfigPhase` (dependiendo del módulo)

### Nombres de variables:
- `target_ip`, `target_device`, etc. - nombres descriptivos
- `packet_count`, `attempts`, `progress` - contadores numéricos
- `phase`, `config_phase`, `state` - estados del sistema

### Colores:
- Usar siempre `crate::theme::{BG_COLOR, TEXT_COLOR, ACCENT_COLOR}`
- Nunca hardcodear colores RGB directamente

### Font size:
- Tamaño 8 para texto principal
- Tamaño 7 para instrucciones y footer hints
- Tamaño 6 o menor para textos pequeños

---

## 🆚 Diferencias vs. Versión Python

| Característica | **Versión Python** | **Versión Rust** |
|--------------|-------------------|------------------|
| **Punto de entrada** | `python main.py` | `cargo run` o `./target/release/hunterzero-rust` |
| **GUI framework** | Pygame + SDL2 | ggez + SDL2 atrás |
| **Rendimiento** | Interpretado, overhead de Python | Compilado nativo, optimizado |
| **Configuración GPIO** | `gpiod` / `RPi.GPIO` | `rppal` crate nativo |
| **Deploy** | `sudo bash install.sh` | `cargo build --release` + deploy script |
| **Módulos** | Python puros | Rust + Python híbrido |
| **Íconos** | PNG normales | SVG (escalables, 20 icons genéricos) |
| **Config.txt** | Aplicado manualmente | Leído automáticamente al iniciar |

---

## 📡 11. Preguntas Frecuentes (FAQ)

### P: ¿Necesito saber Rust para agregar un módulo?
R: No necesariamente. Puedes:
- **Modificar módulos existentes** en Rust
- **Usar módulos Python** sin saber Rust
- **Aprender Rust gradualmente** leyendo el código existente

### P: ¿Puedo tener ambos, Rust y Python, para el mismo módulo?
R: Sí. El sistema híbrido permite que un módulo tenga una versión Rust y otra Python, y el usuario pueda elegir cuál cargar.

### P: ¿Cómo depuro un módulo que da error?
R:
1. Revisar la consola del terminal (`cargo run` muestra errores)
2. Usar `println!` para debugueo temporal (remover antes de build release)
3. Revisar que `screen.rs` compile: `cargo check`
4. Verificar que `manifest.yaml` tenga campos correctos

### P: ¿Los módulos Rust usan más memoria que Python?
R: Al contrario - Rust suele usar **menos memoria** y tener **menor overhead** que Python, lo cual es notable en Raspberry Pi Zero con recursos limitados.

---

## 📜 Licencia

Este proyecto y documentación están bajo la licencia **MIT**, igual que el proyecto original HunterZero.

---

## 📬 Contacto

Para dudas sobre la implementación de módulos:
- Revisar el archivo `MODULARIZACION.md` original
- Examinar los módulos existentes en `modules/`
- Consultar la documentación de `core/plugins.rs`
- Abrir un issue en el repositorio

---
*Última actualización: 25 de noviembre de 2026*
*Generado para HunterZero-Rust v0.1.0*