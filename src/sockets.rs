use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq)]
pub struct SocketInfo {
    pub proto: &'static str,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: &'static str,
}

impl SocketInfo {
    pub fn is_listening(&self) -> bool {
        self.state == "LISTEN"
    }
}

pub struct SocketInspector;

impl SocketInspector {
    /// Reads socket descriptors for a given PID from /proc/<pid>/fd
    pub fn inspect_pid(pid: u32) -> Vec<SocketInfo> {
        let fd_dir = format!("/proc/{}/fd", pid);
        let Ok(entries) = fs::read_dir(fd_dir) else {
            return Vec::new();
        };

        let mut target_inodes = HashSet::new();

        for entry in entries.flatten() {
            if let Ok(link) = fs::read_link(entry.path()) {
                let link_str = link.to_string_lossy();
                if let Some(rest) = link_str.strip_prefix("socket:[") {
                    if let Some(inode_str) = rest.strip_suffix(']') {
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            target_inodes.insert(inode);
                        }
                    }
                }
            }
        }

        if target_inodes.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        Self::parse_proc_net("/proc/net/tcp", "TCP", &target_inodes, &mut results, false);
        Self::parse_proc_net("/proc/net/tcp6", "TCP6", &target_inodes, &mut results, true);
        Self::parse_proc_net("/proc/net/udp", "UDP", &target_inodes, &mut results, false);
        Self::parse_proc_net("/proc/net/udp6", "UDP6", &target_inodes, &mut results, true);

        results
    }

    fn parse_proc_net(
        path: &str,
        proto: &'static str,
        target_inodes: &HashSet<u64>,
        results: &mut Vec<SocketInfo>,
        is_v6: bool,
    ) {
        let Ok(file) = fs::File::open(path) else {
            return;
        };
        let reader = BufReader::new(file);

        for line in reader.lines().skip(1).flatten() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 10 {
                continue;
            }

            let inode_idx = if cols.len() >= 10 { 9 } else { cols.len() - 1 };
            let Ok(inode) = cols[inode_idx].parse::<u64>() else {
                continue;
            };

            if target_inodes.contains(&inode) {
                let (local_ip, local_port) = Self::parse_addr(cols[1], is_v6);
                let (remote_ip, remote_port) = Self::parse_addr(cols[2], is_v6);
                let state_str = match cols[3] {
                    "01" => "ESTABLISHED",
                    "02" => "SYN_SENT",
                    "03" => "SYN_RECV",
                    "04" => "FIN_WAIT1",
                    "05" => "FIN_WAIT2",
                    "06" => "TIME_WAIT",
                    "07" => "CLOSE",
                    "08" => "CLOSE_WAIT",
                    "09" => "LAST_ACK",
                    "0A" => "LISTEN",
                    "0B" => "CLOSING",
                    _ => "UDP/ACTIVE",
                };

                results.push(SocketInfo {
                    proto,
                    local_addr: local_ip,
                    local_port,
                    remote_addr: remote_ip,
                    remote_port,
                    state: state_str,
                });
            }
        }
    }

    fn parse_addr(hex_str: &str, is_v6: bool) -> (String, u16) {
        let mut parts = hex_str.split(':');
        let ip_hex = parts.next().unwrap_or("");
        let port_hex = parts.next().unwrap_or("0");
        let port = u16::from_str_radix(port_hex, 16).unwrap_or(0);

        if !is_v6 && ip_hex.len() == 8 {
            if let Ok(raw_u32) = u32::from_str_radix(ip_hex, 16) {
                // Little endian representation in /proc/net/tcp
                let ip = Ipv4Addr::from(raw_u32.to_le());
                return (ip.to_string(), port);
            }
        } else if is_v6 && ip_hex.len() == 32 {
            // Parse 128-bit hex into Ipv6Addr
            let mut bytes = [0u8; 16];
            for i in 0..4 {
                if let Ok(word) = u32::from_str_radix(&ip_hex[i * 8..(i + 1) * 8], 16) {
                    let word_bytes = word.to_le_bytes();
                    bytes[i * 4..(i + 1) * 4].copy_from_slice(&word_bytes);
                }
            }
            let ip = Ipv6Addr::from(bytes);
            return (ip.to_string(), port);
        }

        (ip_hex.to_string(), port)
    }
}
