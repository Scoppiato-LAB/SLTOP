use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessGroupDomain {
    Docker(String),      // Container name (e.g., "kasm_guac")
    Service(String),     // Systemd service (e.g., "mariadb.service")
    UserSession(String), // User session (e.g., "user-1000")
    System,
}

impl ProcessGroupDomain {
    pub fn label(&self) -> String {
        match self {
            Self::Docker(name) => format!("docker:{}", name),
            Self::Service(svc) => svc.clone(),
            Self::UserSession(u) => format!("user:{}", u),
            Self::System => "system".to_string(),
        }
    }
}

pub struct ContainerTracker {
    docker_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl ContainerTracker {
    pub fn new() -> Self {
        let docker_cache = Arc::new(Mutex::new(HashMap::new()));
        let cache_clone = Arc::clone(&docker_cache);

        // Background thread refreshing docker container ID -> Name every 15 seconds
        thread::Builder::new()
            .name("sltop-container-tracker".into())
            .spawn(move || loop {
                let mut new_map = HashMap::new();
                if let Ok(output) = Command::new("docker")
                    .args(["ps", "--format", "{{.ID}} {{.Names}}"])
                    .output()
                {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            let mut parts = line.split_whitespace();
                            if let (Some(id), Some(name)) = (parts.next(), parts.next()) {
                                new_map.insert(id.to_string(), name.to_string());
                            }
                        }
                    }
                }

                if let Ok(mut lock) = cache_clone.lock() {
                    *lock = new_map;
                }

                thread::sleep(Duration::from_secs(15));
            })
            .ok();

        Self { docker_cache }
    }

    /// Resolves the cgroup domain for a given PID
    pub fn resolve_domain(&self, pid: u32) -> ProcessGroupDomain {
        let cgroup_path = format!("/proc/{}/cgroup", pid);
        let Ok(content) = fs::read_to_string(cgroup_path) else {
            return ProcessGroupDomain::System;
        };

        for line in content.lines() {
            // Check for Docker
            if let Some(idx) = line.find("docker-") {
                let rest = &line[idx + 7..];
                let hex_id = rest
                    .split(|c: char| !c.is_ascii_hexdigit())
                    .next()
                    .unwrap_or("");
                if hex_id.len() >= 12 {
                    let short_id = &hex_id[..12];
                    if let Ok(lock) = self.docker_cache.lock() {
                        if let Some(name) = lock.get(short_id) {
                            return ProcessGroupDomain::Docker(name.clone());
                        }
                    }
                    return ProcessGroupDomain::Docker(short_id.to_string());
                }
            } else if let Some(idx) = line.find("/docker/") {
                let rest = &line[idx + 8..];
                let hex_id = rest
                    .split(|c: char| !c.is_ascii_hexdigit())
                    .next()
                    .unwrap_or("");
                if hex_id.len() >= 12 {
                    let short_id = &hex_id[..12];
                    if let Ok(lock) = self.docker_cache.lock() {
                        if let Some(name) = lock.get(short_id) {
                            return ProcessGroupDomain::Docker(name.clone());
                        }
                    }
                    return ProcessGroupDomain::Docker(short_id.to_string());
                }
            }

            // Check for Systemd Service
            if let Some(idx) = line.find(".service") {
                let before = &line[..idx + 8];
                let service = before.rsplit('/').next().unwrap_or("");
                if !service.is_empty() {
                    return ProcessGroupDomain::Service(service.to_string());
                }
            }

            // Check for User session
            if let Some(idx) = line.find("user-") {
                let rest = &line[idx..];
                let session = rest.split('/').next().unwrap_or("");
                if !session.is_empty() {
                    return ProcessGroupDomain::UserSession(session.to_string());
                }
            }
        }

        ProcessGroupDomain::System
    }
}

impl Default for ContainerTracker {
    fn default() -> Self {
        Self::new()
    }
}
