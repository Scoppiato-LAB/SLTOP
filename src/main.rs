mod app;
mod braille;
mod config;
mod containers;
mod export;
mod gpu;
mod metrics;
mod psi;
mod sockets;
mod theme;
mod ui;

use app::{ActiveModal, App, ProcessSortBy};
use config::AppConfig;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
        KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use metrics::MetricsCollector;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// RAII Guard to ensure the terminal is restored unconditionally on exit or panic
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        );
    }
}

/// Installs a panic hook that safely resets the terminal before printing the panic backtrace
fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        );
        original_hook(panic_info);
    }));
}

fn main() -> io::Result<()> {
    // Process CLI arguments
    let args: Vec<String> = std::env::args().collect();
    let config = AppConfig::load();
    let mut cli_interval = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" | "-j" => {
                let mut collector = MetricsCollector::new();
                thread::sleep(Duration::from_millis(200));
                let metrics = collector.sample();
                use std::io::Write;
                let _ = writeln!(io::stdout(), "{}", export::dump_json(&metrics));
                return Ok(());
            }
            "--snapshot" | "-s" => {
                let mut collector = MetricsCollector::new();
                thread::sleep(Duration::from_millis(200));
                let metrics = collector.sample();
                use std::io::Write;
                let _ = writeln!(io::stdout(), "{}", export::dump_snapshot_text(&metrics));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("sltop 0.2.0 - Real-time terminal system monitor");
                println!("Developed by Scoppiato LAB (https://scoppiato.it)");
                println!("Community Discord: https://discord.scoppiato.it");
                println!();
                println!("USAGE:");
                println!("    sltop [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!("    -j, --json          Dump complete system telemetry in JSON format and exit");
                println!("    -s, --snapshot      Dump clean system overview snapshot in text format and exit");
                println!("    -i, --interval <MS> Sampling interval in milliseconds (default: 100ms or config)");
                println!("    -h, --help          Print help information");
                println!("    -v, --version       Print version information");
                return Ok(());
            }
            "--version" | "-v" => {
                println!("sltop 0.2.0 (Scoppiato LAB - https://scoppiato.it)");
                return Ok(());
            }
            "-i" | "--interval" => {
                if i + 1 < args.len() {
                    if let Ok(ms) = args[i + 1].parse::<u64>() {
                        cli_interval = Some(ms.clamp(100, 5000));
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Terminal restore hook
    setup_panic_hook();

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;

    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Sampling interval
    let initial_interval = cli_interval.unwrap_or(config.sample_interval_ms).clamp(100, 5000);
    let sample_interval_ms = Arc::new(AtomicU64::new(initial_interval));
    let (tx_metrics, rx_metrics) = crossbeam_channel::unbounded();
    let is_running = Arc::new(AtomicBool::new(true));

    // Spawn telemetry sampler thread
    let sampler_handle = MetricsCollector::spawn(
        Arc::clone(&sample_interval_ms),
        tx_metrics,
        Arc::clone(&is_running),
    );

    // Input thread
    let (tx_events, rx_events) = crossbeam_channel::unbounded();
    let is_running_input = Arc::clone(&is_running);

    let input_handle = thread::Builder::new()
        .name("sltop-input".into())
        .spawn(move || {
            while is_running_input.load(Ordering::Relaxed) {
                if let Ok(true) = event::poll(Duration::from_millis(50)) {
                    if let Ok(evt) = event::read() {
                        let _ = tx_events.send(evt);
                    }
                }
            }
        })
        .expect("Failed to spawn input thread");

    // Initialize application state
    let mut app = App::new(sample_interval_ms, config);
    let mut needs_draw = true;

    // Event loop
    while app.is_running {
        if needs_draw {
            terminal.draw(|f| ui::render(f, &mut app))?;
            needs_draw = false;
        }

        crossbeam_channel::select! {
            recv(rx_metrics) -> msg => {
                if let Ok(snap) = msg {
                    app.update_metrics(snap);
                    while let Ok(snap) = rx_metrics.try_recv() {
                        app.update_metrics(snap);
                    }
                    needs_draw = true;
                }
            }

            recv(rx_events) -> msg => {
                if let Ok(evt) = msg {
                    match evt {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            handle_key_event(&mut app, key.code, key.modifiers);
                            needs_draw = true;
                        }
                        Event::Mouse(mouse) => {
                            match mouse.kind {
                                MouseEventKind::ScrollDown => {
                                    app.handle_mouse_scroll(false);
                                    needs_draw = true;
                                }
                                MouseEventKind::ScrollUp => {
                                    app.handle_mouse_scroll(true);
                                    needs_draw = true;
                                }
                                MouseEventKind::Down(MouseButton::Left) => {
                                    app.handle_mouse_click(mouse.column, mouse.row);
                                    needs_draw = true;
                                }
                                _ => {}
                            }
                        }
                        Event::Resize(_, _) => {
                            needs_draw = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Gracefully stop background threads
    is_running.store(false, Ordering::Relaxed);
    let _ = sampler_handle.join();
    let _ = input_handle.join();

    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.quit();
        return;
    }

    // Modal Interaction Mode
    if app.active_modal != ActiveModal::None {
        match app.active_modal.clone() {
            ActiveModal::Help => {
                match code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                        app.close_modal();
                    }
                    _ => {}
                }
            }
            ActiveModal::ProcessInspect(_) => {
                match code {
                    KeyCode::Char('r') => app.open_renice_modal(),
                    KeyCode::Char('s') => app.open_sockets_modal(),
                    KeyCode::Char('l') => app.open_logs_modal(),
                    KeyCode::Char('e') => app.open_environ_modal(),
                    KeyCode::Char('x') => app.open_kill_modal(),
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                        app.close_modal();
                    }
                    _ => {}
                }
            }
            ActiveModal::ProcessRenice { .. } => {
                match code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('-') => app.adjust_renice_target(-1),
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('+') | KeyCode::Char('=') => app.adjust_renice_target(1),
                    KeyCode::Char('1') => app.set_renice_preset(-10),
                    KeyCode::Char('2') => app.set_renice_preset(0),
                    KeyCode::Char('3') => app.set_renice_preset(10),
                    KeyCode::Char('4') => app.set_renice_preset(19),
                    KeyCode::Enter => app.apply_renice(),
                    KeyCode::Esc | KeyCode::Char('q') => app.close_modal(),
                    _ => {}
                }
            }
            ActiveModal::UserFilter { selected_idx, ref users } => {
                let users_len = users.len();
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        let next_idx = if selected_idx == 0 {
                            users_len
                        } else {
                            selected_idx - 1
                        };
                        if let ActiveModal::UserFilter { ref mut selected_idx, .. } = app.active_modal {
                            *selected_idx = next_idx;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let next_idx = if selected_idx >= users_len {
                            0
                        } else {
                            selected_idx + 1
                        };
                        if let ActiveModal::UserFilter { ref mut selected_idx, .. } = app.active_modal {
                            *selected_idx = next_idx;
                        }
                    }
                    KeyCode::Enter => {
                        let chosen = if selected_idx == 0 {
                            None
                        } else if let Some((u, _)) = users.get(selected_idx - 1) {
                            Some(u.clone())
                        } else {
                            None
                        };
                        app.select_user_filter(chosen);
                    }
                    KeyCode::Char('c') => {
                        app.select_user_filter(None);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => app.close_modal(),
                    _ => {}
                }
            }
            ActiveModal::ProcessSockets { .. }
            | ActiveModal::ProcessEnviron { .. }
            | ActiveModal::ProcessLogs { .. } => {
                match code {
                    KeyCode::Up | KeyCode::Char('k') => app.modal_scroll_up(1),
                    KeyCode::Down | KeyCode::Char('j') => app.modal_scroll_down(1),
                    KeyCode::PageUp => app.modal_scroll_up(10),
                    KeyCode::PageDown => app.modal_scroll_down(10),
                    KeyCode::Esc | KeyCode::Char('q') => app.close_modal(),
                    _ => {}
                }
            }
            ActiveModal::ProcessKill { .. } => {
                match code {
                    KeyCode::Char('t') => app.kill_process_with_signal(15), // SIGTERM
                    KeyCode::Char('k') => app.kill_process_with_signal(9),  // SIGKILL
                    KeyCode::Esc | KeyCode::Char('q') => app.close_modal(),
                    _ => {}
                }
            }
            ActiveModal::None => {}
        }
        return;
    }

    // Search Input Mode
    if app.is_searching {
        match code {
            KeyCode::Esc | KeyCode::Enter => {
                app.is_searching = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.recompute_rows();
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.search_query.clear();
                app.recompute_rows();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.recompute_rows();
            }
            _ => {}
        }
        return;
    }

    // Standard Navigation & Control Mode
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.recompute_rows();
            } else {
                app.quit();
            }
        }

        KeyCode::Char('/') => {
            app.is_searching = true;
        }

        KeyCode::Down | KeyCode::Char('j') => app.next_process(),
        KeyCode::Up | KeyCode::Char('k') => app.previous_process(),
        KeyCode::PageDown => app.page_down(10),
        KeyCode::PageUp => app.page_up(10),
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => app.page_down(10),
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => app.page_up(10),
        KeyCode::Home => app.scroll_to_top(),
        KeyCode::End | KeyCode::Char('G') => app.scroll_to_bottom(),

        // Process Organization & Views
        KeyCode::Char(' ') => app.toggle_expand_selected(),
        KeyCode::Char('g') => app.toggle_grouping(),
        KeyCode::Char('t') => app.toggle_tree_view(),
        KeyCode::Char('C') => app.toggle_container_view(),

        // Live Diagnostic Tools
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => app.save_snapshot(),
        KeyCode::F(2) => app.save_snapshot(),
        KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => app.save_config_file(),

        KeyCode::Char('s') => app.open_sockets_modal(),
        KeyCode::Char('l') => app.open_logs_modal(),
        KeyCode::Char('e') => app.open_environ_modal(),

        // Process Priority (Renice) & Filter
        KeyCode::Char('r') => app.open_renice_modal(),
        KeyCode::Char('[') => app.quick_renice(-1),
        KeyCode::Char(']') => app.quick_renice(1),
        KeyCode::Char('u') => app.open_user_filter_modal(),

        // Network Interface Selector
        KeyCode::Char('n') => app.cycle_network_interface(),

        // Theme cycling
        KeyCode::Char('T') => app.cycle_theme(),

        KeyCode::Enter => app.open_inspect_modal(),
        KeyCode::Char('x') | KeyCode::Delete => app.open_kill_modal(),

        // Sorting
        KeyCode::Char('c') => app.set_sort(ProcessSortBy::Cpu),
        KeyCode::Char('m') => app.set_sort(ProcessSortBy::Memory),
        KeyCode::Char('p') => app.set_sort(ProcessSortBy::Pid),
        KeyCode::Char('N') => app.set_sort(ProcessSortBy::Name),
        KeyCode::Char('d') => app.set_sort(ProcessSortBy::Disk),
        KeyCode::Char('S') => app.set_sort(ProcessSortBy::Surge),
        KeyCode::Char('o') => app.toggle_sort_order(),

        KeyCode::Char('K') => app.toggle_kernel_threads(),
        KeyCode::Tab => app.toggle_view_mode(),

        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_sample_speed(),
        KeyCode::Char('-') => app.decrease_sample_speed(),

        KeyCode::Char('?') | KeyCode::Char('h') => app.toggle_help(),

        _ => {}
    }
}
