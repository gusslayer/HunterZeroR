# HunterZero-Rust

**HunterZero-Rust** - Migración a Rust del proyecto HunterZero para Raspberry Pi Zero W v1.1 con PiOS Lite.

## 📖 Descripción

Proyecto de código abierto que migra la interfaz HunterZero a Rust, manteniendo compatibilidad total con la versión Python original. Diseñado para ejecutarse en **Raspberry Pi Zero W v1.1** con **Raspberry Pi OS Lite (32-bit armhf)**.

El proyecto combina una interfaz gráfica Rust (ggez/SDL2) con módulos Python ejecutándose como subprocessos, permitiendo una migración gradual de módulos críticos manteniendo la compatibilidad con el ecosistema Python existente.

## 🎯 Características Principales

- ✅ **Interfaz Rust nativa** - Rendimiento optimizado para Raspberry Pi
- ✅ **Configuración automática GPi Case 2W** - Lee y valida `/boot/firmware/config.txt`
- ✅ **Soporte GPIO 26** - Botón físico de apagado seguro
- ✅ **7 módulos migrados** a Rust (ping, bleeding, nmap, system_monitor, proc_monitor, nslookup, update)
- ✅ **Arquitectura híbrida** - Módulos Python ejecutándose como subprocessos
- ✅ **20 iconos SVG** genéricos para nuevos módulos
- ✅ **Diseño UI consistente** - 320×240 virtual → 640×480 escalado
- ✅ **Footer hints fijos** - Siempre los mismos 4 hints (D-PAD Mover, A Abrir, START Ejecutar, B Volver)

## 📸 Capturas de Interfaz

<p align="center">
  <img src="assets/icons/ping.svg" width="64" height="64" alt="Ping Module Icon">
  <img src="assets/icons/bleeding.svg" width="64" height="64" alt="Bleeding Module Icon">
  <img src="assets/icons/nmap.svg" width="64" height="64" alt="Nmap Module Icon">
</p>

<p align="center">
  <em>Interfaz principal con módulos migrados a Rust</em>
</p>

## 🚀 Quick Start

### En Raspberry Pi Zero W:

```bash
# 1. Clonar el repositorio
git clone https://github.com/gusslayer/HunterZeroR.git
cd HunterZeroR

# 2. Ejecutar instalador (como root)
sudo bash deploy/install.sh

# 3. Reiniciar la Pi
sudo reboot

# 4. Habilitar el servicio
sudo systemctl enable --now hunterzero.service
```

### En PC (desarrollo):

```bash
# Ejecutar en modo debug
cargo run

# Compilar para release
cargo build --release
```

## 📦 Módulos Disponibles

| Módulo | Estado | Descripción |
|--------|--------|-------------|
| **ping** | ✅ Rust | ICMP Echo requests manuales |
| **bleeding** | ✅ Rust | Bluetooth scanning/deauth |
| **nmap** | ✅ Rust | Escaneo de puertos TCP |
| **system_monitor** | ✅ Rust | Métricas CPU/memoria/temperatura |
| **proc_monitor** | ✅ Rust | Lista de procesos activos |
| **nslookup** | ✅ Rust | Resolución DNS interactiva |
| **update** | ✅ Rust | Actualización de la aplicación |

## 🛠️ Arquitectura

### Capa 1: Rust GUI (hunterzero-rust/)
- Ventana principal 640x480 escalada desde 320×240 virtual
- Manejo de entrada D-pad/A/B/START
- Renderizado con estética pixel-art (Press Start 2P)
- Gestión de estados (menú, módulos, ayuda, configuración, resultados)

### Capa 2: Módulos Python (addons)
- Los módulos originales se mantienen intactos
- Se comunican con Rust a través de **subprocessos**
- Cada módulo es independiente y no necesita ser reescrito en Rust

### Capa 3: Bridge (Comunicación)
- Rust inicia intérprete de módulos Python como subprocess
- La salida del módulo Python se muestra en la pantalla Rust
- Los eventos del módulo (fin, cancelación) se reportan de vuelta a Rust

## 📁 Estructura del Repositorio

```
hunterzero-rust/
├── Cargo.toml          # Dependencias y configuración Rust
├── src/
│   └── main.rs         # Punto de entrada principal
├── assets/
│   ├── fonts/          # PressStart2P-Regular.ttf (por instalar)
│   └── icons/          # 20 iconos SVG para módulos
├── modules/
│   ├── ping/          # Módulo Ping (Rust)
│   ├── bleeding/      # Módulo Bleeding (Rust)
│   ├── nmap/          # Módulo Nmap (Rust)
│   ├── system_monitor/ # Módulo System Monitor (Rust)
│   ├── proc_monitor/  # Módulo Proc Monitor (Rust)
│   ├── nslookup/      # Módulo NSLookup (Rust)
│   └── update/        # Módulo Update (Rust)
├── deploy/            # Scripts de despliegue Raspberry Pi
│   ├── config.txt     # Configuración GPi Case 2W
│   ├── install.sh     # Instalación completa
│   └── display-config.sh # Configuración de pantalla
└── MODULARIZACION.md  # Documentación de modularización
```

## 🛠️ Desarrollo

### Agregar un nuevo módulo:

1. Crear `modules/nuevo_modulo/screen.rs` siguiendo el patrón de los módulos existentes
2. Agregar `modules/nuevo_modulo/` a `Cargo.toml` si es necesario
3. El módulo se registrará automáticamente a través de `PluginCatalog`

### Compilar y probar:

```bash
# En PC para pruebas
cargo build --release

# En Raspberry Pi para despliegue
ssh pi@192.168.1.x "cargo build --release"
```

## 📄 Licencia

Este proyecto está bajo la licencia [MIT](LICENSE) - igual que el proyecto original HunterZero.

## 🙏 Agradecimientos

- **gusslayer** - Autor del repositorio original HunterZero
- **Flipper Zero** - Concepto de dispositivo de hacking portátil
- **Raspberry Pi Foundation** - Hardware plataforma
- **ggez community** - Framework Rust para la interfaz gráfica

---

*Para más información, consulta el archivo [MODULARIZACION.md](MODULARIZACION.md) para el plan detallado de modularización.*