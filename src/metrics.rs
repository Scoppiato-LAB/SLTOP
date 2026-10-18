use crossbeam_channel::Sender;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    ProcessesToUpdate, RefreshKind, System, UpdateKind, Users,
};

use crate::containers::{ContainerTracker, ProcessGroupDomain};
use crate::gpu::{GpuCollector, GpuMetric};
use crate::psi::PsiMetrics;

/// Maximum historical data points stored for sparklines and braille graphs
const MAX_HISTORY_LEN: usize = 512;

/// Information for an individual CPU core
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CoreMetric {
    pub id: usize,
    pub name: String,
    pub usage: f32,
    pub frequency_mhz: u64,
    pub temp_celsius: Option<f32>,
}

/// Information for a single mounted filesystem
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiskMetric {
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub used_percent: f32,
    pub fs_type: String,
}

/// Information for a single network interface
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InterfaceMetric {
    pub name: String,
    pub rx_bytes_sec: u64,
    pub tx_bytes_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

/// Network traffic snapshot
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NetworkMetric {
    pub interface_name: String,
    pub rx_bytes_sec: u64,
    pub tx_bytes_sec: u64,
    pub peak_rx_sec: u64,
    pub peak_tx_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub rx_history: Vec<u64>,
    pub tx_history: Vec<u64>,
}

/// Detailed information for a single process
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessMetric {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub status: String,
    pub parent_pid: Option<u32>,
    pub cpu_usage: f32,
    pub cpu_delta: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub user: String,
    pub is_kernel_thread: bool,
    pub disk_read_bytes_sec: u64,
    pub disk_written_bytes_sec: u64,
    pub domain: ProcessGroupDomain,
    pub nice: i32,
}

/// Aggregated system snapshot sent from the background collector
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemMetrics {
    pub cpu_global_usage: f32,
    pub cpu_cores: Vec<CoreMetric>,
    pub cpu_temp_celsius: Option<f32>,
    pub cpu_history: Vec<u64>,
    pub cpu_model: String,
    pub cpu_arch: String,
    pub kernel_version: String,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_free: u64,
    pub mem_available: u64,
    pub mem_percent: f32,
    pub mem_history: Vec<u64>,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub swap_percent: f32,
    pub disks: Vec<DiskMetric>,
    pub network: NetworkMetric,
    pub interfaces: Vec<InterfaceMetric>,
    pub per_interface: HashMap<String, NetworkMetric>,
    pub gpu: Option<GpuMetric>,
    pub uptime_seconds: u64,
    pub load_avg: (f64, f64, f64),
    pub host_name: String,
    pub os_name: String,
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub sleeping_tasks: usize,
    pub processes: Vec<ProcessMetric>,
    pub psi: Option<PsiMetrics>,
    pub surge_process: Option<(String, u32, f32)>,
}

/// Background worker that periodically polls sysinfo and emits snapshots
pub struct MetricsCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    users: Users,
    container_tracker: ContainerTracker,
    cached_domain_map: HashMap<u32, (ProcessGroupDomain, Instant)>,
    prev_cpu_map: HashMap<u32, f32>,
    latest_surge: Option<(String, u32, f32)>,
    cpu_history: VecDeque<u64>,
    mem_history: VecDeque<u64>,
    net_rx_history: VecDeque<u64>,
    net_tx_history: VecDeque<u64>,
    iface_histories: HashMap<String, (VecDeque<u64>, VecDeque<u64>, u64, u64)>,
    gpu_collector: GpuCollector,
    cached_processes: Vec<ProcessMetric>,
    cached_disks: Vec<DiskMetric>,
    cached_psi: Option<PsiMetrics>,
    last_sample_time: Instant,
    last_proc_refresh: Instant,
    last_disk_refresh: Instant,
    last_temp_refresh: Instant,
    last_psi_refresh: Instant,
    cached_cpu_temp: Option<f32>,
    smoothed_global_cpu: f32,
    smoothed_cores_cpu: Vec<f32>,
    peak_rx_sec: u64,
    peak_tx_sec: u64,
    cpu_model: String,
    cpu_arch: String,
    kernel_version: String,
}

fn read_thermal_zone_temp() -> Option<f32> {
    if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().map(|n| n.to_string_lossy().starts_with("thermal_zone")).unwrap_or(false) {
                let temp_file = path.join("temp");
                if let Ok(content) = std::fs::read_to_string(temp_file) {
                    if let Ok(milli) = content.trim().parse::<f32>() {
                        if milli > 0.0 && milli < 150000.0 {
                            return Some(milli / 1000.0);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Reads Linux /proc/<pid>/stat to extract the nice priority value (field 19)
pub fn read_proc_nice(pid: u32) -> i32 {
    let stat_file = format!("/proc/{}/stat", pid);
    if let Ok(content) = std::fs::read_to_string(stat_file) {
        if let Some(pos) = content.rfind(')') {
            let rest = &content[pos + 1..];
            let mut iter = rest.split_whitespace();
            // rest starts at field 3 (state), so field 19 is item 16 (0-indexed)
            if let Some(nice_str) = iter.nth(16) {
                if let Ok(nice) = nice_str.parse::<i32>() {
                    return nice;
                }
            }
        }
    }
    0
}

impl MetricsCollector {
    pub fn new() -> Self {
        let proc_refresh_kind = ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage()
            .with_user(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet);

        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(proc_refresh_kind);

        let mut sys = System::new_with_specifics(refresh_kind);
        let disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let users = Users::new_with_refreshed_list();
        let container_tracker = ContainerTracker::new();

        sys.refresh_cpu_all();
        sys.refresh_memory();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        networks.refresh(true);

        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Generic CPU".to_string());

        let cpu_arch = System::cpu_arch();
        let kernel_version = System::kernel_version().unwrap_or_else(|| "Linux".to_string());

        Self {
            sys,
            disks,
            networks,
            components,
            users,
            container_tracker,
            cached_domain_map: HashMap::new(),
            prev_cpu_map: HashMap::new(),
            latest_surge: None,
            cpu_history: VecDeque::with_capacity(MAX_HISTORY_LEN),
            mem_history: VecDeque::with_capacity(MAX_HISTORY_LEN),
            net_rx_history: VecDeque::with_capacity(MAX_HISTORY_LEN),
            net_tx_history: VecDeque::with_capacity(MAX_HISTORY_LEN),
            iface_histories: HashMap::new(),
            gpu_collector: GpuCollector::new(),
            cached_processes: Vec::new(),
            cached_disks: Vec::new(),
            cached_psi: None,
            last_sample_time: Instant::now(),
            last_proc_refresh: Instant::now() - Duration::from_secs(10),
            last_disk_refresh: Instant::now() - Duration::from_secs(10),
            last_temp_refresh: Instant::now() - Duration::from_secs(10),
            last_psi_refresh: Instant::now() - Duration::from_secs(10),
            cached_cpu_temp: None,
            smoothed_global_cpu: 0.0,
            smoothed_cores_cpu: Vec::new(),
            peak_rx_sec: 0,
            peak_tx_sec: 0,
            cpu_model,
            cpu_arch,
            kernel_version,
        }
    }

    /// Spawns a dedicated background thread for metrics sampling
    pub fn spawn(
        sample_interval_ms: Arc<AtomicU64>,
        tx: Sender<SystemMetrics>,
        running: Arc<AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("sltop-sampler".to_string())
            .spawn(move || {
                let mut collector = Self::new();
                thread::sleep(Duration::from_millis(100));

                while running.load(Ordering::Relaxed) {
                    let snapshot = collector.sample();
                    if tx.send(snapshot).is_err() {
                        break;
                    }

                    let interval = Duration::from_millis(sample_interval_ms.load(Ordering::Relaxed).max(100));
                    let sleep_step = Duration::from_millis(10);
                    let mut elapsed = Duration::ZERO;
                    while elapsed < interval && running.load(Ordering::Relaxed) {
                        thread::sleep(sleep_step);
                        elapsed += sleep_step;
                    }
                }
            })
            .expect("Failed to spawn metrics sampler thread")
    }

    /// Performs one sampling cycle and produces a `SystemMetrics` struct
    pub fn sample(&mut self) -> SystemMetrics {
        let now = Instant::now();
        let delta_secs = now.duration_since(self.last_sample_time).as_secs_f64().max(0.01);
        self.last_sample_time = now;

        // Refresh CPU, memory, and network stats
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.networks.refresh(true);

        // Linux Pressure Stall Information (PSI) every 500ms
        if self.last_psi_refresh.elapsed() >= Duration::from_millis(500) {
            self.cached_psi = PsiMetrics::read();
            self.last_psi_refresh = now;
        }

        // Thermal check every 2 seconds
        if self.last_temp_refresh.elapsed() >= Duration::from_secs(2) {
            self.components.refresh(true);
            let mut found_temp: Option<f32> = None;
            for c in &self.components {
                if let Some(t) = c.temperature() {
                    found_temp = Some(found_temp.map(|m| m.max(t)).unwrap_or(t));
                }
            }
            if found_temp.is_none() {
                found_temp = read_thermal_zone_temp();
            }
            self.cached_cpu_temp = found_temp;
            self.last_temp_refresh = now;
        }

        // Global CPU usage with smoothing
        let raw_global_cpu = self.sys.global_cpu_usage();
        self.smoothed_global_cpu = if self.smoothed_global_cpu == 0.0 {
            raw_global_cpu
        } else {
            self.smoothed_global_cpu * 0.65 + raw_global_cpu * 0.35
        };
        let cpu_global_usage = self.smoothed_global_cpu;
        let cpu_point = cpu_global_usage.round().clamp(0.0, 100.0) as u64;
        if self.cpu_history.len() >= MAX_HISTORY_LEN {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(cpu_point);

        // Per-core CPU usage with smoothing
        let cpus = self.sys.cpus();
        if self.smoothed_cores_cpu.len() != cpus.len() {
            self.smoothed_cores_cpu = vec![0.0; cpus.len()];
        }
        let mut smoothed_cores = self.smoothed_cores_cpu.clone();
        let cpu_cores: Vec<CoreMetric> = cpus
            .iter()
            .enumerate()
            .map(|(idx, cpu)| {
                let raw_usage = cpu.cpu_usage();
                let smoothed = if smoothed_cores[idx] == 0.0 {
                    raw_usage
                } else {
                    smoothed_cores[idx] * 0.65 + raw_usage * 0.35
                };
                smoothed_cores[idx] = smoothed;
                CoreMetric {
                    id: idx,
                    name: cpu.name().to_string(),
                    usage: smoothed,
                    frequency_mhz: cpu.frequency(),
                    temp_celsius: self.cached_cpu_temp,
                }
            })
            .collect();
        self.smoothed_cores_cpu = smoothed_cores;

        // Memory & Swap
        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let mem_free = self.sys.free_memory();
        let mem_available = self.sys.available_memory();
        let mem_percent = if mem_total > 0 {
            (mem_used as f32 / mem_total as f32) * 100.0
        } else {
            0.0
        };

        let mem_point = mem_percent.round().clamp(0.0, 100.0) as u64;
        if self.mem_history.len() >= MAX_HISTORY_LEN {
            self.mem_history.pop_front();
        }
        self.mem_history.push_back(mem_point);

        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();
        let swap_free = swap_total.saturating_sub(swap_used);
        let swap_percent = if swap_total > 0 {
            (swap_used as f32 / swap_total as f32) * 100.0
        } else {
            0.0
        };

        // Refresh filesystems
        if self.last_disk_refresh.elapsed() >= Duration::from_secs(5) || self.cached_disks.is_empty() {
            self.disks.refresh(true);
            self.last_disk_refresh = now;

            let mut seen_mounts = HashSet::new();
            let mut list = Vec::new();

            for disk in &self.disks {
                let mount = disk.mount_point().to_string_lossy().to_string();
                let fs = disk.file_system().to_string_lossy().to_string();

                if mount.starts_with("/var/lib/docker") || fs == "overlay" {
                    continue;
                }

                if seen_mounts.insert(mount.clone()) {
                    let total = disk.total_space();
                    let avail = disk.available_space();
                    let used = total.saturating_sub(avail);
                    let pct = if total > 0 {
                        (used as f32 / total as f32) * 100.0
                    } else {
                        0.0
                    };

                    list.push(DiskMetric {
                        mount_point: mount,
                        total_bytes: total,
                        available_bytes: avail,
                        used_bytes: used,
                        used_percent: pct,
                        fs_type: fs,
                    });
                }
            }
            self.cached_disks = list;
        }

        // Network traffic rates
        let mut total_rx_rate = 0u64;
        let mut total_tx_rate = 0u64;
        let mut total_rx_all = 0u64;
        let mut total_tx_all = 0u64;
        let mut best_interface = String::new();
        let mut best_traffic = 0u64;
        let mut interfaces = Vec::new();
        let mut per_interface: HashMap<String, NetworkMetric> = HashMap::new();

        for (iface_name, net) in &self.networks {
            if iface_name == "lo" {
                continue;
            }

            let rx_bytes = net.received();
            let tx_bytes = net.transmitted();
            let rx_rate = (rx_bytes as f64 / delta_secs) as u64;
            let tx_rate = (tx_bytes as f64 / delta_secs) as u64;

            let iface_hist = self.iface_histories.entry(iface_name.clone()).or_insert_with(|| {
                (
                    VecDeque::with_capacity(MAX_HISTORY_LEN),
                    VecDeque::with_capacity(MAX_HISTORY_LEN),
                    0,
                    0,
                )
            });

            iface_hist.2 = iface_hist.2.max(rx_rate);
            iface_hist.3 = iface_hist.3.max(tx_rate);

            if iface_hist.0.len() >= MAX_HISTORY_LEN {
                iface_hist.0.pop_front();
            }
            iface_hist.0.push_back(rx_rate);

            if iface_hist.1.len() >= MAX_HISTORY_LEN {
                iface_hist.1.pop_front();
            }
            iface_hist.1.push_back(tx_rate);

            let iface_metric = InterfaceMetric {
                name: iface_name.clone(),
                rx_bytes_sec: rx_rate,
                tx_bytes_sec: tx_rate,
                total_rx_bytes: net.total_received(),
                total_tx_bytes: net.total_transmitted(),
            };
            interfaces.push(iface_metric);

            per_interface.insert(
                iface_name.clone(),
                NetworkMetric {
                    interface_name: iface_name.clone(),
                    rx_bytes_sec: rx_rate,
                    tx_bytes_sec: tx_rate,
                    peak_rx_sec: iface_hist.2,
                    peak_tx_sec: iface_hist.3,
                    total_rx_bytes: net.total_received(),
                    total_tx_bytes: net.total_transmitted(),
                    rx_history: iface_hist.0.iter().copied().collect(),
                    tx_history: iface_hist.1.iter().copied().collect(),
                },
            );

            let is_virtual = iface_name.starts_with("veth")
                || iface_name.starts_with("br-")
                || iface_name == "docker0";

            if !is_virtual {
                total_rx_rate += rx_rate;
                total_tx_rate += tx_rate;
                total_rx_all += net.total_received();
                total_tx_all += net.total_transmitted();

                if rx_bytes + tx_bytes > best_traffic || best_interface.is_empty() {
                    best_traffic = rx_bytes + tx_bytes;
                    best_interface = iface_name.clone();
                }
            }
        }

        interfaces.sort_by(|a, b| {
            let a_virt = a.name.starts_with("veth") || a.name.starts_with("br-") || a.name == "docker0";
            let b_virt = b.name.starts_with("veth") || b.name.starts_with("br-") || b.name == "docker0";
            match (a_virt, b_virt) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => (b.rx_bytes_sec + b.tx_bytes_sec).cmp(&(a.rx_bytes_sec + a.tx_bytes_sec)),
            }
        });

        if best_interface.is_empty() {
            best_interface = String::from("eth0");
        }

        self.peak_rx_sec = self.peak_rx_sec.max(total_rx_rate);
        self.peak_tx_sec = self.peak_tx_sec.max(total_tx_rate);

        // Maintain full raw precision (bytes/sec)
        if self.net_rx_history.len() >= MAX_HISTORY_LEN {
            self.net_rx_history.pop_front();
        }
        self.net_rx_history.push_back(total_rx_rate);

        if self.net_tx_history.len() >= MAX_HISTORY_LEN {
            self.net_tx_history.pop_front();
        }
        self.net_tx_history.push_back(total_tx_rate);

        let network = NetworkMetric {
            interface_name: best_interface,
            rx_bytes_sec: total_rx_rate,
            tx_bytes_sec: total_tx_rate,
            peak_rx_sec: self.peak_rx_sec,
            peak_tx_sec: self.peak_tx_sec,
            total_rx_bytes: total_rx_all,
            total_tx_bytes: total_tx_all,
            rx_history: self.net_rx_history.iter().copied().collect(),
            tx_history: self.net_tx_history.iter().copied().collect(),
        };

        // Process refresh
        let mut running_tasks = 0;
        let mut sleeping_tasks = 0;

        if self.last_proc_refresh.elapsed() >= Duration::from_millis(400) || self.cached_processes.is_empty() {
            let proc_delta_secs = now.duration_since(self.last_proc_refresh).as_secs_f64().max(0.05);
            self.sys.refresh_processes(ProcessesToUpdate::All, true);
            self.last_proc_refresh = now;

            let mut new_cpu_map = HashMap::new();
            let mut top_surge: Option<(String, u32, f32)> = None;
            let mut max_surge = 8.0f32; // Report surges > 8%

            let proc_list: Vec<ProcessMetric> = self
                .sys
                .processes()
                .iter()
                .map(|(pid, proc_info)| {
                    let u_pid = pid.as_u32();
                    let current_cpu = proc_info.cpu_usage();
                    new_cpu_map.insert(u_pid, current_cpu);

                    let prev_cpu = self.prev_cpu_map.get(&u_pid).copied().unwrap_or(current_cpu);
                    let cpu_delta = (current_cpu - prev_cpu).max(0.0);

                    let proc_mem = proc_info.memory();
                    let proc_mem_pct = if mem_total > 0 {
                        (proc_mem as f32 / mem_total as f32) * 100.0
                    } else {
                        0.0
                    };

                    let user = proc_info
                        .user_id()
                        .and_then(|uid| self.users.get_user_by_id(uid))
                        .map(|u| u.name().to_string())
                        .unwrap_or_else(|| "-".to_string());

                    let status_str = format!("{:?}", proc_info.status());
                    if status_str == "Run" {
                        running_tasks += 1;
                    } else {
                        sleeping_tasks += 1;
                    }

                    let name = proc_info.name().to_string_lossy().to_string();
                    let cmd_vec: Vec<String> = proc_info
                        .cmd()
                        .iter()
                        .map(|c| c.to_string_lossy().to_string())
                        .collect();
                    let cmdline = if cmd_vec.is_empty() {
                        name.clone()
                    } else {
                        cmd_vec.join(" ")
                    };

                    let is_kernel_thread = (proc_mem == 0 && proc_info.cmd().is_empty())
                        || name.starts_with("kworker")
                        || name.starts_with("rcu_")
                        || name.starts_with("migration/")
                        || name.starts_with("jbd2/");

                    let disk = proc_info.disk_usage();
                    let disk_read_bytes_sec = (disk.read_bytes as f64 / proc_delta_secs) as u64;
                    let disk_written_bytes_sec = (disk.written_bytes as f64 / proc_delta_secs) as u64;

                    // Domain resolution with 30s cache TTL
                    let domain = match self.cached_domain_map.get(&u_pid) {
                        Some((dom, ts)) if now.duration_since(*ts) < Duration::from_secs(30) => dom.clone(),
                        _ => {
                            let d = self.container_tracker.resolve_domain(u_pid);
                            self.cached_domain_map.insert(u_pid, (d.clone(), now));
                            d
                        }
                    };

                    if cpu_delta > max_surge && !name.is_empty() && !is_kernel_thread {
                        max_surge = cpu_delta;
                        top_surge = Some((name.clone(), u_pid, cpu_delta));
                    }

                    let nice = read_proc_nice(u_pid);

                    ProcessMetric {
                        pid: u_pid,
                        name,
                        cmdline,
                        status: status_str,
                        parent_pid: proc_info.parent().map(|p| p.as_u32()),
                        cpu_usage: current_cpu,
                        cpu_delta,
                        memory_bytes: proc_mem,
                        memory_percent: proc_mem_pct,
                        user,
                        is_kernel_thread,
                        disk_read_bytes_sec,
                        disk_written_bytes_sec,
                        domain,
                        nice,
                    }
                })
                .collect();

            self.prev_cpu_map = new_cpu_map;
            self.cached_processes = proc_list;
            self.latest_surge = top_surge;
        }

        // System telemetry and GPU metrics
        let uptime_seconds = System::uptime();
        let load = System::load_average();
        let host_name = System::host_name().unwrap_or_else(|| "localhost".to_string());
        let os_name = System::long_os_version()
            .or_else(System::name)
            .unwrap_or_else(|| "Linux".to_string());

        let total_tasks = self.cached_processes.len();
        let gpu = self.gpu_collector.sample();

        SystemMetrics {
            cpu_global_usage,
            cpu_cores,
            cpu_temp_celsius: self.cached_cpu_temp,
            cpu_history: self.cpu_history.iter().copied().collect(),
            cpu_model: self.cpu_model.clone(),
            cpu_arch: self.cpu_arch.clone(),
            kernel_version: self.kernel_version.clone(),
            mem_total,
            mem_used,
            mem_free,
            mem_available,
            mem_percent,
            mem_history: self.mem_history.iter().copied().collect(),
            swap_total,
            swap_used,
            swap_free,
            swap_percent,
            disks: self.cached_disks.clone(),
            network,
            interfaces,
            per_interface,
            gpu,
            uptime_seconds,
            load_avg: (load.one, load.five, load.fifteen),
            host_name,
            os_name,
            total_tasks,
            running_tasks,
            sleeping_tasks,
            processes: self.cached_processes.clone(),
            psi: self.cached_psi,
            surge_process: self.latest_surge.clone(),
        }
    }
}
