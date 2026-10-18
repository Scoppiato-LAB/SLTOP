use std::fs;
use std::io;
use std::path::PathBuf;

/// Application configuration persisted to ~/.config/sltop/sltop.toml
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sample_interval_ms: u64,
    pub theme: String,
    pub sort_by: String,
    pub sort_order: String,
    pub view_mode: String,
    pub process_view_type: String,
    pub hide_kernel_threads: bool,
    pub network_interface: Option<String>,
    pub user_filter: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sample_interval_ms: 100,
            theme: "tokyo_slate".to_string(),
            sort_by: "cpu".to_string(),
            sort_order: "desc".to_string(),
            view_mode: "balanced".to_string(),
            process_view_type: "grouped".to_string(),
            hide_kernel_threads: true,
            network_interface: None,
            user_filter: None,
        }
    }
}

impl AppConfig {
    /// Returns default path: ~/.config/sltop/sltop.toml or $XDG_CONFIG_HOME/sltop/sltop.toml
    pub fn config_path() -> Option<PathBuf> {
        let base_dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.trim().is_empty() {
                PathBuf::from(xdg)
            } else {
                get_home_config_dir()?
            }
        } else {
            get_home_config_dir()?
        };

        Some(base_dir.join("sltop").join("sltop.toml"))
    }

    /// Loads configuration from ~/.config/sltop/sltop.toml, falling back to defaults
    pub fn load() -> Self {
        let mut config = Self::default();

        let Some(path) = Self::config_path() else {
            return config;
        };

        let Ok(content) = fs::read_to_string(&path) else {
            return config;
        };

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }

            if let Some((key, val_raw)) = line.split_once('=') {
                let key = key.trim();
                let val = val_raw.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "sample_interval_ms" | "interval" => {
                        if let Ok(num) = val.parse::<u64>() {
                            config.sample_interval_ms = num.clamp(100, 5000);
                        }
                    }
                    "theme" => {
                        config.theme = val.to_string();
                    }
                    "sort_by" => {
                        config.sort_by = val.to_string();
                    }
                    "sort_order" => {
                        config.sort_order = val.to_string();
                    }
                    "view_mode" => {
                        config.view_mode = val.to_string();
                    }
                    "process_view_type" | "view_type" => {
                        config.process_view_type = val.to_string();
                    }
                    "hide_kernel_threads" => {
                        config.hide_kernel_threads = val == "true" || val == "1";
                    }
                    "network_interface" | "interface" => {
                        if val.is_empty() || val.eq_ignore_ascii_case("all") {
                            config.network_interface = None;
                        } else {
                            config.network_interface = Some(val.to_string());
                        }
                    }
                    "user_filter" => {
                        if val.is_empty() || val.eq_ignore_ascii_case("all") {
                            config.user_filter = None;
                        } else {
                            config.user_filter = Some(val.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        config
    }

    /// Writes configuration to ~/.config/sltop/sltop.toml
    pub fn save(&self) -> io::Result<PathBuf> {
        let Some(path) = Self::config_path() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Unable to determine home directory for configuration path",
            ));
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut toml = String::new();
        toml.push_str("# sltop Configuration File\n");
        toml.push_str("# Generated automatically by sltop\n\n");

        toml.push_str("[display]\n");
        toml.push_str(&format!("theme = \"{}\"\n", self.theme));
        toml.push_str(&format!("view_mode = \"{}\"\n", self.view_mode));
        toml.push_str(&format!(
            "sample_interval_ms = {}\n\n",
            self.sample_interval_ms
        ));

        toml.push_str("[process]\n");
        toml.push_str(&format!("sort_by = \"{}\"\n", self.sort_by));
        toml.push_str(&format!("sort_order = \"{}\"\n", self.sort_order));
        toml.push_str(&format!(
            "process_view_type = \"{}\"\n",
            self.process_view_type
        ));
        toml.push_str(&format!(
            "hide_kernel_threads = {}\n",
            self.hide_kernel_threads
        ));
        if let Some(ref u) = self.user_filter {
            toml.push_str(&format!("user_filter = \"{}\"\n\n", u));
        } else {
            toml.push_str("user_filter = \"\"\n\n");
        }

        toml.push_str("[network]\n");
        if let Some(ref iface) = self.network_interface {
            toml.push_str(&format!("interface = \"{}\"\n", iface));
        } else {
            toml.push_str("interface = \"all\"\n");
        }

        fs::write(&path, toml)?;
        Ok(path)
    }
}

fn get_home_config_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Some(PathBuf::from(home).join(".config"));
        }
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        if !user_profile.trim().is_empty() {
            return Some(PathBuf::from(user_profile).join(".config"));
        }
    }
    None
}
