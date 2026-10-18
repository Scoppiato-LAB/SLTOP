use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

/// Real-time GPU telemetry snapshot
#[derive(Debug, Clone, PartialEq)]
pub struct GpuMetric {
    pub name: String,
    pub brand: String,
    pub usage_percent: Option<f32>,
    pub mem_used_bytes: Option<u64>,
    pub mem_total_bytes: Option<u64>,
    pub temp_celsius: Option<f32>,
    pub power_watts: Option<f32>,
}

pub struct GpuCollector {
    cached: Option<GpuMetric>,
    last_probe: Instant,
    has_nvidia_smi: Option<bool>,
}

impl GpuCollector {
    pub fn new() -> Self {
        Self {
            cached: None,
            last_probe: Instant::now() - Duration::from_secs(10),
            has_nvidia_smi: None,
        }
    }

    /// Probes GPU metrics (rate-limited to 1.5s)
    pub fn sample(&mut self) -> Option<GpuMetric> {
        let now = Instant::now();
        if now.duration_since(self.last_probe) < Duration::from_millis(1500) {
            return self.cached.clone();
        }
        self.last_probe = now;

        // Check NVIDIA via nvidia-smi
        if self.has_nvidia_smi.unwrap_or(true) {
            if let Some(gpu) = self.probe_nvidia() {
                self.has_nvidia_smi = Some(true);
                self.cached = Some(gpu.clone());
                return Some(gpu);
            } else if self.has_nvidia_smi.is_none() {
                self.has_nvidia_smi = Some(false);
            }
        }

        // Check Linux DRM sysfs
        if let Some(gpu) = self.probe_drm_sysfs() {
            self.cached = Some(gpu.clone());
            return Some(gpu);
        }

        self.cached = None;
        None
    }

    fn probe_nvidia(&self) -> Option<GpuMetric> {
        // Run nvidia-smi with compact csv output
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let first_line = text.lines().next()?;
        let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();

        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].to_string();
        let usage = parts.get(1).and_then(|s| s.parse::<f32>().ok());
        let mem_used_mb = parts.get(2).and_then(|s| s.parse::<u64>().ok());
        let mem_total_mb = parts.get(3).and_then(|s| s.parse::<u64>().ok());
        let temp = parts.get(4).and_then(|s| s.parse::<f32>().ok());
        let power = parts.get(5).and_then(|s| s.parse::<f32>().ok());

        Some(GpuMetric {
            name,
            brand: "NVIDIA".to_string(),
            usage_percent: usage,
            mem_used_bytes: mem_used_mb.map(|mb| mb * 1024 * 1024),
            mem_total_bytes: mem_total_mb.map(|mb| mb * 1024 * 1024),
            temp_celsius: temp,
            power_watts: power,
        })
    }

    fn probe_drm_sysfs(&self) -> Option<GpuMetric> {
        let drm_dir = fs::read_dir("/sys/class/drm").ok()?;

        for entry in drm_dir.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.starts_with("card") || file_name.contains('-') {
                continue;
            }

            let card_path = entry.path();
            let device_path = card_path.join("device");
            if !device_path.exists() {
                continue;
            }

            // Probe driver identity from uevent
            let mut driver_name = String::new();
            if let Ok(uevent) = fs::read_to_string(device_path.join("uevent")) {
                for line in uevent.lines() {
                    if let Some(drv) = line.strip_prefix("DRIVER=") {
                        driver_name = drv.trim().to_string();
                        break;
                    }
                }
            }

            // Read AMD/DRM usage
            let usage_percent = fs::read_to_string(device_path.join("gpu_busy_percent"))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok());

            // Read VRAM
            let mem_used = fs::read_to_string(device_path.join("mem_info_vram_used"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());
            let mem_total = fs::read_to_string(device_path.join("mem_info_vram_total"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());

            // Read Temperature from hwmon
            let mut temp_celsius = None;
            if let Ok(hwmon_entries) = fs::read_dir(device_path.join("hwmon")) {
                for h in hwmon_entries.flatten() {
                    let temp_file = h.path().join("temp1_input");
                    if let Ok(t_str) = fs::read_to_string(temp_file) {
                        if let Ok(milli) = t_str.trim().parse::<f32>() {
                            temp_celsius = Some(milli / 1000.0);
                            break;
                        }
                    }
                }
            }

            let (brand, name) = match driver_name.as_str() {
                "amdgpu" | "radeon" => ("AMD".to_string(), "AMD Radeon Graphics".to_string()),
                "i915" | "xe" => (
                    "Intel".to_string(),
                    "Intel HD/Iris/Arc Graphics".to_string(),
                ),
                "nouveau" => ("NVIDIA".to_string(), "NVIDIA (Nouveau DRM)".to_string()),
                "virtio-gpu" => ("VirtIO".to_string(), "VirtIO GPU Accelerator".to_string()),
                "bochs-drm" => (
                    "Bochs".to_string(),
                    "Virtual Display (bochs-drm)".to_string(),
                ),
                other if !other.is_empty() => (other.to_string(), format!("DRM Device ({other})")),
                _ => (
                    "DRM".to_string(),
                    format!("DRM Display Adapter ({file_name})"),
                ),
            };

            return Some(GpuMetric {
                name,
                brand,
                usage_percent,
                mem_used_bytes: mem_used,
                mem_total_bytes: mem_total,
                temp_celsius,
                power_watts: None,
            });
        }

        None
    }
}

impl Default for GpuCollector {
    fn default() -> Self {
        Self::new()
    }
}
