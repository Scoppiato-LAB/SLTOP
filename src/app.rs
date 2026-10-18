use crate::config::AppConfig;
use crate::containers::ProcessGroupDomain;
use crate::metrics::{ProcessMetric, SystemMetrics};
use crate::sockets::{SocketInfo, SocketInspector};
use crate::theme::ThemeMode;
use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSortBy {
    Cpu,
    Memory,
    Pid,
    Name,
    User,
    Disk,
    Surge,
}

impl ProcessSortBy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU%",
            Self::Memory => "MEM%",
            Self::Pid => "PID",
            Self::Name => "NAME",
            Self::User => "USER",
            Self::Disk => "DISK",
            Self::Surge => "SURGE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Ascending => "▲",
            Self::Descending => "▼",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessViewType {
    Grouped,
    Flat,
    Tree,
    Container,
}

impl ProcessViewType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Grouped => "Grouped",
            Self::Flat => "Flat",
            Self::Tree => "Tree",
            Self::Container => "Container/Pod",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Balanced,    // 2 Top panels + Process Table
    ProcFocus,   // Process Table dominant
    GraphsFocus, // Top metrics & Graphs dominant
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveModal {
    None,
    Help,
    ProcessInspect(ProcessMetric),
    ProcessKill {
        pid: u32,
        name: String,
    },
    ProcessRenice {
        pid: u32,
        name: String,
        current_nice: i32,
        target_nice: i32,
    },
    UserFilter {
        selected_idx: usize,
        users: Vec<(String, usize)>,
    },
    ProcessSockets {
        pid: u32,
        name: String,
        sockets: Vec<SocketInfo>,
    },
    ProcessEnviron {
        pid: u32,
        name: String,
        vars: Vec<(String, String)>,
    },
    ProcessLogs {
        pid: u32,
        name: String,
        logs: Vec<String>,
    },
}

/// Unified row structure rendered in the process table
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DisplayProcessRow {
    pub pid_display: String,
    pub name: String,
    pub prefix: String,
    pub user: String,
    pub status: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub disk_read_sec: u64,
    pub disk_write_sec: u64,
    pub cmdline: String,
    pub domain_label: String,
    pub is_group_header: bool,
    pub has_children: bool,
    pub is_expanded: bool,
    pub group_key: Option<String>,
    pub actual_pid: u32,
    pub original_metric: Option<ProcessMetric>,
}

pub struct App {
    pub metrics: Option<SystemMetrics>,
    pub table_state: TableState,
    pub sort_by: ProcessSortBy,
    pub sort_order: SortOrder,
    pub selected_index: usize,
    pub is_running: bool,
    pub view_mode: ViewMode,
    pub active_modal: ActiveModal,
    pub hide_kernel_threads: bool,
    pub search_query: String,
    pub is_searching: bool,
    pub sample_interval_ms: Arc<AtomicU64>,
    pub status_toast: Option<(String, std::time::Instant)>,
    pub process_view_type: ProcessViewType,
    pub expanded_groups: HashSet<String>,
    pub collapsed_tree_pids: HashSet<u32>,
    pub theme_mode: ThemeMode,
    pub cached_rows: Vec<DisplayProcessRow>,
    pub modal_scroll: usize,
    pub table_area_y: u16,
    pub table_area_height: u16,
    pub table_header_y: u16,
    pub selected_interface: Option<String>,
    pub user_filter: Option<String>,
    pub config: AppConfig,
}

impl App {
    pub fn new(sample_interval_ms: Arc<AtomicU64>, config: AppConfig) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let theme_mode = match config.theme.to_lowercase().as_str() {
            "catppuccin" | "catppuccin_mocha" => ThemeMode::CatppuccinMocha,
            "gruvbox" | "gruvbox_dark" => ThemeMode::GruvboxDark,
            "nord" | "nordic" | "nordic_frost" => ThemeMode::NordicFrost,
            "monokai" | "monokai_minimal" => ThemeMode::MonokaiMinimal,
            _ => ThemeMode::TokyoSlate,
        };

        let sort_by = match config.sort_by.to_lowercase().as_str() {
            "mem" | "memory" => ProcessSortBy::Memory,
            "pid" => ProcessSortBy::Pid,
            "name" => ProcessSortBy::Name,
            "user" => ProcessSortBy::User,
            "disk" => ProcessSortBy::Disk,
            "surge" => ProcessSortBy::Surge,
            _ => ProcessSortBy::Cpu,
        };

        let sort_order = if config.sort_order.eq_ignore_ascii_case("asc") {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };

        let view_mode = match config.view_mode.to_lowercase().as_str() {
            "proc_focus" | "proc" => ViewMode::ProcFocus,
            "graphs_focus" | "graphs" => ViewMode::GraphsFocus,
            _ => ViewMode::Balanced,
        };

        let process_view_type = match config.process_view_type.to_lowercase().as_str() {
            "flat" => ProcessViewType::Flat,
            "tree" => ProcessViewType::Tree,
            "container" => ProcessViewType::Container,
            _ => ProcessViewType::Grouped,
        };

        let selected_interface = config.network_interface.clone();
        let user_filter = config.user_filter.clone();
        let hide_kernel_threads = config.hide_kernel_threads;

        Self {
            metrics: None,
            table_state,
            sort_by,
            sort_order,
            selected_index: 0,
            is_running: true,
            view_mode,
            active_modal: ActiveModal::None,
            hide_kernel_threads,
            search_query: String::new(),
            is_searching: false,
            sample_interval_ms,
            status_toast: None,
            process_view_type,
            expanded_groups: HashSet::new(),
            collapsed_tree_pids: HashSet::new(),
            theme_mode,
            cached_rows: Vec::new(),
            modal_scroll: 0,
            table_area_y: 20,
            table_area_height: 18,
            table_header_y: 21,
            selected_interface,
            user_filter,
            config,
        }
    }

    pub fn update_metrics(&mut self, metrics: SystemMetrics) {
        self.metrics = Some(metrics);
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn cycle_theme(&mut self) {
        self.theme_mode = self.theme_mode.next();
        self.show_toast(format!("Theme: {}", self.theme_mode.name()));
    }

    pub fn toggle_grouping(&mut self) {
        self.process_view_type = match self.process_view_type {
            ProcessViewType::Grouped => ProcessViewType::Flat,
            _ => ProcessViewType::Grouped,
        };
        self.show_toast(format!("Process View: {}", self.process_view_type.label()));
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn toggle_tree_view(&mut self) {
        self.process_view_type = match self.process_view_type {
            ProcessViewType::Tree => ProcessViewType::Grouped,
            _ => ProcessViewType::Tree,
        };
        self.show_toast(format!("Process View: {}", self.process_view_type.label()));
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn toggle_container_view(&mut self) {
        self.process_view_type = match self.process_view_type {
            ProcessViewType::Container => ProcessViewType::Grouped,
            _ => ProcessViewType::Container,
        };
        self.show_toast(format!("Process View: {}", self.process_view_type.label()));
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn toggle_expand_selected(&mut self) {
        if let Some(row) = self.cached_rows.get(self.selected_index) {
            match self.process_view_type {
                ProcessViewType::Grouped | ProcessViewType::Container => {
                    if let Some(ref key) = row.group_key {
                        if self.expanded_groups.contains(key) {
                            self.expanded_groups.remove(key);
                        } else {
                            self.expanded_groups.insert(key.clone());
                        }
                        self.recompute_rows();
                        self.clamp_selection();
                    }
                }
                ProcessViewType::Tree => {
                    if row.has_children {
                        let pid = row.actual_pid;
                        if self.collapsed_tree_pids.contains(&pid) {
                            self.collapsed_tree_pids.remove(&pid);
                        } else {
                            self.collapsed_tree_pids.insert(pid);
                        }
                        self.recompute_rows();
                        self.clamp_selection();
                    }
                }
                ProcessViewType::Flat => {}
            }
        }
    }

    pub fn recompute_rows(&mut self) {
        let query = self.search_query.trim().to_lowercase();

        // 1. Initial filter into an independent Vec to avoid borrow collisions
        let filtered: Vec<ProcessMetric> = match self.metrics {
            Some(ref metrics) => metrics
                .processes
                .iter()
                .filter(|p| {
                    if self.hide_kernel_threads && p.is_kernel_thread {
                        return false;
                    }
                    if let Some(ref u) = self.user_filter {
                        if !p.user.eq_ignore_ascii_case(u) {
                            return false;
                        }
                    }
                    if !query.is_empty() {
                        let matches_name = p.name.to_lowercase().contains(&query);
                        let matches_cmd = p.cmdline.to_lowercase().contains(&query);
                        let matches_pid = p.pid.to_string().contains(&query);
                        let matches_user = p.user.to_lowercase().contains(&query);
                        let matches_domain = p.domain.label().to_lowercase().contains(&query);
                        matches_name || matches_cmd || matches_pid || matches_user || matches_domain
                    } else {
                        true
                    }
                })
                .cloned()
                .collect(),
            None => {
                self.cached_rows.clear();
                return;
            }
        };

        match self.process_view_type {
            ProcessViewType::Flat => {
                let mut list: Vec<ProcessMetric> = filtered;
                self.sort_processes(&mut list);

                self.cached_rows = list
                    .into_iter()
                    .map(|p| DisplayProcessRow {
                        pid_display: format!("{}", p.pid),
                        name: p.name.clone(),
                        prefix: String::new(),
                        user: p.user.clone(),
                        status: p.status.clone(),
                        cpu_usage: p.cpu_usage,
                        memory_bytes: p.memory_bytes,
                        memory_percent: p.memory_percent,
                        disk_read_sec: p.disk_read_bytes_sec,
                        disk_write_sec: p.disk_written_bytes_sec,
                        cmdline: p.cmdline.clone(),
                        domain_label: p.domain.label(),
                        is_group_header: false,
                        has_children: false,
                        is_expanded: false,
                        group_key: None,
                        actual_pid: p.pid,
                        original_metric: Some(p),
                    })
                    .collect();
            }

            ProcessViewType::Grouped => {
                self.build_grouped_rows(&filtered, |p| p.name.clone());
            }

            ProcessViewType::Container => {
                self.build_grouped_rows(&filtered, |p| p.domain.label());
            }

            ProcessViewType::Tree => {
                let mut proc_map: HashMap<u32, &ProcessMetric> = HashMap::new();
                let mut children_map: HashMap<u32, Vec<&ProcessMetric>> = HashMap::new();

                for p in &filtered {
                    proc_map.insert(p.pid, p);
                }

                let mut roots = Vec::new();
                for p in &filtered {
                    if let Some(ppid) = p.parent_pid {
                        if proc_map.contains_key(&ppid) && ppid != p.pid {
                            children_map.entry(ppid).or_default().push(p);
                        } else {
                            roots.push(p);
                        }
                    } else {
                        roots.push(p);
                    }
                }

                self.sort_processes_ref(&mut roots);
                for list in children_map.values_mut() {
                    self.sort_processes_ref(list);
                }

                let mut rows = Vec::new();
                for root in roots {
                    self.render_tree_node(root, "", &children_map, &mut rows);
                }
                self.cached_rows = rows;
            }
        }
    }

    fn build_grouped_rows<F>(&mut self, filtered: &[ProcessMetric], key_fn: F)
    where
        F: Fn(&ProcessMetric) -> String,
    {
        let mut map: HashMap<String, Vec<&ProcessMetric>> = HashMap::new();
        for p in filtered {
            map.entry(key_fn(p)).or_default().push(p);
        }

        struct GroupAggregate<'a> {
            name: String,
            items: Vec<&'a ProcessMetric>,
            total_cpu: f32,
            total_mem: u64,
            total_mem_pct: f32,
            total_read: u64,
            total_write: u64,
            max_surge: f32,
        }

        let mut groups: Vec<GroupAggregate> = map
            .into_iter()
            .map(|(name, items)| {
                let total_cpu = items.iter().map(|p| p.cpu_usage).sum();
                let total_mem = items.iter().map(|p| p.memory_bytes).sum();
                let total_mem_pct = items.iter().map(|p| p.memory_percent).sum();
                let total_read = items.iter().map(|p| p.disk_read_bytes_sec).sum();
                let total_write = items.iter().map(|p| p.disk_written_bytes_sec).sum();
                let max_surge = items.iter().map(|p| p.cpu_delta).fold(0.0f32, f32::max);
                GroupAggregate {
                    name,
                    items,
                    total_cpu,
                    total_mem,
                    total_mem_pct,
                    total_read,
                    total_write,
                    max_surge,
                }
            })
            .collect();

        groups.sort_by(|a, b| {
            let ord = match self.sort_by {
                ProcessSortBy::Cpu => a
                    .total_cpu
                    .partial_cmp(&b.total_cpu)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ProcessSortBy::Memory => a.total_mem.cmp(&b.total_mem),
                ProcessSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessSortBy::Pid => a.items.len().cmp(&b.items.len()),
                ProcessSortBy::User => a.name.cmp(&b.name),
                ProcessSortBy::Disk => {
                    (a.total_read + a.total_write).cmp(&(b.total_read + b.total_write))
                }
                ProcessSortBy::Surge => a
                    .max_surge
                    .partial_cmp(&b.max_surge)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };
            match self.sort_order {
                SortOrder::Ascending => ord,
                SortOrder::Descending => ord.reverse(),
            }
        });

        let mut rows = Vec::new();
        for g in groups {
            let count = g.items.len();
            if count == 1 {
                let p = g.items[0];
                rows.push(DisplayProcessRow {
                    pid_display: format!("{}", p.pid),
                    name: p.name.clone(),
                    prefix: String::new(),
                    user: p.user.clone(),
                    status: p.status.clone(),
                    cpu_usage: p.cpu_usage,
                    memory_bytes: p.memory_bytes,
                    memory_percent: p.memory_percent,
                    disk_read_sec: p.disk_read_bytes_sec,
                    disk_write_sec: p.disk_written_bytes_sec,
                    cmdline: p.cmdline.clone(),
                    domain_label: p.domain.label(),
                    is_group_header: false,
                    has_children: false,
                    is_expanded: false,
                    group_key: None,
                    actual_pid: p.pid,
                    original_metric: Some(p.clone()),
                });
            } else {
                let is_expanded = self.expanded_groups.contains(&g.name);
                let prefix = if is_expanded { "▼ " } else { "▶ " };
                let status = if g.items.iter().any(|p| p.status == "Run") {
                    "Run".to_string()
                } else {
                    "Sleep".to_string()
                };
                let user = g.items[0].user.clone();
                let first_pid = g.items[0].pid;

                let title = if self.process_view_type == ProcessViewType::Container {
                    format!("{} ({} procs)", g.name, count)
                } else {
                    format!("{} threads of {}", count, g.name)
                };

                rows.push(DisplayProcessRow {
                    pid_display: format!("[{}]", count),
                    name: g.name.clone(),
                    prefix: prefix.to_string(),
                    user,
                    status,
                    cpu_usage: g.total_cpu,
                    memory_bytes: g.total_mem,
                    memory_percent: g.total_mem_pct,
                    disk_read_sec: g.total_read,
                    disk_write_sec: g.total_write,
                    cmdline: title,
                    domain_label: g.items[0].domain.label(),
                    is_group_header: true,
                    has_children: true,
                    is_expanded,
                    group_key: Some(g.name.clone()),
                    actual_pid: first_pid,
                    original_metric: Some(g.items[0].clone()),
                });

                if is_expanded {
                    let mut sorted_children = g.items;
                    self.sort_processes_ref(&mut sorted_children);
                    for (idx, child) in sorted_children.iter().enumerate() {
                        let branch = if idx == count - 1 {
                            "  └─ "
                        } else {
                            "  ├─ "
                        };
                        rows.push(DisplayProcessRow {
                            pid_display: format!("{}", child.pid),
                            name: child.name.clone(),
                            prefix: branch.to_string(),
                            user: child.user.clone(),
                            status: child.status.clone(),
                            cpu_usage: child.cpu_usage,
                            memory_bytes: child.memory_bytes,
                            memory_percent: child.memory_percent,
                            disk_read_sec: child.disk_read_bytes_sec,
                            disk_write_sec: child.disk_written_bytes_sec,
                            cmdline: child.cmdline.clone(),
                            domain_label: child.domain.label(),
                            is_group_header: false,
                            has_children: false,
                            is_expanded: false,
                            group_key: Some(g.name.clone()),
                            actual_pid: child.pid,
                            original_metric: Some((*child).clone()),
                        });
                    }
                }
            }
        }
        self.cached_rows = rows;
    }

    fn render_tree_node(
        &self,
        node: &ProcessMetric,
        prefix: &str,
        children_map: &HashMap<u32, Vec<&ProcessMetric>>,
        rows: &mut Vec<DisplayProcessRow>,
    ) {
        let children = children_map.get(&node.pid);
        let has_children = children.map(|c| !c.is_empty()).unwrap_or(false);
        let is_collapsed = self.collapsed_tree_pids.contains(&node.pid);
        let is_expanded = has_children && !is_collapsed;

        let branch_icon = if has_children {
            if is_expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            ""
        };

        rows.push(DisplayProcessRow {
            pid_display: format!("{}", node.pid),
            name: node.name.clone(),
            prefix: format!("{}{}", prefix, branch_icon),
            user: node.user.clone(),
            status: node.status.clone(),
            cpu_usage: node.cpu_usage,
            memory_bytes: node.memory_bytes,
            memory_percent: node.memory_percent,
            disk_read_sec: node.disk_read_bytes_sec,
            disk_write_sec: node.disk_written_bytes_sec,
            cmdline: node.cmdline.clone(),
            domain_label: node.domain.label(),
            is_group_header: false,
            has_children,
            is_expanded,
            group_key: None,
            actual_pid: node.pid,
            original_metric: Some(node.clone()),
        });

        if is_expanded {
            if let Some(kids) = children {
                let count = kids.len();
                for (i, child) in kids.iter().enumerate() {
                    let is_last = i == count - 1;
                    let connector = if is_last { "└─ " } else { "├─ " };
                    let child_indent = if is_last { "   " } else { "│  " };
                    let new_prefix = format!("{}{}", prefix, connector);
                    let deeper_prefix = format!("{}{}", prefix, child_indent);

                    let sub_children = children_map.get(&child.pid);
                    let sub_has = sub_children.map(|c| !c.is_empty()).unwrap_or(false);
                    let sub_collapsed = self.collapsed_tree_pids.contains(&child.pid);
                    let sub_expanded = sub_has && !sub_collapsed;
                    let sub_icon = if sub_has {
                        if sub_expanded {
                            "▼ "
                        } else {
                            "▶ "
                        }
                    } else {
                        ""
                    };

                    rows.push(DisplayProcessRow {
                        pid_display: format!("{}", child.pid),
                        name: child.name.clone(),
                        prefix: format!("{}{}", new_prefix, sub_icon),
                        user: child.user.clone(),
                        status: child.status.clone(),
                        cpu_usage: child.cpu_usage,
                        memory_bytes: child.memory_bytes,
                        memory_percent: child.memory_percent,
                        disk_read_sec: child.disk_read_bytes_sec,
                        disk_write_sec: child.disk_written_bytes_sec,
                        cmdline: child.cmdline.clone(),
                        domain_label: child.domain.label(),
                        is_group_header: false,
                        has_children: sub_has,
                        is_expanded: sub_expanded,
                        group_key: None,
                        actual_pid: child.pid,
                        original_metric: Some((*child).clone()),
                    });

                    if sub_expanded {
                        if let Some(sub_kids) = sub_children {
                            for sub_k in sub_kids {
                                self.render_tree_node(sub_k, &deeper_prefix, children_map, rows);
                            }
                        }
                    }
                }
            }
        }
    }

    fn sort_processes(&self, list: &mut [ProcessMetric]) {
        list.sort_by(|a, b| {
            let ordering = match self.sort_by {
                ProcessSortBy::Cpu => a
                    .cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ProcessSortBy::Memory => a.memory_bytes.cmp(&b.memory_bytes),
                ProcessSortBy::Pid => a.pid.cmp(&b.pid),
                ProcessSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessSortBy::User => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
                ProcessSortBy::Disk => (a.disk_read_bytes_sec + a.disk_written_bytes_sec)
                    .cmp(&(b.disk_read_bytes_sec + b.disk_written_bytes_sec)),
                ProcessSortBy::Surge => a
                    .cpu_delta
                    .partial_cmp(&b.cpu_delta)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });
    }

    fn sort_processes_ref(&self, list: &mut [&ProcessMetric]) {
        list.sort_by(|a, b| {
            let ordering = match self.sort_by {
                ProcessSortBy::Cpu => a
                    .cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ProcessSortBy::Memory => a.memory_bytes.cmp(&b.memory_bytes),
                ProcessSortBy::Pid => a.pid.cmp(&b.pid),
                ProcessSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessSortBy::User => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
                ProcessSortBy::Disk => (a.disk_read_bytes_sec + a.disk_written_bytes_sec)
                    .cmp(&(b.disk_read_bytes_sec + b.disk_written_bytes_sec)),
                ProcessSortBy::Surge => a
                    .cpu_delta
                    .partial_cmp(&b.cpu_delta)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
        });
    }

    pub fn selected_process(&self) -> Option<ProcessMetric> {
        if self.cached_rows.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(self.cached_rows.len() - 1);
            self.cached_rows[idx].original_metric.clone()
        }
    }

    pub fn next_process(&mut self) {
        let count = self.cached_rows.len();
        if count == 0 {
            return;
        }
        if self.selected_index + 1 < count {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
        self.table_state.select(Some(self.selected_index));
    }

    pub fn previous_process(&mut self) {
        let count = self.cached_rows.len();
        if count == 0 {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = count.saturating_sub(1);
        }
        self.table_state.select(Some(self.selected_index));
    }

    pub fn page_down(&mut self, page_size: usize) {
        let count = self.cached_rows.len();
        if count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + page_size).min(count.saturating_sub(1));
        self.table_state.select(Some(self.selected_index));
    }

    pub fn page_up(&mut self, page_size: usize) {
        let count = self.cached_rows.len();
        if count == 0 {
            return;
        }
        self.selected_index = self.selected_index.saturating_sub(page_size);
        self.table_state.select(Some(self.selected_index));
    }

    pub fn scroll_to_top(&mut self) {
        self.selected_index = 0;
        self.table_state.select(Some(0));
    }

    pub fn scroll_to_bottom(&mut self) {
        let count = self.cached_rows.len();
        if count > 0 {
            self.selected_index = count - 1;
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn set_sort(&mut self, sort_by: ProcessSortBy) {
        if self.sort_by == sort_by {
            self.sort_order = self.sort_order.toggle();
        } else {
            self.sort_by = sort_by;
            self.sort_order = match sort_by {
                ProcessSortBy::Cpu
                | ProcessSortBy::Memory
                | ProcessSortBy::Disk
                | ProcessSortBy::Surge => SortOrder::Descending,
                ProcessSortBy::Pid | ProcessSortBy::Name | ProcessSortBy::User => {
                    SortOrder::Ascending
                }
            };
        }
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.recompute_rows();
        self.clamp_selection();
    }

    pub fn toggle_kernel_threads(&mut self) {
        self.hide_kernel_threads = !self.hide_kernel_threads;
        self.recompute_rows();
        self.clamp_selection();
        self.show_toast(if self.hide_kernel_threads {
            "Kernel threads hidden".into()
        } else {
            "Showing all processes & threads".into()
        });
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Balanced => ViewMode::ProcFocus,
            ViewMode::ProcFocus => ViewMode::GraphsFocus,
            ViewMode::GraphsFocus => ViewMode::Balanced,
        };
    }

    pub fn open_inspect_modal(&mut self) {
        if let Some(row) = self.cached_rows.get(self.selected_index) {
            if row.has_children {
                self.toggle_expand_selected();
                return;
            }
        }

        if let Some(proc) = self.selected_process() {
            self.modal_scroll = 0;
            self.active_modal = ActiveModal::ProcessInspect(proc);
        }
    }

    pub fn open_sockets_modal(&mut self) {
        if let Some(proc) = self.selected_process() {
            let sockets = SocketInspector::inspect_pid(proc.pid);
            if sockets.is_empty() {
                self.show_toast(format!("No open network sockets for PID {}", proc.pid));
            } else {
                self.modal_scroll = 0;
                self.active_modal = ActiveModal::ProcessSockets {
                    pid: proc.pid,
                    name: proc.name,
                    sockets,
                };
            }
        }
    }

    pub fn open_environ_modal(&mut self) {
        if let Some(proc) = self.selected_process() {
            let path = format!("/proc/{}/environ", proc.pid);
            let mut vars = Vec::new();
            if let Ok(bytes) = fs::read(path) {
                for entry in bytes.split(|&b| b == 0) {
                    if let Ok(s) = std::str::from_utf8(entry) {
                        if let Some((k, v)) = s.split_once('=') {
                            vars.push((k.to_string(), v.to_string()));
                        }
                    }
                }
            }
            if vars.is_empty() {
                self.show_toast(format!(
                    "Environment empty or access restricted for PID {}",
                    proc.pid
                ));
            } else {
                self.modal_scroll = 0;
                self.active_modal = ActiveModal::ProcessEnviron {
                    pid: proc.pid,
                    name: proc.name,
                    vars,
                };
            }
        }
    }

    pub fn open_logs_modal(&mut self) {
        if let Some(proc) = self.selected_process() {
            let mut logs = Vec::new();
            match &proc.domain {
                ProcessGroupDomain::Docker(container) => {
                    if let Ok(output) = Command::new("docker")
                        .args(["logs", "--tail", "50", container])
                        .output()
                    {
                        let text = String::from_utf8_lossy(&output.stdout);
                        let err_text = String::from_utf8_lossy(&output.stderr);
                        for line in text.lines().chain(err_text.lines()) {
                            logs.push(line.to_string());
                        }
                    }
                }
                ProcessGroupDomain::Service(svc) => {
                    if let Ok(output) = Command::new("journalctl")
                        .args(["-u", svc, "-n", "50", "--no-pager"])
                        .output()
                    {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            logs.push(line.to_string());
                        }
                    }
                }
                _ => {
                    if let Ok(output) = Command::new("journalctl")
                        .args(["_PID", &proc.pid.to_string(), "-n", "50", "--no-pager"])
                        .output()
                    {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            logs.push(line.to_string());
                        }
                    }
                }
            }

            if logs.is_empty() {
                self.show_toast(format!("No log records found for PID {}", proc.pid));
            } else {
                self.modal_scroll = 0;
                self.active_modal = ActiveModal::ProcessLogs {
                    pid: proc.pid,
                    name: proc.name,
                    logs,
                };
            }
        }
    }

    pub fn open_kill_modal(&mut self) {
        if let Some(proc) = self.selected_process() {
            self.active_modal = ActiveModal::ProcessKill {
                pid: proc.pid,
                name: proc.name,
            };
        }
    }

    pub fn kill_process_with_signal(&mut self, signal_num: i32) {
        if let ActiveModal::ProcessKill { pid, ref name } = self.active_modal {
            let res = Command::new("kill")
                .arg(format!("-{}", signal_num))
                .arg(pid.to_string())
                .output();

            match res {
                Ok(out) if out.status.success() => {
                    self.show_toast(format!("Sent SIG{} to PID {} ({})", signal_num, pid, name));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    self.show_toast(format!("Failed to kill PID {}: {}", pid, err.trim()));
                }
                Err(e) => {
                    self.show_toast(format!("Kill error: {}", e));
                }
            }
        }
        self.active_modal = ActiveModal::None;
    }

    pub fn active_network_metric(&self) -> Option<crate::metrics::NetworkMetric> {
        let m = self.metrics.as_ref()?;
        if let Some(ref iface) = self.selected_interface {
            if let Some(net) = m.per_interface.get(iface) {
                return Some(net.clone());
            }
        }
        Some(m.network.clone())
    }

    pub fn cycle_network_interface(&mut self) {
        let Some(ref metrics) = self.metrics else {
            return;
        };

        let iface_names: Vec<String> = metrics.interfaces.iter().map(|i| i.name.clone()).collect();
        if iface_names.is_empty() {
            self.selected_interface = None;
            self.show_toast("Network: ALL (Aggregated)".into());
            return;
        }

        let next_iface = match self.selected_interface {
            None => Some(iface_names[0].clone()),
            Some(ref current) => {
                if let Some(idx) = iface_names.iter().position(|n| n == current) {
                    if idx + 1 < iface_names.len() {
                        Some(iface_names[idx + 1].clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        match next_iface {
            Some(ref name) => {
                let rx_str = metrics
                    .per_interface
                    .get(name)
                    .map(|n| crate::theme::Theme::format_bytes_compact(n.rx_bytes_sec))
                    .unwrap_or_else(|| "0B".into());
                let tx_str = metrics
                    .per_interface
                    .get(name)
                    .map(|n| crate::theme::Theme::format_bytes_compact(n.tx_bytes_sec))
                    .unwrap_or_else(|| "0B".into());
                self.show_toast(format!(
                    "Network Interface: {} (↓ {}/s, ↑ {}/s)",
                    name, rx_str, tx_str
                ));
            }
            None => {
                self.show_toast("Network Interface: ALL (Aggregated)".into());
            }
        }

        self.selected_interface = next_iface;
    }

    pub fn open_renice_modal(&mut self) {
        if let Some(proc) = self.selected_process() {
            self.active_modal = ActiveModal::ProcessRenice {
                pid: proc.pid,
                name: proc.name,
                current_nice: proc.nice,
                target_nice: proc.nice,
            };
        }
    }

    pub fn adjust_renice_target(&mut self, delta: i32) {
        if let ActiveModal::ProcessRenice {
            ref mut target_nice,
            ..
        } = self.active_modal
        {
            *target_nice = (*target_nice + delta).clamp(-20, 19);
        }
    }

    pub fn set_renice_preset(&mut self, preset: i32) {
        if let ActiveModal::ProcessRenice {
            ref mut target_nice,
            ..
        } = self.active_modal
        {
            *target_nice = preset.clamp(-20, 19);
        }
    }

    pub fn apply_renice(&mut self) {
        if let ActiveModal::ProcessRenice {
            pid,
            ref name,
            target_nice,
            ..
        } = self.active_modal
        {
            let res = Command::new("renice")
                .arg("-n")
                .arg(target_nice.to_string())
                .arg(pid.to_string())
                .output();

            match res {
                Ok(out) if out.status.success() => {
                    self.show_toast(format!(
                        "✓ Reniced PID {} ({}) to nice {}",
                        pid, name, target_nice
                    ));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    self.show_toast(format!("✗ Renice PID {} failed: {}", pid, err.trim()));
                }
                Err(e) => {
                    self.show_toast(format!("✗ Renice command error: {}", e));
                }
            }
        }
        self.active_modal = ActiveModal::None;
    }

    pub fn quick_renice(&mut self, delta: i32) {
        if let Some(proc) = self.selected_process() {
            let new_nice = (proc.nice + delta).clamp(-20, 19);
            let res = Command::new("renice")
                .arg("-n")
                .arg(new_nice.to_string())
                .arg(proc.pid.to_string())
                .output();

            match res {
                Ok(out) if out.status.success() => {
                    self.show_toast(format!(
                        "✓ Reniced PID {} ({}) to {}",
                        proc.pid, proc.name, new_nice
                    ));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    self.show_toast(format!("✗ Renice error: {}", err.trim()));
                }
                Err(e) => {
                    self.show_toast(format!("✗ Renice error: {}", e));
                }
            }
        }
    }

    pub fn open_user_filter_modal(&mut self) {
        let Some(ref metrics) = self.metrics else {
            return;
        };

        let mut user_counts: HashMap<String, usize> = HashMap::new();
        for p in &metrics.processes {
            *user_counts.entry(p.user.clone()).or_insert(0) += 1;
        }

        let mut users: Vec<(String, usize)> = user_counts.into_iter().collect();
        users.sort_by_key(|a| std::cmp::Reverse(a.1));

        let current_idx = if let Some(ref current) = self.user_filter {
            users
                .iter()
                .position(|(u, _)| u == current)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        self.active_modal = ActiveModal::UserFilter {
            selected_idx: current_idx,
            users,
        };
    }

    pub fn select_user_filter(&mut self, user: Option<String>) {
        if let Some(ref u) = user {
            self.show_toast(format!("Filtering by user: {}", u));
        } else {
            self.show_toast("User filter cleared (Showing ALL users)".into());
        }
        self.user_filter = user;
        self.recompute_rows();
        self.clamp_selection();
        self.active_modal = ActiveModal::None;
    }

    pub fn save_config_file(&mut self) {
        self.config.sample_interval_ms = self.sample_interval_ms.load(Ordering::Relaxed);
        self.config.theme = match self.theme_mode {
            ThemeMode::TokyoSlate => "tokyo_slate".into(),
            ThemeMode::CatppuccinMocha => "catppuccin".into(),
            ThemeMode::GruvboxDark => "gruvbox".into(),
            ThemeMode::NordicFrost => "nord".into(),
            ThemeMode::MonokaiMinimal => "monokai".into(),
        };
        self.config.sort_by = self.sort_by.label().to_lowercase();
        self.config.sort_order = match self.sort_order {
            SortOrder::Ascending => "asc".into(),
            SortOrder::Descending => "desc".into(),
        };
        self.config.view_mode = match self.view_mode {
            ViewMode::Balanced => "balanced".into(),
            ViewMode::ProcFocus => "proc_focus".into(),
            ViewMode::GraphsFocus => "graphs_focus".into(),
        };
        self.config.process_view_type = match self.process_view_type {
            ProcessViewType::Grouped => "grouped".into(),
            ProcessViewType::Flat => "flat".into(),
            ProcessViewType::Tree => "tree".into(),
            ProcessViewType::Container => "container".into(),
        };
        self.config.hide_kernel_threads = self.hide_kernel_threads;
        self.config.network_interface = self.selected_interface.clone();
        self.config.user_filter = self.user_filter.clone();

        match self.config.save() {
            Ok(p) => {
                self.show_toast(format!("✓ Saved configuration to {}", p.display()));
            }
            Err(e) => {
                self.show_toast(format!("✗ Failed to save config: {}", e));
            }
        }
    }

    pub fn save_snapshot(&mut self) {
        let Some(ref metrics) = self.metrics else {
            self.show_toast("No metrics snapshot available yet".into());
            return;
        };

        match crate::export::save_snapshot_file(metrics) {
            Ok(filename) => {
                self.show_toast(format!("✓ Saved telemetry snapshot to {}", filename));
            }
            Err(e) => {
                self.show_toast(format!("✗ Snapshot error: {}", e));
            }
        }
    }

    pub fn toggle_help(&mut self) {
        if self.active_modal == ActiveModal::Help {
            self.active_modal = ActiveModal::None;
        } else {
            self.active_modal = ActiveModal::Help;
        }
    }

    pub fn close_modal(&mut self) {
        self.active_modal = ActiveModal::None;
    }

    pub fn modal_scroll_down(&mut self, delta: usize) {
        self.modal_scroll += delta;
    }

    pub fn modal_scroll_up(&mut self, delta: usize) {
        self.modal_scroll = self.modal_scroll.saturating_sub(delta);
    }

    pub fn increase_sample_speed(&mut self) {
        let current = self.sample_interval_ms.load(Ordering::Relaxed);
        let next = match current {
            5000 => 2000,
            2000 => 1000,
            1000 => 500,
            500 => 250,
            _ => 100,
        };
        self.sample_interval_ms.store(next, Ordering::Relaxed);
        if next == 100 {
            self.show_toast("⚡ Sampling: 100ms (MAX SPEED)".into());
        } else {
            self.show_toast(format!(
                "Sampling: {}ms ({:.2}s)",
                next,
                next as f32 / 1000.0
            ));
        }
    }

    pub fn decrease_sample_speed(&mut self) {
        let current = self.sample_interval_ms.load(Ordering::Relaxed);
        let next = match current {
            100 => 250,
            250 => 500,
            500 => 1000,
            1000 => 2000,
            _ => 5000,
        };
        self.sample_interval_ms.store(next, Ordering::Relaxed);
        self.show_toast(format!(
            "Sampling: {}ms ({:.2}s)",
            next,
            next as f32 / 1000.0
        ));
    }

    pub fn show_toast(&mut self, message: String) {
        self.status_toast = Some((message, std::time::Instant::now()));
    }

    pub fn active_toast(&self) -> Option<&str> {
        if let Some((ref msg, time)) = self.status_toast {
            if time.elapsed() < std::time::Duration::from_secs(3) {
                return Some(msg);
            }
        }
        None
    }

    pub fn quit(&mut self) {
        self.is_running = false;
    }

    pub fn handle_mouse_click(&mut self, col: u16, row: u16) {
        if row <= 2 {
            if col < 15 {
                self.cycle_theme();
            } else if col >= 85 {
                self.increase_sample_speed();
            }
            return;
        }

        if row > 2 && row < self.table_header_y {
            // Upper hardware charts area:
            // Right half network chart: cycle interfaces
            if col >= 65 {
                self.cycle_network_interface();
                return;
            }
        }

        if row == self.table_header_y {
            if col < 8 {
                self.set_sort(ProcessSortBy::Pid);
            } else if col < 18 {
                self.set_sort(ProcessSortBy::User);
            } else if col < 38 {
                self.set_sort(ProcessSortBy::Cpu);
            } else if col < 52 {
                self.set_sort(ProcessSortBy::Memory);
            } else if col < 66 {
                self.set_sort(ProcessSortBy::Disk);
            } else {
                self.set_sort(ProcessSortBy::Name);
            }
            return;
        }

        if row > self.table_header_y && row < self.table_area_y + self.table_area_height {
            let offset = self.table_state.offset();
            let relative_row = (row - self.table_header_y - 1) as usize;
            let target_idx = offset + relative_row;

            if target_idx < self.cached_rows.len() {
                if self.selected_index == target_idx {
                    if self.cached_rows[target_idx].has_children {
                        self.toggle_expand_selected();
                    } else {
                        self.open_inspect_modal();
                    }
                } else {
                    self.selected_index = target_idx;
                    self.table_state.select(Some(self.selected_index));
                }
            }
        }
    }

    pub fn handle_mouse_scroll(&mut self, up: bool) {
        if self.active_modal != ActiveModal::None {
            if up {
                self.modal_scroll_up(3);
            } else {
                self.modal_scroll_down(3);
            }
            return;
        }

        if up {
            for _ in 0..3 {
                self.previous_process();
            }
        } else {
            for _ in 0..3 {
                self.next_process();
            }
        }
    }

    fn clamp_selection(&mut self) {
        let count = self.cached_rows.len();
        if count == 0 {
            self.selected_index = 0;
            self.table_state.select(None);
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
            self.table_state.select(Some(self.selected_index));
        } else {
            self.table_state.select(Some(self.selected_index));
        }
    }
}
