use crate::metrics::SystemMetrics;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Serializes system metrics into JSON format
pub fn dump_json(metrics: &SystemMetrics) -> String {
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut out = String::with_capacity(32 * 1024);
    out.push_str("{\n");
    out.push_str(&format!("  \"timestamp\": {},\n", now_ts));
    out.push_str("  \"system\": {\n");
    out.push_str(&format!(
        "    \"host_name\": \"{}\",\n",
        escape_json(&metrics.host_name)
    ));
    out.push_str(&format!(
        "    \"os_name\": \"{}\",\n",
        escape_json(&metrics.os_name)
    ));
    out.push_str(&format!(
        "    \"kernel\": \"{}\",\n",
        escape_json(&metrics.kernel_version)
    ));
    out.push_str(&format!(
        "    \"uptime_seconds\": {},\n",
        metrics.uptime_seconds
    ));
    out.push_str(&format!(
        "    \"load_avg\": [{:.2}, {:.2}, {:.2}],\n",
        metrics.load_avg.0, metrics.load_avg.1, metrics.load_avg.2
    ));
    out.push_str(&format!(
        "    \"tasks\": {{\"total\": {}, \"running\": {}, \"sleeping\": {}}},\n",
        metrics.total_tasks, metrics.running_tasks, metrics.sleeping_tasks
    ));

    // CPU telemetry
    out.push_str("    \"cpu\": {\n");
    out.push_str(&format!(
        "      \"model\": \"{}\",\n",
        escape_json(&metrics.cpu_model)
    ));
    out.push_str(&format!(
        "      \"arch\": \"{}\",\n",
        escape_json(&metrics.cpu_arch)
    ));
    out.push_str(&format!(
        "      \"global_usage_percent\": {:.1},\n",
        metrics.cpu_global_usage
    ));
    if let Some(t) = metrics.cpu_temp_celsius {
        out.push_str(&format!("      \"temperature_celsius\": {:.1},\n", t));
    } else {
        out.push_str("      \"temperature_celsius\": null,\n");
    }
    out.push_str("      \"cores\": [\n");
    for (i, c) in metrics.cpu_cores.iter().enumerate() {
        let comma = if i + 1 == metrics.cpu_cores.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "        {{\"id\": {}, \"name\": \"{}\", \"usage_percent\": {:.1}, \"freq_mhz\": {}}}{}\n",
            c.id, escape_json(&c.name), c.usage, c.frequency_mhz, comma
        ));
    }
    out.push_str("      ]\n");
    out.push_str("    },\n");

    // Memory
    out.push_str("    \"memory\": {\n");
    out.push_str(&format!("      \"total_bytes\": {},\n", metrics.mem_total));
    out.push_str(&format!("      \"used_bytes\": {},\n", metrics.mem_used));
    out.push_str(&format!("      \"free_bytes\": {},\n", metrics.mem_free));
    out.push_str(&format!(
        "      \"available_bytes\": {},\n",
        metrics.mem_available
    ));
    out.push_str(&format!(
        "      \"usage_percent\": {:.1}\n",
        metrics.mem_percent
    ));
    out.push_str("    },\n");

    // Swap
    out.push_str("    \"swap\": {\n");
    out.push_str(&format!("      \"total_bytes\": {},\n", metrics.swap_total));
    out.push_str(&format!("      \"used_bytes\": {},\n", metrics.swap_used));
    out.push_str(&format!("      \"free_bytes\": {},\n", metrics.swap_free));
    out.push_str(&format!(
        "      \"usage_percent\": {:.1}\n",
        metrics.swap_percent
    ));
    out.push_str("    },\n");

    // GPU
    if let Some(ref gpu) = metrics.gpu {
        out.push_str("    \"gpu\": {\n");
        out.push_str(&format!(
            "      \"name\": \"{}\",\n",
            escape_json(&gpu.name)
        ));
        out.push_str(&format!(
            "      \"brand\": \"{}\",\n",
            escape_json(&gpu.brand)
        ));
        if let Some(u) = gpu.usage_percent {
            out.push_str(&format!("      \"usage_percent\": {:.1},\n", u));
        } else {
            out.push_str("      \"usage_percent\": null,\n");
        }
        if let Some(used) = gpu.mem_used_bytes {
            out.push_str(&format!("      \"vram_used_bytes\": {},\n", used));
        }
        if let Some(total) = gpu.mem_total_bytes {
            out.push_str(&format!("      \"vram_total_bytes\": {},\n", total));
        }
        if let Some(temp) = gpu.temp_celsius {
            out.push_str(&format!("      \"temp_celsius\": {:.1},\n", temp));
        }
        if let Some(p) = gpu.power_watts {
            out.push_str(&format!("      \"power_watts\": {:.1}\n", p));
        } else {
            out.push_str("      \"power_watts\": null\n");
        }
        out.push_str("    },\n");
    } else {
        out.push_str("    \"gpu\": null,\n");
    }

    // Network
    out.push_str("    \"network\": {\n");
    out.push_str(&format!(
        "      \"selected_interface\": \"{}\",\n",
        escape_json(&metrics.network.interface_name)
    ));
    out.push_str(&format!(
        "      \"rx_bytes_sec\": {},\n",
        metrics.network.rx_bytes_sec
    ));
    out.push_str(&format!(
        "      \"tx_bytes_sec\": {},\n",
        metrics.network.tx_bytes_sec
    ));
    out.push_str(&format!(
        "      \"peak_rx_sec\": {},\n",
        metrics.network.peak_rx_sec
    ));
    out.push_str(&format!(
        "      \"peak_tx_sec\": {},\n",
        metrics.network.peak_tx_sec
    ));
    out.push_str(&format!(
        "      \"total_rx_bytes\": {},\n",
        metrics.network.total_rx_bytes
    ));
    out.push_str(&format!(
        "      \"total_tx_bytes\": {},\n",
        metrics.network.total_tx_bytes
    ));
    out.push_str("      \"interfaces\": [\n");
    for (i, iface) in metrics.interfaces.iter().enumerate() {
        let comma = if i + 1 == metrics.interfaces.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "        {{\"name\": \"{}\", \"rx_sec\": {}, \"tx_sec\": {}, \"rx_total\": {}, \"tx_total\": {}}}{}\n",
            escape_json(&iface.name), iface.rx_bytes_sec, iface.tx_bytes_sec, iface.total_rx_bytes, iface.total_tx_bytes, comma
        ));
    }
    out.push_str("      ]\n");
    out.push_str("    },\n");

    // Disks
    out.push_str("    \"disks\": [\n");
    for (i, d) in metrics.disks.iter().enumerate() {
        let comma = if i + 1 == metrics.disks.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "      {{\"mount\": \"{}\", \"fs\": \"{}\", \"total_bytes\": {}, \"used_bytes\": {}, \"percent\": {:.1}}}{}\n",
            escape_json(&d.mount_point), escape_json(&d.fs_type), d.total_bytes, d.used_bytes, d.used_percent, comma
        ));
    }
    out.push_str("    ]\n");
    out.push_str("  },\n");

    // Processes
    out.push_str("  \"processes\": [\n");
    for (i, p) in metrics.processes.iter().enumerate() {
        let comma = if i + 1 == metrics.processes.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"pid\": {}, \"name\": \"{}\", \"user\": \"{}\", \"status\": \"{}\", \"cpu_percent\": {:.1}, \"mem_bytes\": {}, \"mem_percent\": {:.1}, \"disk_read_sec\": {}, \"disk_write_sec\": {}, \"domain\": \"{}\", \"nice\": {}}}{}\n",
            p.pid, escape_json(&p.name), escape_json(&p.user), escape_json(&p.status), p.cpu_usage, p.memory_bytes, p.memory_percent, p.disk_read_bytes_sec, p.disk_written_bytes_sec, escape_json(&p.domain.label()), p.nice, comma
        ));
    }
    out.push_str("  ]\n");
    out.push_str("}\n");

    out
}

/// Generates a text dashboard snapshot
pub fn dump_snapshot_text(metrics: &SystemMetrics) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("=== sltop System Snapshot | Scoppiato LAB (https://scoppiato.it) ===\n");
    out.push_str(&format!(
        "Host: {} | OS: {} | Kernel: {}\n",
        metrics.host_name, metrics.os_name, metrics.kernel_version
    ));
    out.push_str(&format!(
        "Uptime: {}s | Load: {:.2}, {:.2}, {:.2}\n",
        metrics.uptime_seconds, metrics.load_avg.0, metrics.load_avg.1, metrics.load_avg.2
    ));
    out.push_str(&format!(
        "CPU: {} ({} Cores) - {:.1}%\n",
        metrics.cpu_model,
        metrics.cpu_cores.len(),
        metrics.cpu_global_usage
    ));
    out.push_str(&format!(
        "RAM: {:.1}% ({:.2} GiB / {:.2} GiB)\n",
        metrics.mem_percent,
        metrics.mem_used as f64 / 1073741824.0,
        metrics.mem_total as f64 / 1073741824.0
    ));
    out.push_str(&format!(
        "Network ({}): ↓ {}/s | ↑ {}/s\n",
        metrics.network.interface_name, metrics.network.rx_bytes_sec, metrics.network.tx_bytes_sec
    ));
    if let Some(ref g) = metrics.gpu {
        out.push_str(&format!("GPU: {} ({})\n", g.name, g.brand));
    }
    out.push_str("\n--- Top Processes by CPU ---\n");
    out.push_str("PID     USER       CPU%   MEM%    COMMAND\n");

    let mut procs = metrics.processes.clone();
    procs.sort_by(|a, b| {
        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for p in procs.iter().take(15) {
        out.push_str(&format!(
            "{:<7} {:<10} {:>5.1}% {:>5.1}%   {}\n",
            p.pid, p.user, p.cpu_usage, p.memory_percent, p.name
        ));
    }

    out
}

/// Saves JSON snapshot to disk and returns file path
pub fn save_snapshot_file(metrics: &SystemMetrics) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename = format!("sltop_snapshot_{}.json", now);
    let json = dump_json(metrics);
    fs::write(&filename, json).map_err(|e| format!("Failed to write snapshot: {}", e))?;
    Ok(filename)
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
