//! Live telemetry and process sampling.
//!
//! On Linux every reading comes from procfs/sysfs, exactly like the Python
//! implementation, so the Pi build needs no extra native library. On Windows
//! (development only) `sysinfo` provides the same fields where the operating
//! system exposes them; missing sensors are reported as `None`, never invented.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use chrono::Local;

/// One reading shown by the HUD and the system monitor.
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_percent: u32,
    pub memory_percent: u32,
    pub temperature_c: Option<i32>,
    pub battery_percent: Option<u32>,
    pub battery_charging: bool,
    pub clock: String,
    pub wifi_connected: bool,
    pub local_ip: String,
    pub ssid: Option<String>,
    pub disk_percent: u32,
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            cpu_percent: 0,
            memory_percent: 0,
            temperature_c: None,
            battery_percent: None,
            battery_charging: false,
            clock: "--:--".to_string(),
            wifi_connected: false,
            local_ip: "N/D".to_string(),
            ssid: None,
            disk_percent: 0,
        }
    }
}

const NETWORK_REFRESH_SECONDS: u64 = 10;

/// Stateful reader that keeps the previous CPU sample and network cache.
pub struct StatsReader {
    previous_cpu: Option<(u64, u64)>,
    network_cache: Option<(String, Option<String>)>,
    network_read_at: Option<Instant>,
    #[cfg(windows)]
    system: sysinfo::System,
}

impl StatsReader {
    pub fn new() -> Self {
        #[cfg(windows)]
        let system = {
            let mut system = sysinfo::System::new();
            system.refresh_memory();
            system.refresh_cpu_usage();
            system
        };
        Self {
            previous_cpu: None,
            network_cache: None,
            network_read_at: None,
            #[cfg(windows)]
            system,
        }
    }

    /// Read the current values. Call it every few seconds, not every frame.
    pub fn read(&mut self) -> SystemStats {
        let (cpu_percent, memory_percent) = self.cpu_and_memory();

        let now = Instant::now();
        let refresh_network = self
            .network_read_at
            .map(|last| now.duration_since(last) >= Duration::from_secs(NETWORK_REFRESH_SECONDS))
            .unwrap_or(true);
        if refresh_network {
            let current = (local_ip_address(), wireless_ssid());
            self.network_read_at = Some(now);
            self.network_cache = Some(current);
        }
        let (local_ip, ssid) = self
            .network_cache
            .clone()
            .unwrap_or_else(|| ("N/D".to_string(), None));

        SystemStats {
            cpu_percent,
            memory_percent,
            temperature_c: read_temperature(),
            battery_percent: read_battery(),
            battery_charging: read_battery_charging(),
            clock: Local::now().format("%H:%M").to_string(),
            wifi_connected: ssid.is_some() || wifi_operstate_up(),
            local_ip,
            ssid,
            disk_percent: disk_usage_percent(),
        }
    }

    #[cfg(target_os = "linux")]
    fn cpu_and_memory(&mut self) -> (u32, u32) {
        let cpu_percent = match read_cpu_times() {
            Some((total, idle)) => {
                let percent = match self.previous_cpu {
                    Some((previous_total, previous_idle))
                        if total > previous_total && idle >= previous_idle =>
                    {
                        let total_delta = total - previous_total;
                        let idle_delta = idle - previous_idle;
                        (((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0)
                            .round()
                            .clamp(0.0, 100.0) as u32
                    }
                    _ => 0,
                };
                self.previous_cpu = Some((total, idle));
                percent
            }
            None => 0,
        };
        (cpu_percent, read_memory_percent().unwrap_or(0))
    }

    #[cfg(windows)]
    fn cpu_and_memory(&mut self) -> (u32, u32) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        let cpu_percent = self
            .system
            .global_cpu_info()
            .cpu_usage()
            .round()
            .clamp(0.0, 100.0) as u32;
        let total = self.system.total_memory();
        let memory_percent = if total == 0 {
            0
        } else {
            ((self.system.used_memory() as f64 / total as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u32
        };
        (cpu_percent, memory_percent)
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    fn cpu_and_memory(&mut self) -> (u32, u32) {
        (0, 0)
    }
}

impl Default for StatsReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Return the address of the interface used for outbound traffic.
///
/// A UDP socket only selects a route; it sends no packets.
pub fn local_ip_address() -> String {
    use std::net::UdpSocket;
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return "N/D".to_string(),
    };
    if socket.connect("8.8.8.8:80").is_err() {
        return "N/D".to_string();
    }
    socket
        .local_addr()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|_| "N/D".to_string())
}

#[cfg(target_os = "linux")]
fn read_cpu_times() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let mut fields = line.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values: Vec<u64> = fields.filter_map(|value| value.parse().ok()).collect();
    if values.len() < 4 {
        return None;
    }
    let total: u64 = values.iter().sum();
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    Some((total, idle))
}

#[cfg(target_os = "linux")]
fn read_memory_percent() -> Option<u32> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = 0u64;
    let mut available = 0u64;
    let mut free = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        let value: u64 = parts.next()?.parse().ok()?;
        match key {
            "MemTotal:" => total = value,
            "MemAvailable:" => available = value,
            "MemFree:" => free = value,
            "Buffers:" => buffers = value,
            "Cached:" => cached = value,
            _ => {}
        }
    }
    if total == 0 {
        return None;
    }
    if available == 0 {
        available = free + buffers + cached;
    }
    Some(
        (((total.saturating_sub(available)) as f64 / total as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32,
    )
}

#[cfg(target_os = "linux")]
fn wifi_operstate_up() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("wlan") {
            continue;
        }
        if let Ok(state) = std::fs::read_to_string(entry.path().join("operstate")) {
            if state.trim() == "up" {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn wifi_operstate_up() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn wireless_ssid() -> Option<String> {
    for interface in wireless_interfaces() {
        let commands: [&[&str]; 3] = [
            &["iw", "dev", &interface, "link"],
            &["iwgetid", "-r"],
            &["nmcli", "-t", "-f", "active,ssid", "dev", "wifi"],
        ];
        for command in commands {
            let output = Command::new(command[0]).args(&command[1..]).output();
            let Ok(output) = output else { continue };
            if !output.status.success() {
                continue;
            }
            let text = String::from_utf8_lossy(&output.stdout);
            for raw_line in text.lines() {
                let line = raw_line.trim();
                if line.is_empty() {
                    continue;
                }
                let candidate = match command[0] {
                    "iw" => line
                        .strip_prefix("SSID:")
                        .map(|value| value.trim().to_string())
                        .unwrap_or_default(),
                    "iwgetid" => line.rsplit(':').next().unwrap_or(line).trim().to_string(),
                    "nmcli" => {
                        let (active, ssid) = line.split_once(':').unwrap_or(("", ""));
                        if active == "yes" {
                            ssid.trim().to_string()
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                };
                if !candidate.is_empty() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn wireless_ssid() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn wireless_interfaces() -> Vec<String> {
    let mut interfaces: Vec<String> = std::fs::read_dir("/sys/class/net")
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.starts_with("wlan").then_some(name)
                })
                .collect()
        })
        .unwrap_or_default();
    interfaces.sort();
    if interfaces.is_empty() {
        interfaces.push("wlan0".to_string());
    }
    interfaces
}

#[cfg(target_os = "linux")]
fn read_temperature() -> Option<i32> {
    for path in [
        "/sys/class/thermal/thermal_zone0/temp",
        "/sys/class/hwmon/hwmon0/temp1_input",
    ] {
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(milli) = raw.trim().parse::<i64>() {
                return Some((milli / 1000) as i32);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_temperature() -> Option<i32> {
    None
}

#[cfg(target_os = "linux")]
fn read_battery() -> Option<u32> {
    for entry in std::fs::read_dir("/sys/class/power_supply").ok()? {
        let path = entry.ok()?.path().join("capacity");
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(value) = raw.trim().parse::<u32>() {
                return Some(value.min(100));
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_battery() -> Option<u32> {
    None
}

#[cfg(target_os = "linux")]
fn read_battery_charging() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path().join("status");
        if let Ok(raw) = std::fs::read_to_string(path) {
            if raw.trim() == "Charging" {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn read_battery_charging() -> bool {
    false
}

/// Used percentage of the filesystem containing the application.
#[cfg(target_os = "linux")]
pub fn disk_usage_percent() -> u32 {
    let path = std::ffi::CString::new("/").unwrap_or_default();
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };
    if result != 0 || stats.f_blocks == 0 {
        return 0;
    }
    let used = stats.f_blocks.saturating_sub(stats.f_bfree);
    ((used as f64 / stats.f_blocks as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u32
}

#[cfg(windows)]
pub fn disk_usage_percent() -> u32 {
    use sysinfo::{DiskExt, Disks};
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .iter()
        .find(|disk| disk.mount_point() == Path::new("C:\\"))
        .or_else(|| disks.iter().next());
    match disk {
        Some(disk) => {
            let total = disk.total_space();
            if total == 0 {
                0
            } else {
                let used = total.saturating_sub(disk.available_space());
                ((used as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32
            }
        }
        None => 0,
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn disk_usage_percent() -> u32 {
    0
}

/// One row of the process monitor.
#[derive(Debug, Clone)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub elapsed: String,
}

/// Stateful process sampler. CPU percentages need two refreshes separated by a
/// short interval, so the screen keeps one sampler alive.
pub struct ProcessSampler {
    #[cfg(target_os = "linux")]
    previous_ticks: std::collections::HashMap<u32, f64>,
    #[cfg(target_os = "linux")]
    previous_time: Option<Instant>,
    #[cfg(windows)]
    system: sysinfo::System,
}

impl ProcessSampler {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            previous_ticks: std::collections::HashMap::new(),
            #[cfg(target_os = "linux")]
            previous_time: None,
            #[cfg(windows)]
            system: {
                let mut system = sysinfo::System::new();
                system.refresh_memory();
                system.refresh_processes();
                system
            },
        }
    }

    #[cfg(target_os = "linux")]
    pub fn snapshot(&mut self) -> Vec<ProcessRow> {
        let now = Instant::now();
        let interval = self
            .previous_time
            .map(|previous| now.duration_since(previous).as_secs_f64())
            .unwrap_or(0.0);
        let (total_memory, uptime) = linux_memory_and_uptime();
        let mut rows = Vec::new();
        let mut current_ticks = std::collections::HashMap::new();

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(pid) = name.to_string_lossy().parse::<u32>().ok() else {
                    continue;
                };
                let Ok(raw) = std::fs::read_to_string(entry.path().join("stat")) else {
                    continue;
                };
                let Some(open) = raw.find('(') else { continue };
                let Some(close) = raw.rfind(')') else {
                    continue;
                };
                if close <= open {
                    continue;
                }
                let fields: Vec<&str> = raw[close + 1..].split_whitespace().collect();
                if fields.len() < 22 {
                    continue;
                }
                let parse = |index: usize| fields[index].parse::<f64>().ok();
                let (Some(utime), Some(stime), Some(starttime), Some(rss)) =
                    (parse(11), parse(12), parse(19), parse(21))
                else {
                    continue;
                };
                let cpu_seconds = (utime + stime) / clock_ticks_per_second();
                let cpu_percent = self
                    .previous_ticks
                    .get(&pid)
                    .filter(|_| interval > 0.0)
                    .map(|previous| (((cpu_seconds - previous) / interval) * 100.0).max(0.0) as f32)
                    .unwrap_or(0.0);
                let memory_percent = if total_memory > 0.0 {
                    (rss * page_size() / total_memory * 100.0) as f32
                } else {
                    0.0
                };
                let elapsed = (uptime - starttime / clock_ticks_per_second()).max(0.0);
                let process_name = raw[open + 1..close].to_string();
                current_ticks.insert(pid, cpu_seconds);
                rows.push(ProcessRow {
                    pid,
                    name: if process_name.is_empty() {
                        format!("pid{pid}")
                    } else {
                        process_name
                    },
                    cpu_percent,
                    memory_percent,
                    elapsed: format_elapsed(elapsed as u64),
                });
            }
        }

        self.previous_ticks = current_ticks;
        self.previous_time = Some(now);
        rows.sort_by(|left, right| {
            right
                .cpu_percent
                .partial_cmp(&left.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.pid.cmp(&right.pid))
        });
        rows.truncate(24);
        rows
    }

    #[cfg(windows)]
    pub fn snapshot(&mut self) -> Vec<ProcessRow> {
        self.system.refresh_memory();
        self.system.refresh_processes();
        let total_memory = self.system.total_memory().max(1);
        let mut rows: Vec<ProcessRow> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessRow {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_percent: process.cpu_usage(),
                memory_percent: (process.memory() as f64 / total_memory as f64 * 100.0) as f32,
                elapsed: format_elapsed(process.run_time()),
            })
            .collect();
        rows.sort_by(|left, right| {
            right
                .cpu_percent
                .partial_cmp(&left.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.pid.cmp(&right.pid))
        });
        rows.truncate(24);
        rows
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    pub fn snapshot(&mut self) -> Vec<ProcessRow> {
        Vec::new()
    }
}

impl Default for ProcessSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
fn clock_ticks_per_second() -> f64 {
    let value = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if value > 0 {
        value as f64
    } else {
        100.0
    }
}

#[cfg(target_os = "linux")]
fn page_size() -> f64 {
    let value = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };
    if value > 0 {
        value as f64
    } else {
        4096.0
    }
}

#[cfg(target_os = "linux")]
fn linux_memory_and_uptime() -> (f64, f64) {
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| {
            content
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    let total_memory = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                if line.starts_with("MemTotal:") {
                    line.split_whitespace()
                        .nth(1)
                        .and_then(|value| value.parse::<f64>().ok())
                        .map(|kilobytes| kilobytes * 1024.0)
                } else {
                    None
                }
            })
        })
        .unwrap_or(0.0);
    (total_memory, uptime)
}

fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// Snapshot written to `state/heartbeats/app.json`.
pub fn heartbeat_payload() -> serde_json::Value {
    serde_json::json!({
        "application": "hunterzero-rust",
        "pid": std::process::id(),
        "status": "running",
        "updated_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_formats_like_the_python_monitor() {
        assert_eq!(format_elapsed(5), "5s");
        assert_eq!(format_elapsed(65), "1m 05s");
        assert_eq!(format_elapsed(3665), "1h 01m");
    }

    #[test]
    fn local_ip_is_never_empty() {
        assert!(!local_ip_address().is_empty());
    }
}
