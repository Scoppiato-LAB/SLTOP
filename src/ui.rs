use crate::app::{ActiveModal, App, ProcessSortBy, ProcessViewType, ViewMode};
use crate::braille::{render_braille_chart, render_network_braille_chart};
use crate::theme::{Theme, ThemeColors};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Main UI entry point that renders the full dashboard and any active modals
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let p = Theme::palette(app.theme_mode);

    let (header_len, mid_pct, table_min) = match app.view_mode {
        ViewMode::Balanced => (3, 42, 14),
        ViewMode::ProcFocus => (3, 22, 22),
        ViewMode::GraphsFocus => (3, 68, 6),
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_len),
            Constraint::Percentage(mid_pct),
            Constraint::Min(table_min),
            Constraint::Length(1),
        ])
        .split(size);

    render_command_strip(frame, main_chunks[0], app, &p);
    render_hardware_panels(frame, main_chunks[1], app, &p);
    render_process_grid(frame, main_chunks[2], app, &p);
    render_footer(frame, main_chunks[3], app, &p);

    // Overlay active modal popups
    match app.active_modal.clone() {
        ActiveModal::Help => render_help_modal(frame, size, &p),
        ActiveModal::ProcessInspect(ref proc) => render_inspect_modal(frame, size, proc, &p),
        ActiveModal::ProcessKill { pid, ref name } => render_kill_modal(frame, size, pid, name, &p),
        ActiveModal::ProcessRenice {
            pid,
            ref name,
            current_nice,
            target_nice,
        } => render_renice_modal(frame, size, pid, name, current_nice, target_nice, &p),
        ActiveModal::UserFilter {
            selected_idx,
            ref users,
        } => render_user_filter_modal(
            frame,
            size,
            selected_idx,
            users,
            app.user_filter.as_deref(),
            &p,
        ),
        ActiveModal::ProcessSockets {
            pid,
            ref name,
            ref sockets,
        } => render_sockets_modal(frame, size, pid, name, sockets, app.modal_scroll, &p),
        ActiveModal::ProcessEnviron {
            pid,
            ref name,
            ref vars,
        } => render_environ_modal(frame, size, pid, name, vars, app.modal_scroll, &p),
        ActiveModal::ProcessLogs {
            pid,
            ref name,
            ref logs,
        } => render_logs_modal(frame, size, pid, name, logs, app.modal_scroll, &p),
        ActiveModal::None => {}
    }
}

/// Top Unified Command Strip: Clean telemetry overview + Surge & PSI alerts
fn render_command_strip(frame: &mut Frame, area: Rect, app: &App, p: &ThemeColors) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref metrics) = app.metrics else {
        return;
    };

    let rate = app
        .sample_interval_ms
        .load(std::sync::atomic::Ordering::Relaxed);
    let rate_color = if rate <= 100 {
        p.cyan
    } else {
        p.text_secondary
    };

    let clean_os = if metrics.os_name.contains("Debian") {
        "Debian"
    } else if metrics.os_name.contains("Ubuntu") {
        "Ubuntu"
    } else if metrics.os_name.contains("Arch") {
        "Arch"
    } else {
        &metrics.os_name
    };

    let short_kernel = metrics
        .kernel_version
        .split(['+', '-'])
        .next()
        .unwrap_or(&metrics.kernel_version);

    let right_width = 43u16;
    let left_width = inner.width.saturating_sub(right_width);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width),
            Constraint::Length(right_width),
        ])
        .split(inner);

    // Col 1: System Identity, Theme, Uptime & Surge Detector
    let lab_green = Color::Rgb(0, 230, 118);
    let mut left_spans = vec![
        Span::styled(
            " sltop ",
            Style::default()
                .bg(p.blue)
                .fg(p.bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Scoppiato LAB ",
            Style::default()
                .bg(lab_green)
                .fg(Color::Rgb(10, 16, 20))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" [{}]", app.theme_mode.name()),
            Style::default().fg(p.lavender),
        ),
        Span::styled(" Host: ", Style::default().fg(p.text_muted)),
        Span::styled(
            &metrics.host_name,
            Style::default()
                .fg(p.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(ref iface) = app.selected_interface {
        left_spans.push(Span::styled(
            format!(" [Net: {}]", iface),
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(ref u) = app.user_filter {
        left_spans.push(Span::styled(
            format!(" [User: {}]", u),
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ));
    }

    if left_width >= 80 {
        left_spans.push(Span::styled(
            format!(" ({clean_os})"),
            Style::default().fg(p.text_secondary),
        ));
    }

    left_spans.push(Span::styled(" │ Up: ", Style::default().fg(p.text_muted)));
    left_spans.push(Span::styled(
        Theme::format_duration(metrics.uptime_seconds),
        Style::default().fg(p.mint),
    ));

    if let Some((ref surge_name, surge_pid, surge_delta)) = metrics.surge_process {
        if left_width >= 92 {
            left_spans.push(Span::styled(
                " │ ⚡ Surge: ",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ));
            left_spans.push(Span::styled(
                format!("{surge_name} ({surge_pid}) +{surge_delta:.0}%"),
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ));
        } else if left_width >= 65 {
            left_spans.push(Span::styled(
                " │ ⚡ ",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ));
            left_spans.push(Span::styled(
                format!("{surge_name} +{surge_delta:.0}%"),
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ));
        }
    }

    frame.render_widget(Paragraph::new(Line::from(left_spans)), cols[0]);

    // Col 2: Load, Kernel, Sampling Rate (Right Aligned)
    let right_line = Line::from(vec![
        Span::styled("Load: ", Style::default().fg(p.text_muted)),
        Span::styled(
            format!("{:.2} ", metrics.load_avg.0),
            Style::default()
                .fg(p.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:.2} ", metrics.load_avg.1),
            Style::default().fg(p.text_secondary),
        ),
        Span::styled(
            format!("{:.2}", metrics.load_avg.2),
            Style::default().fg(p.text_muted),
        ),
        Span::styled(" │ ", Style::default().fg(p.text_muted)),
        Span::styled(short_kernel, Style::default().fg(p.text_secondary)),
        Span::styled(
            " │ ⚡ ",
            Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}ms", rate),
            Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(right_line).alignment(Alignment::Right),
        cols[1],
    );
}

/// Hardware overview panel:
/// Tier 1: Dual Telemetry Charts (CPU History + Network Throughput)
/// Tier 2: System Gauges (CPU Status & Cores + Memory & Storage)
fn render_hardware_panels(frame: &mut Frame, area: Rect, app: &App, p: &ThemeColors) {
    let Some(ref metrics) = app.metrics else {
        return;
    };

    // If available height is very tight (e.g. <= 9 rows on small terminals), show only the charts
    if area.height <= 9 {
        render_dual_charts(frame, area, app, metrics, p);
        return;
    }

    let disk_count = metrics.disks.len();
    let core_count = metrics.cpu_cores.len();
    let core_rows = core_count.div_ceil(2).clamp(1, 4) as u16;

    // Harmonize Gauges height (cores + storage):
    let needed_gauges = (3 + disk_count as u16).max(1 + core_rows) + 2;
    let gauges_height = needed_gauges.clamp(5, 7).min(area.height.saturating_sub(5));

    let tiers = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),                // Tier 1: Dual Telemetry Charts (TOP)
            Constraint::Length(gauges_height), // Tier 2: System Gauges (BELOW GRAPHS)
        ])
        .split(area);

    let gauges_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(tiers[1]);

    render_dual_charts(frame, tiers[0], app, metrics, p);
    render_cpu_status_panel(frame, gauges_cols[0], metrics, p);
    render_memory_storage_panel(frame, gauges_cols[1], app, p);
}

/// Renders side-by-side Dual Telemetry Charts
fn render_dual_charts(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    metrics: &crate::metrics::SystemMetrics,
    p: &ThemeColors,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_cpu_chart_panel(frame, cols[0], metrics, p);
    render_network_chart_panel(frame, cols[1], app, p);
}

/// CPU status panel (Total usage bar, frequency, temp, PSI, core grid)
fn render_cpu_status_panel(
    frame: &mut Frame,
    area: Rect,
    metrics: &crate::metrics::SystemMetrics,
    p: &ThemeColors,
) {
    let core_count = metrics.cpu_cores.len();
    let clean_model = Theme::clean_cpu_model(&metrics.cpu_model);

    let title = if area.width >= 48 && !clean_model.is_empty() {
        format!(" [ CPU STATUS - {core_count} Cores │ {clean_model} ] ")
    } else {
        format!(" [ CPU STATUS - {core_count} Cores ] ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .title(Span::styled(
            title,
            Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let global_cpu = metrics.cpu_global_usage;

    let mut cpu_lines = Vec::new();

    // Total CPU Bar with temperature and pressure indicators
    let total_bar_w = if inner.width >= 70 {
        20
    } else if inner.width >= 50 {
        16
    } else {
        12
    };
    let mut total_spans = vec![Span::styled("Total: ", Style::default().fg(p.text_muted))];
    total_spans.extend(Theme::render_bar_spans(
        p,
        global_cpu / 100.0,
        total_bar_w,
        None,
    ));
    total_spans.push(Span::styled(
        format!(" {:5.1}%", global_cpu),
        Style::default()
            .fg(Theme::usage_color_with_palette(p, global_cpu))
            .add_modifier(Modifier::BOLD),
    ));

    if let Some(temp) = metrics.cpu_temp_celsius {
        let temp_color = if temp > 80.0 {
            p.coral
        } else if temp > 65.0 {
            p.amber
        } else {
            p.mint
        };
        total_spans.push(Span::styled(
            format!(" {:.0}°C", temp),
            Style::default().fg(temp_color).add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(ref psi) = metrics.psi {
        let psi_color = if psi.cpu_some > 15.0 {
            p.coral
        } else if psi.cpu_some > 5.0 {
            p.amber
        } else {
            p.teal
        };
        total_spans.push(Span::styled(
            format!(" │ PSI CPU:{:.1}%", psi.cpu_some),
            Style::default().fg(psi_color).add_modifier(Modifier::BOLD),
        ));
        if psi.io_some > 1.0 {
            total_spans.push(Span::styled(
                format!(" IO:{:.1}%", psi.io_some),
                Style::default().fg(p.amber),
            ));
        }
    }

    cpu_lines.push(Line::from(total_spans));

    // Per-core breakdown
    let max_core_rows = inner.height.saturating_sub(1) as usize;
    let bar_w = if inner.width >= 70 {
        12
    } else if inner.width >= 50 {
        10
    } else {
        6
    };

    for row in 0..max_core_rows {
        let idx_l = row * 2;
        let idx_r = idx_l + 1;

        if idx_l >= metrics.cpu_cores.len() {
            break;
        }

        let mut row_spans = Vec::new();

        let c_l = &metrics.cpu_cores[idx_l];
        row_spans.push(Span::styled(
            format!("{:<4} ", c_l.name),
            Style::default()
                .fg(p.text_secondary)
                .add_modifier(Modifier::BOLD),
        ));
        row_spans.extend(Theme::micro_bar_spans(p, c_l.usage, bar_w, None));
        row_spans.extend(vec![
            Span::styled(
                format!(" {:4.1}% ", c_l.usage),
                Style::default()
                    .fg(Theme::usage_color_with_palette(p, c_l.usage))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:4}MHz", c_l.frequency_mhz),
                Style::default().fg(p.text_muted),
            ),
        ]);
        if let Some(temp) = c_l.temp_celsius {
            row_spans.push(Span::styled(
                format!(" {:.0}°C", temp),
                Style::default().fg(p.text_secondary),
            ));
        }

        if idx_r < metrics.cpu_cores.len() {
            let c_r = &metrics.cpu_cores[idx_r];
            row_spans.push(Span::styled(" │ ", Style::default().fg(p.card_border)));
            row_spans.push(Span::styled(
                format!("{:<4} ", c_r.name),
                Style::default()
                    .fg(p.text_secondary)
                    .add_modifier(Modifier::BOLD),
            ));
            row_spans.extend(Theme::micro_bar_spans(p, c_r.usage, bar_w, None));
            row_spans.extend(vec![
                Span::styled(
                    format!(" {:4.1}% ", c_r.usage),
                    Style::default()
                        .fg(Theme::usage_color_with_palette(p, c_r.usage))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:4}MHz", c_r.frequency_mhz),
                    Style::default().fg(p.text_muted),
                ),
            ]);
            if let Some(temp) = c_r.temp_celsius {
                row_spans.push(Span::styled(
                    format!(" {:.0}°C", temp),
                    Style::default().fg(p.text_secondary),
                ));
            }
        }

        cpu_lines.push(Line::from(row_spans));
    }

    // PSI pressure details
    if cpu_lines.len() < inner.height as usize {
        if let Some(ref psi) = metrics.psi {
            let line_w = (inner.width as usize).saturating_sub(18);
            let psi_header = format!("─ Linux Pressure Stall {}", "─".repeat(line_w));
            cpu_lines.push(Line::from(vec![Span::styled(
                psi_header,
                Style::default().fg(p.card_border),
            )]));
            cpu_lines.push(Line::from(vec![
                Span::styled("CPU Stall: ", Style::default().fg(p.text_muted)),
                Span::styled(
                    format!("{:.1}% (some) ", psi.cpu_some),
                    Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
                ),
                Span::styled("│ MEM: ", Style::default().fg(p.card_border)),
                Span::styled(
                    format!("{:.1}% ", psi.mem_some),
                    Style::default().fg(p.lavender),
                ),
                Span::styled("│ I/O: ", Style::default().fg(p.card_border)),
                Span::styled(format!("{:.1}%", psi.io_some), Style::default().fg(p.amber)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(cpu_lines), inner);
}

/// CPU usage chart panel with Braille waveform
fn render_cpu_chart_panel(
    frame: &mut Frame,
    area: Rect,
    metrics: &crate::metrics::SystemMetrics,
    p: &ThemeColors,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .title(Span::styled(
            " [ CPU HISTORY - Usage Waveform ] ",
            Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Realtime summary
            Constraint::Min(3),    // Braille waveform chart
        ])
        .split(inner);

    let global_cpu = metrics.cpu_global_usage;
    let psi_cpu = metrics.psi.as_ref().map(|s| s.cpu_some).unwrap_or(0.0);

    let summary = Line::from(vec![
        Span::styled("Realtime: ", Style::default().fg(p.text_muted)),
        Span::styled(
            format!("{:>5.1}% ", global_cpu),
            Style::default()
                .fg(Theme::usage_color_with_palette(p, global_cpu))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ Load: ", Style::default().fg(p.card_border)),
        Span::styled(
            format!(
                "{:.2} {:.2} {:.2}",
                metrics.load_avg.0, metrics.load_avg.1, metrics.load_avg.2
            ),
            Style::default().fg(p.text_secondary),
        ),
        Span::styled(" │ PSI: ", Style::default().fg(p.card_border)),
        Span::styled(format!("CPU {:.1}%", psi_cpu), Style::default().fg(p.teal)),
    ]);
    frame.render_widget(Paragraph::new(summary), chunks[0]);

    let graph_height = chunks[1].height as usize;
    let graph_width = chunks[1].width as usize;
    if graph_height > 0 && graph_width > 0 {
        let braille_lines =
            render_braille_chart(p, &metrics.cpu_history, graph_width, graph_height, 100.0);
        let graph_widget = Paragraph::new(braille_lines);
        frame.render_widget(graph_widget, chunks[1]);
    }
}

/// Panel 2: Memory (RAM & SWAP) and Physical Storage + Memory PSI Stall
fn render_memory_storage_panel(frame: &mut Frame, area: Rect, app: &App, p: &ThemeColors) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .title(Span::styled(
            " [ MEMORY & STORAGE ] ",
            Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref metrics) = app.metrics else {
        return;
    };

    let mut lines = Vec::new();

    // RAM Bar
    let mem_bar_w = if inner.width >= 70 {
        20
    } else if inner.width >= 50 {
        16
    } else {
        12
    };
    let ram_ratio = (metrics.mem_percent / 100.0).clamp(0.0, 1.0);
    let mut ram_spans = vec![Span::styled(
        "RAM  ",
        Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
    )];
    ram_spans.extend(Theme::render_bar_spans(
        p,
        ram_ratio,
        mem_bar_w,
        Some(p.lavender),
    ));
    ram_spans.extend(vec![
        Span::styled(
            format!(" {:>5.1}%", metrics.mem_percent),
            Style::default()
                .fg(Theme::usage_color_with_palette(p, metrics.mem_percent))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  {} / {}",
                Theme::format_bytes_compact(metrics.mem_used),
                Theme::format_bytes_compact(metrics.mem_total)
            ),
            Style::default().fg(p.text_primary),
        ),
        Span::styled(
            format!(
                " (Avail: {})",
                Theme::format_bytes_compact(metrics.mem_available)
            ),
            Style::default().fg(p.text_secondary),
        ),
    ]);

    if let Some(ref psi) = metrics.psi {
        if psi.mem_some > 0.1 {
            ram_spans.push(Span::styled(
                format!(" │ Stall {:.1}%", psi.mem_some),
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ));
        }
    }
    lines.push(Line::from(ram_spans));

    // SWAP Bar
    let swap_ratio = (metrics.swap_percent / 100.0).clamp(0.0, 1.0);
    let mut swap_spans = vec![Span::styled(
        "SWAP ",
        Style::default().fg(p.plum).add_modifier(Modifier::BOLD),
    )];
    let swap_color = if metrics.swap_percent > 90.0 {
        p.coral
    } else {
        p.plum
    };
    swap_spans.extend(Theme::render_bar_spans(
        p,
        swap_ratio,
        mem_bar_w,
        Some(swap_color),
    ));
    if metrics.swap_total > 0 {
        swap_spans.extend(vec![
            Span::styled(
                format!(" {:>5.1}%", metrics.swap_percent),
                Style::default().fg(swap_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  {} / {}",
                    Theme::format_bytes_compact(metrics.swap_used),
                    Theme::format_bytes_compact(metrics.swap_total)
                ),
                Style::default().fg(p.text_secondary),
            ),
        ]);
    } else {
        swap_spans.push(Span::styled(
            "   0.0%  Disabled / None",
            Style::default().fg(p.text_muted),
        ));
    }
    lines.push(Line::from(swap_spans));

    // GPU Telemetry (if detected)
    if let Some(ref gpu) = metrics.gpu {
        let mut gpu_spans = vec![Span::styled(
            "GPU  ",
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        )];
        if let Some(usage) = gpu.usage_percent {
            gpu_spans.extend(Theme::render_bar_spans(
                p,
                usage / 100.0,
                mem_bar_w,
                Some(p.mint),
            ));
            gpu_spans.push(Span::styled(
                format!(" {:>5.1}%", usage),
                Style::default()
                    .fg(Theme::usage_color_with_palette(p, usage))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            gpu_spans.push(Span::styled(
                format!("{:<15}", gpu.name),
                Style::default().fg(p.text_secondary),
            ));
        }
        if let (Some(used), Some(total)) = (gpu.mem_used_bytes, gpu.mem_total_bytes) {
            gpu_spans.push(Span::styled(
                format!(
                    "  {} / {}",
                    Theme::format_bytes_compact(used),
                    Theme::format_bytes_compact(total)
                ),
                Style::default().fg(p.text_primary),
            ));
        }
        if let Some(t) = gpu.temp_celsius {
            gpu_spans.push(Span::styled(
                format!(" {:.0}°C", t),
                Style::default().fg(p.amber),
            ));
        }
        lines.push(Line::from(gpu_spans));
    }

    // Storage Filesystems Header
    let line_w = (inner.width as usize).saturating_sub(14);
    let fs_header = format!("─ Filesystems {}", "─".repeat(line_w));
    lines.push(Line::from(vec![Span::styled(
        fs_header,
        Style::default().fg(p.card_border),
    )]));

    // Disks list
    let disk_bar_w = if inner.width >= 70 {
        16
    } else if inner.width >= 50 {
        12
    } else {
        10
    };
    for disk in &metrics.disks {
        let mut disk_spans = vec![Span::styled(
            format!("{:<11} ", disk.mount_point),
            Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
        )];
        disk_spans.extend(Theme::render_bar_spans(
            p,
            disk.used_percent / 100.0,
            disk_bar_w,
            Some(p.teal),
        ));
        disk_spans.extend(vec![
            Span::styled(
                format!(" {:>5.1}% ", disk.used_percent),
                Style::default()
                    .fg(Theme::usage_color_with_palette(p, disk.used_percent))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{}/{} ",
                    Theme::format_bytes_compact(disk.used_bytes),
                    Theme::format_bytes_compact(disk.total_bytes)
                ),
                Style::default().fg(p.text_primary),
            ),
            Span::styled(
                format!("({})", disk.fs_type),
                Style::default().fg(p.text_muted),
            ),
        ]);
        lines.push(Line::from(disk_spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Real-time network throughput and dual Braille waveform (Rx / Tx)
fn render_network_chart_panel(frame: &mut Frame, area: Rect, app: &App, p: &ThemeColors) {
    let iface_display = match app.selected_interface {
        Some(ref name) => name.clone(),
        None => "ALL".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .title(Span::styled(
            format!(" [ NETWORK ({iface_display}) - ↓ Rx / ↑ Tx ] [n: iface] "),
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref metrics) = app.metrics else {
        return;
    };

    let net = app
        .active_network_metric()
        .unwrap_or_else(|| metrics.network.clone());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Clean real-time rates & peak summary
            Constraint::Min(3),    // Dynamic High-Res Braille Waveform
        ])
        .split(inner);

    let rx_pk = Theme::format_bytes_compact(net.peak_rx_sec);
    let tx_pk = Theme::format_bytes_compact(net.peak_tx_sec);
    let total_rx = Theme::format_bytes_compact(net.total_rx_bytes);
    let total_tx = Theme::format_bytes_compact(net.total_tx_bytes);

    let (rx_str, tx_str) = if chunks[0].width >= 68 {
        (
            format!("{}/s", Theme::format_bytes(net.rx_bytes_sec)),
            format!("{}/s", Theme::format_bytes(net.tx_bytes_sec)),
        )
    } else {
        (
            format!("{}/s", Theme::format_bytes_compact(net.rx_bytes_sec)),
            format!("{}/s", Theme::format_bytes_compact(net.tx_bytes_sec)),
        )
    };

    let summary_line = Line::from(vec![
        Span::styled(
            "↓ ",
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<8} ", rx_str),
            Style::default()
                .fg(p.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(Pk {}) ", rx_pk),
            Style::default().fg(p.text_secondary),
        ),
        Span::styled("│ ", Style::default().fg(p.card_border)),
        Span::styled(
            "↑ ",
            Style::default().fg(p.sky).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<8} ", tx_str),
            Style::default()
                .fg(p.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(Pk {}) ", tx_pk),
            Style::default().fg(p.text_secondary),
        ),
        Span::styled("│ ", Style::default().fg(p.card_border)),
        Span::styled(
            format!("Σ {}/{}", total_rx, total_tx),
            Style::default().fg(p.text_muted),
        ),
    ]);
    frame.render_widget(Paragraph::new(summary_line), chunks[0]);

    // High-Resolution Dual Braille Network Waveform
    let graph_height = chunks[1].height as usize;
    let graph_width = chunks[1].width as usize;
    if graph_height > 0 && graph_width > 0 {
        let braille_lines = render_network_braille_chart(
            p,
            &net.rx_history,
            &net.tx_history,
            graph_width,
            graph_height,
        );
        frame.render_widget(Paragraph::new(braille_lines), chunks[1]);
    }
}

/// Process list table with containers, sockets and folding
fn render_process_grid(frame: &mut Frame, area: Rect, app: &mut App, p: &ThemeColors) {
    app.table_area_y = area.y;
    app.table_area_height = area.height;
    app.table_header_y = area.y + 1;

    let all_count = app.metrics.as_ref().map(|m| m.processes.len()).unwrap_or(0);
    let running_count = app.metrics.as_ref().map(|m| m.running_tasks).unwrap_or(0);
    let shown_count = app.cached_rows.len();

    let view_tag = match app.process_view_type {
        ProcessViewType::Grouped => "[Grouped (g)]",
        ProcessViewType::Flat => "[Flat (g)]",
        ProcessViewType::Tree => "[Tree (t)]",
        ProcessViewType::Container => "[Container (C)]",
    };

    let mut title_spans = vec![
        Span::styled(
            format!(" [ PROCESSES ({all_count} Tasks, {running_count} Active) ] "),
            Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Shown: {shown_count} │ "),
            Style::default().fg(p.text_muted),
        ),
        Span::styled(
            format!("View: {view_tag} │ "),
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ),
    ];

    if app.is_searching || !app.search_query.is_empty() {
        title_spans.push(Span::styled("Filter: ", Style::default().fg(p.text_muted)));
        title_spans.push(Span::styled(
            format!("[/{}] │ ", app.search_query),
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(ref u) = app.user_filter {
        title_spans.push(Span::styled("User: ", Style::default().fg(p.text_muted)));
        title_spans.push(Span::styled(
            format!("[{}] │ ", u),
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::styled("Sort: ", Style::default().fg(p.text_muted)));
    title_spans.push(Span::styled(
        format!("[{} {}] │ ", app.sort_by.label(), app.sort_order.symbol()),
        Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
    ));

    title_spans.push(Span::styled("Kernel: ", Style::default().fg(p.text_muted)));
    if app.hide_kernel_threads {
        title_spans.push(Span::styled("[Hidden (K)] ", Style::default().fg(p.orange)));
    } else {
        title_spans.push(Span::styled("[Shown (K)] ", Style::default().fg(p.mint)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border))
        .title(Line::from(title_spans));

    let header_cells = [
        ("PID", ProcessSortBy::Pid),
        ("USER", ProcessSortBy::User),
        ("STATE", ProcessSortBy::Name),
        ("CPU% (Load)", ProcessSortBy::Cpu),
        ("MEM% (Usage)", ProcessSortBy::Memory),
        ("DISK R/W", ProcessSortBy::Disk),
        ("CONTAINER/SVC", ProcessSortBy::Name),
        ("NAME / COMMAND LINE", ProcessSortBy::Name),
    ]
    .into_iter()
    .map(|(name, sort_type)| {
        let is_sorted = app.sort_by == sort_type;
        let mut text = name.to_string();
        if is_sorted
            && (name.starts_with(sort_type.label())
                || (sort_type == ProcessSortBy::Memory && name.starts_with("MEM%"))
                || (sort_type == ProcessSortBy::Disk && name.starts_with("DISK")))
        {
            text.push(' ');
            text.push_str(app.sort_order.symbol());
        }

        let style = if is_sorted {
            Style::default().fg(p.cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(p.text_secondary)
                .add_modifier(Modifier::BOLD)
        };

        Cell::from(text).style(style)
    });

    let header = Row::new(header_cells)
        .style(Style::default().bg(p.surface))
        .height(1);

    let rows: Vec<Row> = app
        .cached_rows
        .iter()
        .map(|row| {
            let cpu_color = Theme::usage_color_with_palette(p, row.cpu_usage);
            let mem_color = Theme::usage_color_with_palette(p, row.memory_percent);

            let (state_icon, state_color) = if row.status == "Run" {
                ("● Run", p.mint)
            } else if row.status == "Sleep" {
                ("○ Slp", p.text_muted)
            } else {
                ("✖ Zmb", p.coral)
            };

            // CPU Cell
            let mut cpu_spans = vec![Span::styled(
                format!("{:5.1}% ", row.cpu_usage),
                Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
            )];
            cpu_spans.extend(Theme::micro_bar_spans(p, row.cpu_usage, 5, None));

            // MEM Cell
            let mut mem_spans = vec![Span::styled(
                format!("{:5.1}% ", row.memory_percent),
                Style::default().fg(mem_color),
            )];
            mem_spans.extend(Theme::micro_bar_spans(
                p,
                row.memory_percent,
                5,
                Some(p.lavender),
            ));

            // DISK R/W Cell
            let disk_rw = if row.disk_read_sec == 0 && row.disk_write_sec == 0 {
                "0B".to_string()
            } else {
                format!(
                    "{}/{}",
                    Theme::format_bytes_compact(row.disk_read_sec),
                    Theme::format_bytes_compact(row.disk_write_sec)
                )
            };

            // Domain Badge (Docker container or systemd service)
            let (domain_text, domain_style) = if row.domain_label.starts_with("docker:") {
                let name = row
                    .domain_label
                    .strip_prefix("docker:")
                    .unwrap_or(&row.domain_label);
                (
                    format!("🐳 {}", name),
                    Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
                )
            } else if row.domain_label.ends_with(".service") {
                let name = row
                    .domain_label
                    .strip_suffix(".service")
                    .unwrap_or(&row.domain_label);
                (format!("⚙ {}", name), Style::default().fg(p.teal))
            } else if row.domain_label.starts_with("user:") {
                let name = row
                    .domain_label
                    .strip_prefix("user:")
                    .unwrap_or(&row.domain_label);
                let clean = name.strip_suffix(".slice").unwrap_or(name);
                (format!("👤 {}", clean), Style::default().fg(p.amber))
            } else {
                ("-".to_string(), Style::default().fg(p.text_muted))
            };

            // Name / Command Line Cell
            let cmd_style = if row.is_group_header {
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD)
            } else if row.has_children {
                Style::default().fg(p.lavender).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.text_primary)
            };

            let name_cell = Line::from(vec![
                Span::styled(
                    row.prefix.as_str(),
                    Style::default()
                        .fg(p.card_border_focus)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(row.cmdline.as_str(), cmd_style),
            ]);

            Row::new(vec![
                Cell::from(row.pid_display.as_str()).style(Style::default().fg(
                    if row.is_group_header {
                        p.cyan
                    } else {
                        p.text_secondary
                    },
                )),
                Cell::from(row.user.as_str()).style(Style::default().fg(p.amber)),
                Cell::from(state_icon).style(Style::default().fg(state_color)),
                Cell::from(Line::from(cpu_spans)),
                Cell::from(Line::from(mem_spans)),
                Cell::from(disk_rw).style(Style::default().fg(
                    if row.disk_read_sec + row.disk_write_sec > 0 {
                        p.mint
                    } else {
                        p.text_muted
                    },
                )),
                Cell::from(domain_text).style(domain_style),
                Cell::from(name_cell),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),  // PID / [Count]
        Constraint::Length(9),  // USER
        Constraint::Length(7),  // STATE
        Constraint::Length(14), // CPU% + Microbar
        Constraint::Length(14), // MEM% + Microbar
        Constraint::Length(11), // DISK R/W
        Constraint::Length(15), // CONTAINER / SERVICE BADGE
        Constraint::Min(25),    // NAME / COMMAND LINE
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(p.selection_bg)
                .fg(p.text_primary)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

/// Bottom Footer Bar with active shortcuts, toast alerts and sampling indicator
fn render_footer(frame: &mut Frame, area: Rect, app: &App, p: &ThemeColors) {
    let mut footer_spans = Vec::new();

    if let Some(toast) = app.active_toast() {
        footer_spans.push(Span::styled(
            format!(" ℹ {} ", toast),
            Style::default()
                .bg(p.blue)
                .fg(p.bg)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.is_searching {
        footer_spans.push(Span::styled(
            " SEARCH MODE: Type query │ [Enter/Esc] Done │ [Ctrl+u] Clear ",
            Style::default()
                .bg(p.amber)
                .fg(p.bg)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let current_rate = app
            .sample_interval_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let rate_color = if current_rate <= 100 {
            p.cyan
        } else if current_rate <= 500 {
            p.amber
        } else {
            p.text_secondary
        };

        footer_spans.extend(vec![
            Span::styled(
                " [q]",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Quit ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[↑↓]",
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Nav ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[Spc]",
                Style::default().fg(p.blue).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Fold ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[n]",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Net ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[u]",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled("User ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[r]",
                Style::default().fg(p.orange).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Nice ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[g]",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Grp ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[t]",
                Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Tree ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[C]",
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Docker ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[s]",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Ports ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[l]",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Logs ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[x]",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Kill ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[T]",
                Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Thm ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[/]",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Find ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[o/c/m/p]",
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Sort ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[^S]",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Snap ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[?]",
                Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Help", Style::default().fg(p.text_muted)),
        ]);

        if area.width >= 120 {
            footer_spans.extend(vec![
                Span::styled(
                    " │ Scoppiato LAB ",
                    Style::default()
                        .fg(Color::Rgb(0, 230, 118))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("scoppiato.it", Style::default().fg(p.text_muted)),
            ]);
        }
        if area.width >= 150 {
            footer_spans.extend(vec![
                Span::styled(
                    " │ [Tab]",
                    Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" View ", Style::default().fg(p.text_muted)),
                Span::styled(
                    format!("[+/-]{}ms", current_rate),
                    Style::default().fg(rate_color).add_modifier(Modifier::BOLD),
                ),
            ]);
        }
    }

    let footer = Paragraph::new(Line::from(footer_spans)).style(Style::default().bg(p.surface));
    frame.render_widget(footer, area);
}

/// Modal: Process Details Inspector Popup
fn render_inspect_modal(
    frame: &mut Frame,
    area: Rect,
    proc: &crate::metrics::ProcessMetric,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(68, 68, area);
    frame.render_widget(Clear, popup_area);

    let title = format!(" [ Process Details: {} (PID: {}) ] ", proc.name, proc.pid);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border_focus))
        .title(Span::styled(
            title,
            Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let parent_str = proc
        .parent_pid
        .map(|p| p.to_string())
        .unwrap_or_else(|| "None".to_string());
    let cpu_color = Theme::usage_color_with_palette(p, proc.cpu_usage);
    let mem_color = Theme::usage_color_with_palette(p, proc.memory_percent);

    let text = vec![
        Line::from(vec![
            Span::styled("Name:             ", Style::default().fg(p.text_muted)),
            Span::styled(
                &proc.name,
                Style::default()
                    .fg(p.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Domain: ", Style::default().fg(p.text_muted)),
            Span::styled(
                proc.domain.label(),
                Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("PID / Parent:     ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!("{} / {}", proc.pid, parent_str),
                Style::default().fg(p.cyan),
            ),
            Span::styled("   User: ", Style::default().fg(p.text_muted)),
            Span::styled(&proc.user, Style::default().fg(p.amber)),
            Span::styled("   State: ", Style::default().fg(p.text_muted)),
            Span::styled(&proc.status, Style::default().fg(p.mint)),
            Span::styled("   Nice: ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!("{}", proc.nice),
                Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("CPU Usage:        ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!("{:.1}% (Surge: +{:.1}%)", proc.cpu_usage, proc.cpu_delta),
                Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Resident Memory:  ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!(
                    "{} ({:.1}%)",
                    Theme::format_bytes(proc.memory_bytes),
                    proc.memory_percent
                ),
                Style::default().fg(mem_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Disk I/O Rate:    ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!(
                    "Read: {}/s  Write: {}/s",
                    Theme::format_bytes(proc.disk_read_bytes_sec),
                    Theme::format_bytes(proc.disk_written_bytes_sec)
                ),
                Style::default().fg(p.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled("Kernel Thread:    ", Style::default().fg(p.text_muted)),
            Span::styled(
                if proc.is_kernel_thread { "Yes" } else { "No" },
                Style::default().fg(p.text_secondary),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Full Command Line:",
            Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            &proc.cmdline,
            Style::default().fg(p.text_primary),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Quick Tools: ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[r] ",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Renice Priority   ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[s] ",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Ports   ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[l] ",
                Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Logs   ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[e] ",
                Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Env   ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[x] ",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Kill", Style::default().fg(p.coral)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Esc / Enter]",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Close Inspector", Style::default().fg(p.text_muted)),
        ]),
    ];

    let p_widget = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(p_widget, inner);
}

/// Modal: Live Open Sockets & Network Ports
fn render_sockets_modal(
    frame: &mut Frame,
    area: Rect,
    pid: u32,
    name: &str,
    sockets: &[crate::sockets::SocketInfo],
    scroll: usize,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(72, 70, area);
    frame.render_widget(Clear, popup_area);

    let title = format!(
        " [ Live Network Sockets & Ports: {} (PID: {}) ] ",
        name, pid
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.mint))
        .title(Span::styled(
            title,
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        format!(
            "{:<6} {:<24} {:<24} {:<12}",
            "PROTO", "LOCAL ADDRESS:PORT", "REMOTE ADDRESS:PORT", "STATE"
        ),
        Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(Span::styled(
        "─".repeat(68),
        Style::default().fg(p.card_border),
    )));

    let visible_sockets: Vec<_> = sockets.iter().skip(scroll).collect();
    for sock in visible_sockets {
        let (state_color, state_text) = if sock.is_listening() {
            (p.mint, "LISTEN 🔊")
        } else if sock.state == "ESTABLISHED" {
            (p.amber, "ESTABLISHED 🔗")
        } else {
            (p.text_muted, sock.state)
        };

        let local = format!("{}:{}", sock.local_addr, sock.local_port);
        let remote = if sock.remote_port > 0 {
            format!("{}:{}", sock.remote_addr, sock.remote_port)
        } else {
            "*:*".to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6} ", sock.proto),
                Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<24} ", local),
                Style::default().fg(p.text_primary),
            ),
            Span::styled(
                format!("{:<24} ", remote),
                Style::default().fg(p.text_secondary),
            ),
            Span::styled(
                format!("{:<12}", state_text),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    if sockets.len() > scroll + inner.height as usize {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "... {} more sockets (scroll with ↑/↓/mouse)",
                sockets.len() - scroll - inner.height as usize
            ),
            Style::default().fg(p.amber),
        )));
    }

    let p_widget = Paragraph::new(lines);
    frame.render_widget(p_widget, inner);
}

/// Modal: Environment Variables Viewer
fn render_environ_modal(
    frame: &mut Frame,
    area: Rect,
    pid: u32,
    name: &str,
    vars: &[(String, String)],
    scroll: usize,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(72, 70, area);
    frame.render_widget(Clear, popup_area);

    let title = format!(" [ Environment Variables: {} (PID: {}) ] ", name, pid);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.lavender))
        .title(Span::styled(
            title,
            Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = Vec::new();
    let visible_vars: Vec<_> = vars.iter().skip(scroll).collect();

    for (k, v) in visible_vars {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}: ", k),
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(v.as_str(), Style::default().fg(p.text_primary)),
        ]));
    }

    let p_widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(p_widget, inner);
}

/// Modal: Live Logs / Journal Tail
fn render_logs_modal(
    frame: &mut Frame,
    area: Rect,
    pid: u32,
    name: &str,
    logs: &[String],
    scroll: usize,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(80, 75, area);
    frame.render_widget(Clear, popup_area);

    let title = format!(" [ Live Logs & Journal Output: {} (PID: {}) ] ", name, pid);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.amber))
        .title(Span::styled(
            title,
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = Vec::new();
    let visible_logs: Vec<_> = logs.iter().skip(scroll).collect();

    for l in visible_logs {
        let style = if l.contains("ERROR") || l.contains("err") || l.contains("fatal") {
            Style::default().fg(p.coral).add_modifier(Modifier::BOLD)
        } else if l.contains("WARN") || l.contains("warn") {
            Style::default().fg(p.amber)
        } else {
            Style::default().fg(p.text_primary)
        };

        lines.push(Line::from(Span::styled(l.as_str(), style)));
    }

    let p_widget = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(p_widget, inner);
}

/// Modal: Process Termination (Kill) Confirmation
fn render_kill_modal(frame: &mut Frame, area: Rect, pid: u32, name: &str, p: &ThemeColors) {
    let popup_area = centered_rect(50, 32, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.coral))
        .title(Span::styled(
            " [ Terminate Process ] ",
            Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Send termination signal to:",
            Style::default().fg(p.text_primary),
        )]),
        Line::from(vec![Span::styled(
            format!("PID {} - \"{}\"", pid, name),
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [t] ",
                Style::default().fg(p.orange).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "SIGTERM (15) - Graceful Termination",
                Style::default().fg(p.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  [k] ",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "SIGKILL (9)  - Force Kill Immediately",
                Style::default().fg(p.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  [Esc] ",
                Style::default()
                    .fg(p.text_muted)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Cancel and Return", Style::default().fg(p.text_muted)),
        ]),
    ];

    frame.render_widget(Paragraph::new(text), inner);
}

/// Modal: Renice CPU scheduling priority popup
fn render_renice_modal(
    frame: &mut Frame,
    area: Rect,
    pid: u32,
    name: &str,
    current_nice: i32,
    target_nice: i32,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(58, 44, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.amber))
        .title(Span::styled(
            " [ Adjust Process Priority (Renice) ] ",
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let priority_desc = if target_nice < -10 {
        "Very High Priority"
    } else if target_nice < 0 {
        "High Priority"
    } else if target_nice == 0 {
        "Normal Priority (Default)"
    } else if target_nice <= 10 {
        "Low Priority (Background)"
    } else {
        "Very Low Priority (Idle)"
    };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Process: ", Style::default().fg(p.text_muted)),
            Span::styled(
                format!("PID {} - \"{}\"", pid, name),
                Style::default()
                    .fg(p.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  (Current Nice: {})", current_nice),
                Style::default().fg(p.lavender),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Target Nice:  ", Style::default().fg(p.text_primary)),
            Span::styled(
                format!(" [ {:>3} ] ", target_nice),
                Style::default()
                    .bg(p.amber)
                    .fg(p.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", priority_desc),
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "             (-20 Highest Priority .. 19 Lowest Priority)",
            Style::default().fg(p.text_muted),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "Quick Presets:",
            Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "  [1] ",
                Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Nice -10 (High Priority)   ",
                Style::default().fg(p.text_primary),
            ),
            Span::styled(
                "[2] ",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Nice  0 (Normal / Default)",
                Style::default().fg(p.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  [3] ",
                Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Nice  10 (Low / Background) ",
                Style::default().fg(p.text_primary),
            ),
            Span::styled(
                "[4] ",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Nice 19 (Idle Only)", Style::default().fg(p.text_primary)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(p.text_muted)),
            Span::styled(
                "[↑/↓] ",
                Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Adjust ±1  ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[Enter] ",
                Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Apply Renice  ", Style::default().fg(p.text_primary)),
            Span::styled(
                "[Esc] ",
                Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Cancel", Style::default().fg(p.text_muted)),
        ]),
    ];

    frame.render_widget(Paragraph::new(text), inner);
}

/// Modal: Filter processes by User
fn render_user_filter_modal(
    frame: &mut Frame,
    area: Rect,
    selected_idx: usize,
    users: &[(String, usize)],
    current_filter: Option<&str>,
    p: &ThemeColors,
) {
    let popup_area = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.mint))
        .title(Span::styled(
            " [ Filter Processes by User ] ",
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = Vec::new();
    lines.push(Line::from("Select a user to display only their processes:"));
    lines.push(Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(p.card_border),
    )));

    // Option 0: ALL Users
    let is_all_selected = selected_idx == 0;
    let is_all_active = current_filter.is_none();
    let all_style = if is_all_selected {
        Style::default()
            .bg(p.selection_bg)
            .fg(p.cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.text_primary)
    };
    let marker = if is_all_active { " ● " } else { " ○ " };
    lines.push(Line::from(vec![
        Span::styled(
            if is_all_selected { "▶ " } else { "  " },
            Style::default().fg(p.cyan),
        ),
        Span::styled(
            marker,
            Style::default().fg(if is_all_active { p.mint } else { p.text_muted }),
        ),
        Span::styled("ALL USERS (Show all processes)", all_style),
    ]));

    for (idx, (u, count)) in users.iter().enumerate() {
        let list_idx = idx + 1;
        let is_selected = selected_idx == list_idx;
        let is_active = current_filter == Some(u.as_str());

        let row_style = if is_selected {
            Style::default()
                .bg(p.selection_bg)
                .fg(p.amber)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.text_primary)
        };

        let active_dot = if is_active { " ● " } else { " ○ " };
        lines.push(Line::from(vec![
            Span::styled(
                if is_selected { "▶ " } else { "  " },
                Style::default().fg(p.cyan),
            ),
            Span::styled(
                active_dot,
                Style::default().fg(if is_active { p.mint } else { p.text_muted }),
            ),
            Span::styled(format!("{:<18}", u), row_style),
            Span::styled(
                format!(" ({} tasks)", count),
                Style::default().fg(p.text_muted),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "[↑/↓] ",
            Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Select User  ", Style::default().fg(p.text_muted)),
        Span::styled(
            "[Enter] ",
            Style::default().fg(p.mint).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Apply  ", Style::default().fg(p.text_muted)),
        Span::styled(
            "[c] ",
            Style::default().fg(p.amber).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Clear  ", Style::default().fg(p.text_muted)),
        Span::styled(
            "[Esc] ",
            Style::default().fg(p.coral).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Close", Style::default().fg(p.text_muted)),
    ]));

    let p_widget = Paragraph::new(lines);
    frame.render_widget(p_widget, inner);
}

/// Modal: Comprehensive Keyboard Help Overlay
fn render_help_modal(frame: &mut Frame, area: Rect, p: &ThemeColors) {
    let popup_area = centered_rect(72, 86, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.card_border_focus))
        .title(Span::styled(
            " [ sltop Help & Keybindings ] ",
            Style::default().fg(p.lavender).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(p.surface));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lab_green = Color::Rgb(0, 230, 118);
    let text = vec![
        Line::from(vec![
            Span::styled("⚡ Scoppiato LAB ", Style::default().bg(lab_green).fg(Color::Rgb(10, 16, 20)).add_modifier(Modifier::BOLD)),
            Span::styled(" Web: ", Style::default().fg(p.text_muted)),
            Span::styled("https://scoppiato.it", Style::default().fg(p.cyan)),
            Span::styled(" │ Discord: ", Style::default().fg(p.text_muted)),
            Span::styled("https://discord.scoppiato.it", Style::default().fg(lab_green)),
        ]),
        Line::from(Span::styled("─".repeat((inner.width as usize).saturating_sub(2)), Style::default().fg(p.card_border))),
        Line::from(Span::styled("Process Organization & Views:", Style::default().fg(p.mint).add_modifier(Modifier::BOLD))),
        Line::from("  C            Toggle Docker / Pod Container View (groups processes by container)"),
        Line::from("  g            Toggle Process Grouping (collapses multiple threads into 1 row)"),
        Line::from("  t            Toggle Process Tree View (hierarchical parent-child tree)"),
        Line::from("  Space        Fold / Unfold selected group, container or tree branch"),
        Line::from(""),
        Line::from(Span::styled("Process Controls & Priority (Renice):", Style::default().fg(p.amber).add_modifier(Modifier::BOLD))),
        Line::from("  r            Open interactive Renice dialog to adjust process nice priority"),
        Line::from("  [ / ]        Quick Renice: decrease/increase priority of selected process"),
        Line::from("  u            Filter processes by user (root, www-data, etc.)"),
        Line::from("  x, Delete    Kill process modal (SIGTERM / SIGKILL)"),
        Line::from(""),
        Line::from(Span::styled("Diagnostics & System Info:", Style::default().fg(p.cyan).add_modifier(Modifier::BOLD))),
        Line::from("  s            Inspect Open Sockets & Network Ports live for selected process"),
        Line::from("  l            View Live Logs / Journal Tail (Docker logs or systemd journal)"),
        Line::from("  e            View Environment Variables of selected process"),
        Line::from("  S            Sort by CPU spike delta"),
        Line::from("  n            Cycle Network Interface (ALL / eth0 / wg0 / etc.)"),
        Line::from("  PSI          Real-time Linux Pressure Stall Information (CPU/MEM/IO Stalls)"),
        Line::from(""),
        Line::from(Span::styled("Persistence, Snapshots & Themes:", Style::default().fg(p.lavender).add_modifier(Modifier::BOLD))),
        Line::from("  Ctrl+S, F2   Export full system snapshot to JSON file"),
        Line::from("  Ctrl+P       Save current settings to ~/.config/sltop/sltop.toml"),
        Line::from("  T            Cycle color themes (Tokyo Slate / Catppuccin / Gruvbox / Nord / Monokai)"),
        Line::from("  o            Toggle Sort Order (Ascending / Descending)"),
        Line::from("  c, m, p, d   Sort by CPU%, Memory%, PID, Disk I/O"),
        Line::from("  /            Interactive Search filter (name, pid, user, cmdline, docker container)"),
        Line::from("  Enter        Inspect process details"),
        Line::from("  Tab          Cycle layout views (Balanced / Proc Focus / Graphs Focus)"),
        Line::from("  +, = / -     Increase / decrease sampling rate (100ms - 5000ms)"),
        Line::from("  q, Esc       Quit sltop"),
        Line::from(""),
        Line::from(Span::styled("[Press Esc or ? to Close]", Style::default().fg(p.lavender).add_modifier(Modifier::BOLD))),
    ];

    frame.render_widget(Paragraph::new(text), inner);
}

/// Helper function to calculate centered rect for popups
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
