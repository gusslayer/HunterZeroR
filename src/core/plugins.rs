//! Static module registry.
//!
//! The Python version discovered manifests at runtime and imported a Python
//! screen by name. In Rust every screen is linked into the binary, so the
//! registry is a compile-time table: a typo becomes a compile error instead of
//! a blank menu entry.

#[derive(Debug, Clone, Copy)]
pub struct ModuleInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

pub const MODULES: &[ModuleInfo] = &[
    ModuleInfo {
        id: "bleeding",
        name: "Bleeding",
        description: "Bluetooth LE / BR-EDR scanning, enumeration and bounded DeAuth.",
        icon: "bleeding",
    },
    ModuleInfo {
        id: "logs",
        name: "Logs",
        description: "Visor de logs de aplicacion y ultimo comando ejecutado.",
        icon: "file",
    },
    ModuleInfo {
        id: "nmap",
        name: "Nmap",
        description: "Escaneo TCP connect con scripts NSE y opciones acotadas.",
        icon: "nmap",
    },
    ModuleInfo {
        id: "nslookup",
        name: "Nslookup",
        description: "Resolucion DNS interactiva con editor de dominios.",
        icon: "nslookup",
    },
    ModuleInfo {
        id: "ping",
        name: "Ping",
        description: "Envia paquetes ICMP a una IPv4 privada introducida con el D-pad.",
        icon: "ping",
    },
    ModuleInfo {
        id: "proc_monitor",
        name: "Proc Monitor",
        description: "Lista de procesos activos ordenada por uso de CPU.",
        icon: "proc_monitor",
    },
    ModuleInfo {
        id: "system_monitor",
        name: "System Monitor",
        description: "Metricas de CPU, memoria, temperatura, disco, bateria y red.",
        icon: "system_monitor",
    },
    ModuleInfo {
        id: "update",
        name: "Actualizar HunterZero",
        description: "Descarga el codigo con git pull --ff-only y muestra el resultado.",
        icon: "update",
    },
];
