use ratatui::style::{Color, Style};
use ratatui::text::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    TokyoSlate,
    CatppuccinMocha,
    GruvboxDark,
    NordicFrost,
    MonokaiMinimal,
}

impl ThemeMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TokyoSlate => "Tokyo Slate",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::NordicFrost => "Nordic Frost",
            Self::MonokaiMinimal => "Monokai Minimal",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::TokyoSlate => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::GruvboxDark,
            Self::GruvboxDark => Self::NordicFrost,
            Self::NordicFrost => Self::MonokaiMinimal,
            Self::MonokaiMinimal => Self::TokyoSlate,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub bg: Color,
    pub surface: Color,
    pub card_border: Color,
    pub card_border_focus: Color,
    pub selection_bg: Color,
    pub bar_empty: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub cyan: Color,
    pub blue: Color,
    pub lavender: Color,
    pub plum: Color,
    pub teal: Color,
    pub mint: Color,
    pub sky: Color,
    pub amber: Color,
    pub orange: Color,
    pub coral: Color,
}

pub struct Theme;

impl Theme {
    pub fn palette(mode: ThemeMode) -> ThemeColors {
        match mode {
            ThemeMode::TokyoSlate => ThemeColors {
                bg: Color::Rgb(13, 16, 23),
                surface: Color::Rgb(20, 24, 34),
                card_border: Color::Rgb(50, 62, 85),
                card_border_focus: Color::Rgb(90, 145, 230),
                selection_bg: Color::Rgb(32, 46, 74),
                bar_empty: Color::Rgb(40, 48, 66),
                text_primary: Color::Rgb(240, 244, 252),
                text_secondary: Color::Rgb(160, 175, 205),
                text_muted: Color::Rgb(105, 120, 150),
                cyan: Color::Rgb(70, 195, 220),
                blue: Color::Rgb(95, 165, 240),
                lavender: Color::Rgb(165, 140, 235),
                plum: Color::Rgb(175, 120, 160),
                teal: Color::Rgb(75, 185, 195),
                mint: Color::Rgb(80, 215, 150),
                sky: Color::Rgb(100, 180, 245),
                amber: Color::Rgb(235, 180, 70),
                orange: Color::Rgb(240, 125, 65),
                coral: Color::Rgb(240, 75, 95),
            },
            ThemeMode::CatppuccinMocha => ThemeColors {
                bg: Color::Rgb(30, 30, 46),          // Mocha Base
                surface: Color::Rgb(24, 24, 37),     // Mantle
                card_border: Color::Rgb(69, 71, 90), // Surface1
                card_border_focus: Color::Rgb(203, 166, 247), // Mauve
                selection_bg: Color::Rgb(49, 50, 68), // Surface0
                bar_empty: Color::Rgb(49, 50, 68),
                text_primary: Color::Rgb(205, 214, 244), // Text
                text_secondary: Color::Rgb(186, 194, 222), // Subtext1
                text_muted: Color::Rgb(108, 112, 134),  // Overlay0
                cyan: Color::Rgb(137, 220, 235),      // Sky
                blue: Color::Rgb(137, 180, 250),      // Blue
                lavender: Color::Rgb(203, 166, 247),  // Mauve
                plum: Color::Rgb(245, 194, 231),      // Pink
                teal: Color::Rgb(148, 226, 213),      // Teal
                mint: Color::Rgb(166, 227, 161),      // Green
                sky: Color::Rgb(116, 199, 236),       // Sapphire
                amber: Color::Rgb(249, 226, 175),     // Yellow
                orange: Color::Rgb(250, 179, 135),    // Peach
                coral: Color::Rgb(243, 139, 168),     // Red
            },
            ThemeMode::GruvboxDark => ThemeColors {
                bg: Color::Rgb(40, 40, 40),          // dark0
                surface: Color::Rgb(29, 32, 33),     // dark0_hard
                card_border: Color::Rgb(80, 73, 69), // dark3
                card_border_focus: Color::Rgb(250, 189, 47), // yellow
                selection_bg: Color::Rgb(60, 56, 54), // dark1
                bar_empty: Color::Rgb(50, 48, 47),
                text_primary: Color::Rgb(235, 219, 178), // light1
                text_secondary: Color::Rgb(213, 196, 161), // light2
                text_muted: Color::Rgb(146, 131, 116),  // gray
                cyan: Color::Rgb(142, 192, 124),      // aqua
                blue: Color::Rgb(131, 165, 152),      // blue
                lavender: Color::Rgb(211, 134, 155),  // purple
                plum: Color::Rgb(177, 98, 134),
                teal: Color::Rgb(104, 157, 106),
                mint: Color::Rgb(184, 187, 38),       // green
                sky: Color::Rgb(131, 165, 152),
                amber: Color::Rgb(250, 189, 47),      // yellow
                orange: Color::Rgb(254, 128, 25),     // orange
                coral: Color::Rgb(251, 73, 52),       // red
            },
            ThemeMode::NordicFrost => ThemeColors {
                bg: Color::Rgb(46, 52, 64),          // nord0
                surface: Color::Rgb(36, 41, 51),
                card_border: Color::Rgb(76, 86, 106), // nord3
                card_border_focus: Color::Rgb(136, 192, 208), // nord8 (frost)
                selection_bg: Color::Rgb(59, 66, 82), // nord1
                bar_empty: Color::Rgb(59, 66, 82),
                text_primary: Color::Rgb(236, 239, 244), // nord6
                text_secondary: Color::Rgb(216, 222, 233), // nord4
                text_muted: Color::Rgb(140, 155, 180),
                cyan: Color::Rgb(136, 192, 208),      // nord8
                blue: Color::Rgb(129, 161, 193),      // nord9
                lavender: Color::Rgb(180, 142, 173),  // nord15
                plum: Color::Rgb(165, 130, 160),
                teal: Color::Rgb(143, 188, 187),      // nord7
                mint: Color::Rgb(163, 190, 140),      // nord14
                sky: Color::Rgb(136, 192, 208),
                amber: Color::Rgb(235, 203, 139),     // nord13
                orange: Color::Rgb(208, 135, 112),    // nord12
                coral: Color::Rgb(191, 97, 106),      // nord11
            },
            ThemeMode::MonokaiMinimal => ThemeColors {
                bg: Color::Rgb(18, 18, 18),
                surface: Color::Rgb(28, 28, 28),
                card_border: Color::Rgb(58, 58, 58),
                card_border_focus: Color::Rgb(102, 217, 239),
                selection_bg: Color::Rgb(45, 45, 45),
                bar_empty: Color::Rgb(38, 38, 38),
                text_primary: Color::Rgb(248, 248, 242),
                text_secondary: Color::Rgb(190, 190, 180),
                text_muted: Color::Rgb(120, 120, 115),
                cyan: Color::Rgb(102, 217, 239),
                blue: Color::Rgb(102, 217, 239),
                lavender: Color::Rgb(174, 129, 255),
                plum: Color::Rgb(249, 38, 114),
                teal: Color::Rgb(102, 217, 239),
                mint: Color::Rgb(166, 226, 46),
                sky: Color::Rgb(102, 217, 239),
                amber: Color::Rgb(230, 219, 116),
                orange: Color::Rgb(253, 151, 31),
                coral: Color::Rgb(249, 38, 114),
            },
        }
    }

    /// Dynamic usage color based on active palette
    pub fn usage_color_with_palette(p: &ThemeColors, pct: f32) -> Color {
        if pct < 55.0 {
            p.cyan
        } else if pct < 75.0 {
            p.amber
        } else if pct < 90.0 {
            p.orange
        } else {
            p.coral
        }
    }

    /// Vertical gradient color based on vertical height ratio (1.0 = peak/top, 0.0 = bottom)
    pub fn vertical_gradient_color(p: &ThemeColors, height_ratio: f32) -> Color {
        if height_ratio >= 0.82 {
            p.coral
        } else if height_ratio >= 0.60 {
            p.orange
        } else if height_ratio >= 0.38 {
            p.amber
        } else if height_ratio >= 0.18 {
            p.teal
        } else {
            p.cyan
        }
    }

    /// Default Tokyo Slate usage color fallback
    #[allow(dead_code)]
    pub fn usage_color(pct: f32) -> Color {
        let p = Self::palette(ThemeMode::TokyoSlate);
        Self::usage_color_with_palette(&p, pct)
    }

    /// Progress bar with fractional blocks
    pub fn render_bar_spans(p: &ThemeColors, ratio: f32, width: usize, custom_color: Option<Color>) -> Vec<Span<'static>> {
        let clamped = ratio.clamp(0.0, 1.0);
        let total_chars = (clamped * width as f32).max(0.0);
        let full_blocks = (total_chars.floor() as usize).min(width);
        let remainder = total_chars - full_blocks as f32;

        let frac_char = if remainder >= 0.875 {
            '▉'
        } else if remainder >= 0.75 {
            '▊'
        } else if remainder >= 0.625 {
            '▋'
        } else if remainder >= 0.5 {
            '▌'
        } else if remainder >= 0.375 {
            '▍'
        } else if remainder >= 0.25 {
            '▎'
        } else if remainder >= 0.125 {
            '▏'
        } else {
            ' '
        };

        let has_frac = frac_char != ' ' && full_blocks < width;
        let empty = width.saturating_sub(full_blocks + if has_frac { 1 } else { 0 });
        let color = custom_color.unwrap_or_else(|| Self::usage_color_with_palette(p, clamped * 100.0));

        let mut spans = Vec::with_capacity(5);
        spans.push(Span::styled("[", Style::default().fg(p.text_muted)));
        
        let mut filled = "█".repeat(full_blocks);
        if has_frac {
            filled.push(frac_char);
        }
        if !filled.is_empty() {
            spans.push(Span::styled(filled, Style::default().fg(color)));
        }
        if empty > 0 {
            spans.push(Span::styled("░".repeat(empty), Style::default().fg(p.bar_empty)));
        }
        spans.push(Span::styled("]", Style::default().fg(p.text_muted)));
        spans
    }

    /// Compact progress bar for table cells
    pub fn micro_bar_spans(p: &ThemeColors, percent: f32, width: usize, custom_color: Option<Color>) -> Vec<Span<'static>> {
        let ratio = (percent / 100.0).clamp(0.0, 1.0);
        let total_chars = (ratio * width as f32).max(0.0);
        let full_blocks = (total_chars.floor() as usize).min(width);
        let remainder = total_chars - full_blocks as f32;

        let frac_char = if remainder >= 0.875 {
            '▉'
        } else if remainder >= 0.75 {
            '▊'
        } else if remainder >= 0.625 {
            '▋'
        } else if remainder >= 0.5 {
            '▌'
        } else if remainder >= 0.375 {
            '▍'
        } else if remainder >= 0.25 {
            '▎'
        } else if remainder >= 0.125 {
            '▏'
        } else {
            ' '
        };

        let has_frac = frac_char != ' ' && full_blocks < width;
        let empty = width.saturating_sub(full_blocks + if has_frac { 1 } else { 0 });
        let color = custom_color.unwrap_or_else(|| Self::usage_color_with_palette(p, percent));

        let mut spans = Vec::with_capacity(5);
        spans.push(Span::styled("[", Style::default().fg(p.bar_empty)));
        
        let mut filled = "█".repeat(full_blocks);
        if has_frac {
            filled.push(frac_char);
        }
        if !filled.is_empty() {
            spans.push(Span::styled(filled, Style::default().fg(color)));
        }
        if empty > 0 {
            spans.push(Span::styled("░".repeat(empty), Style::default().fg(p.bar_empty)));
        }
        spans.push(Span::styled("]", Style::default().fg(p.bar_empty)));
        spans
    }

    /// Compact single-string bar with its color
    #[allow(dead_code)]
    pub fn micro_bar(percent: f32, width: usize) -> (String, Color) {
        let ratio = (percent / 100.0).clamp(0.0, 1.0);
        let filled = (ratio * width as f32).round() as usize;
        let filled = filled.min(width);
        let empty = width.saturating_sub(filled);
        let color = Self::usage_color(percent);
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
        (bar, color)
    }

    /// Formats bytes into clean human-readable units (B, KiB, MiB, GiB, TiB)
    pub fn format_bytes(bytes: u64) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * KIB;
        const GIB: u64 = 1024 * MIB;
        const TIB: u64 = 1024 * GIB;

        if bytes >= TIB {
            format!("{:.2} TiB", bytes as f64 / TIB as f64)
        } else if bytes >= GIB {
            format!("{:.2} GiB", bytes as f64 / GIB as f64)
        } else if bytes >= MIB {
            format!("{:.1} MiB", bytes as f64 / MIB as f64)
        } else if bytes >= KIB {
            format!("{:.1} KiB", bytes as f64 / KIB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Formats bytes into compact human-readable units (e.g. "2.4G", "120M", "4.5K")
    pub fn format_bytes_compact(bytes: u64) -> String {
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * KIB;
        const GIB: u64 = 1024 * MIB;
        const TIB: u64 = 1024 * GIB;

        if bytes >= 100 * TIB {
            format!("{:.0}T", bytes as f64 / TIB as f64)
        } else if bytes >= TIB {
            format!("{:.1}T", bytes as f64 / TIB as f64)
        } else if bytes >= 100 * GIB {
            format!("{:.0}G", bytes as f64 / GIB as f64)
        } else if bytes >= GIB {
            format!("{:.1}G", bytes as f64 / GIB as f64)
        } else if bytes >= 100 * MIB {
            format!("{:.0}M", bytes as f64 / MIB as f64)
        } else if bytes >= MIB {
            format!("{:.1}M", bytes as f64 / MIB as f64)
        } else if bytes >= 100 * KIB {
            format!("{:.0}K", bytes as f64 / KIB as f64)
        } else if bytes >= KIB {
            format!("{:.1}K", bytes as f64 / KIB as f64)
        } else {
            format!("{}B", bytes)
        }
    }

    /// Formats seconds into compact duration (e.g. "3d 8h 20m")
    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else {
            format!("{}m {}s", minutes, secs)
        }
    }

    /// Cleans noisy vendor strings from CPU model name
    pub fn clean_cpu_model(raw: &str) -> String {
        let cleaned = raw
            .replace("(R)", "")
            .replace("(TM)", "")
            .replace("(tm)", "")
            .replace("CPU", "")
            .replace("Processor", "");
        let mut s = cleaned.as_str();
        if let Some(pos) = s.find('@') {
            s = &s[..pos];
        }
        let mut parts: Vec<&str> = s.split_whitespace().collect();
        if parts.last() == Some(&"0") || parts.last() == Some(&"v0") {
            parts.pop();
        }
        parts.join(" ")
    }
}
