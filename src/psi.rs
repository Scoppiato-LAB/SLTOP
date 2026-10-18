use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct PsiMetrics {
    pub cpu_some: f32,
    pub mem_some: f32,
    pub mem_full: f32,
    pub io_some: f32,
    pub io_full: f32,
}

impl PsiMetrics {
    /// Reads Pressure Stall Information from Linux /proc/pressure
    pub fn read() -> Option<Self> {
        let cpu_some = Self::parse_pressure_file("/proc/pressure/cpu", "some")?;
        let mem_some = Self::parse_pressure_file("/proc/pressure/memory", "some").unwrap_or(0.0);
        let mem_full = Self::parse_pressure_file("/proc/pressure/memory", "full").unwrap_or(0.0);
        let io_some = Self::parse_pressure_file("/proc/pressure/io", "some").unwrap_or(0.0);
        let io_full = Self::parse_pressure_file("/proc/pressure/io", "full").unwrap_or(0.0);

        Some(Self {
            cpu_some,
            mem_some,
            mem_full,
            io_some,
            io_full,
        })
    }

    fn parse_pressure_file(path: &str, line_prefix: &str) -> Option<f32> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);

        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with(line_prefix) {
                // Example: some avg10=11.14 avg60=12.63 avg300=14.55 total=20121786832
                for part in line.split_whitespace() {
                    if let Some(val_str) = part.strip_prefix("avg10=") {
                        if let Ok(val) = val_str.parse::<f32>() {
                            return Some(val);
                        }
                    }
                }
            }
        }
        None
    }
}
