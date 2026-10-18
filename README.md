<p align="center">
  <img src="assets/logo.png" alt="Scoppiato LAB" width="260"/>
</p>

<h1 align="center">sltop</h1>

<p align="center">
  <strong>Lightweight terminal system monitor for Linux written in Rust</strong><br>
  Crafted & Maintained by <a href="https://scoppiato.it"><strong>Scoppiato LAB</strong></a>
</p>

<p align="center">
  <a href="https://scoppiato.it"><img src="https://img.shields.io/badge/Website-scoppiato.it-00E676?style=for-the-badge&logo=safari&logoColor=white" alt="Website"></a>
  <a href="https://discord.scoppiato.it"><img src="https://img.shields.io/badge/Discord-Join%20Community-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Discord"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/Rust-1.74%2B-orange.svg?style=for-the-badge&logo=rust" alt="Rust 1.74+">
</p>

---

## Overview

**sltop** is an ultra-lightweight, real-time terminal system monitor designed for Linux servers and development environments. Built from the ground up in Rust, it delivers live hardware telemetry, high-resolution Braille graphs, container cgroup resolution, and process management with near-zero CPU and memory footprint.

```text
┌───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ sltop  Scoppiato LAB  [Tokyo Slate] Host: srv-01 (Debian) │ Up: 3d 11h 24m   Load: 1.12 1.05 1.14 │ ⚡ 100ms│
└───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ [ CPU HISTORY - Usage Waveform ] ───────────┐┌ [ NETWORK (eth0) - ↓ Rx / ↑ Tx ] ──────────────────────────┐
│100% ┤                                       ││↓ 1.4 MiB/s (Pk 5.5M) │ ↑ 312 KiB/s (Pk 1.1M) │ Tot: 4.5G/4.1G│
│     │                                       ││ 1.4M ┤                                             ⡇   ⢰⡄  │
│ 50% ┤                             ⢀⣀⡠⠤⠒⠊⠉⠉⠁││   0B ┴───────────────────────────────────────────⣠⣿⣿⣿⣦⣿⣿⣿⣷⡀│
│  0% ┴─────────────────────────────⣿⣿⣿⣿⣶⣤⣀⣀⣀│└───────────────────────────────────────────────────────────┘
└─────────────────────────────────────────────┘┌ [ MEMORY & STORAGE ] ─────────────────────────────────────┐
┌ [ CPU - 2 Cores ] ──────────────────────────┐│RAM  [████████░░░░]  58.6%  2.2G / 3.8G (Avail: 1.6G)       │
│Total: [████████████]  48.2% │ PSI CPU: 0.8% ││SWAP [████████████] 100.0%  512.0M / 512.0M                 │
│cpu0   [████░░░░░░]  38.1% 2294MHz           ││─ Filesystems ───────────────────────────────────────      │
│cpu1   [██████░░░░]  58.3% 2294MHz           ││/          [██████░░░░]  60.4% 29.6G/49.1G (ext4)           │
└─────────────────────────────────────────────┘└───────────────────────────────────────────────────────────┘
┌ [ PROCESSES (644 Tasks, 5 Active) ] Shown: 32 │ View: [Container (C)] │ Sort: [CPU% ▼] ───────────────────┐
│  PID      USER      STATE   CPU% (Load) ▼  MEM% (Usage)   DISK R/W    CONTAINER/SVC   NAME / COMMAND LINE │
│▶ [48]     app       ● Run    34.8% [██░░░]  30.7% [██░░░] 0B          🐳 web_app      ▶ docker:web_app    │
│  [229]    root      ○ Slp    29.7% [█░░░░]  42.5% [██░░░] 0B          ⚙ worker        ▶ worker.service    │
│  [30]     root      ○ Slp    14.8% [█░░░░]  33.6% [██░░░] 0B          ⚙ docker        ▶ docker.service    │
│  [4]      redis     ○ Slp     3.6% [░░░░░]   0.7% [░░░░░] 0B          🐳 redis        ▶ docker:redis      │
└───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 [q] Quit [↑↓] Nav [Spc] Fold [g] Grp [t] Tree [C] Docker [s] Ports [l] Logs [e] Env [T] Thm [/] Find [Ret] Info
```

---

## Features

- **Braille Waveform Graphs**: 2×4 Braille sub-character rendering for historical CPU dynamics and network bandwidth (Rx/Tx) grounded cleanly on continuous 0% / 0B baseline axes.
- **Container & Service Awareness**: Resolves Linux cgroups directly to identify Docker container names and systemd service units for every process.
- **Linux PSI Telemetry**: Real-time Pressure Stall Information tracking resource starvation across CPU, memory, and I/O subsystems.
- **Kernel Socket & Port Inspection**: Inspects active TCP socket tables and listening ports without spawning external utilities.
- **Interactive Process Controls**: Send signals (`SIGTERM`/`SIGKILL`), adjust scheduling priorities (`renice`), and fold/unfold process groups or tree hierarchies.
- **Embedded Log Streamer**: Live journal tailing (`journalctl -u`) and Docker container log streaming (`docker logs`) directly from the selected process.
- **Minimal Resource Footprint**: Optimized with Link-Time Optimization (`LTO = fat`) and binary stripping into a compact standalone binary (~1.1 MB).
- **Multiple Color Themes**: Tokyo Slate, Catppuccin Mocha, Gruvbox Dark, Nordic Frost, and Monokai Minimal.

---

## Installation

### From Source

Requires a working [Rust](https://www.rust-lang.org/) toolchain (1.74+):

```bash
git clone https://github.com/scoppiatolab/sltop.git
cd sltop
cargo build --release
sudo install -m 755 target/release/sltop /usr/local/bin/
```

Run `sltop`:
```bash
sltop
```

---

## Keybindings

### Navigation & Views

| Key | Description |
| :--- | :--- |
| `↑` / `k`, `↓` / `j` | Move selection up / down |
| `PgUp` / `PgDn` | Jump 10 items |
| `Home` / `End` | Jump to beginning / end |
| `C` | Toggle Container View (group by Docker container / systemd service) |
| `g` | Toggle Process Grouping |
| `t` | Toggle Tree View (parent-child hierarchy) |
| `Space` | Expand / collapse group or tree branch |
| `u` | Filter processes by user |
| `/` | Search filter (name, PID, user, command line, container) |
| `K` | Toggle kernel thread visibility |
| `Tab` | Switch view layout (Balanced / Process Focus / Graph Focus) |

### Process Management & Diagnostics

| Key | Description |
| :--- | :--- |
| `Enter` | Show process details inspector |
| `s` | View open sockets and listening ports |
| `l` | View live process / container logs |
| `e` | View environment variables |
| `r` | Adjust process scheduling priority (`renice`) |
| `[` / `]` | Quick decrease / increase nice priority |
| `x` / `Delete` | Kill process dialog (`t` = SIGTERM, `k` = SIGKILL) |
| `n` | Cycle network interface (`ALL`, `eth0`, `docker0`, etc.) |
| `T` | Cycle color themes |
| `+` / `-` | Adjust sampling interval (100ms - 5000ms) |
| `Ctrl+S` / `F2` | Export complete telemetry snapshot to JSON file |
| `Ctrl+P` | Save current preferences to `~/.config/sltop/sltop.toml` |
| `?` / `h` | Open help & keybindings overlay |
| `q` / `Esc` | Close modal / Exit |

---

## Headless & Automation Options

`sltop` can run non-interactively for server audits, metrics collection, or scripts:

```bash
# Print a plain text snapshot to stdout and exit
sltop --snapshot

# Output complete system telemetry in JSON format
sltop --json

# Override sampling interval
sltop --interval 250
```

---

## Configuration

Settings are saved automatically via `Ctrl+P` or can be edited in `~/.config/sltop/sltop.toml`:

```toml
sample_interval_ms = 100
theme = "tokyo_slate"
sort_by = "cpu"
sort_order = "desc"
view_mode = "balanced"
process_view_type = "container"
hide_kernel_threads = true
```

---

## Community & About Scoppiato LAB

**sltop** is an open-source project proudly developed and maintained by **Scoppiato LAB**.

- 🌐 **Official Website**: [https://scoppiato.it](https://scoppiato.it)
- 💬 **Discord Community**: [https://discord.scoppiato.it](https://discord.scoppiato.it)
- 📦 **GitHub**: [https://github.com/scoppiatolab/sltop](https://github.com/scoppiatolab/sltop)

Join our Discord server for support, feature discussions, and updates on our open-source tools!

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2026 **Scoppiato LAB** ([https://scoppiato.it](https://scoppiato.it)).
