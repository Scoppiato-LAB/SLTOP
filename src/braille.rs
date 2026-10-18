use crate::theme::{Theme, ThemeColors};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Renders a 2x4 Braille chart for CPU history with Y-axis scale.
pub fn render_braille_chart(
    p: &ThemeColors,
    data: &[u64],
    width: usize,
    height: usize,
    max_val: f64,
) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let show_axis = width >= 14 && height >= 3;
    let axis_width = if show_axis { 7 } else { 0 };
    let graph_width = width.saturating_sub(axis_width);
    let graph_height = if show_axis {
        height.saturating_sub(1)
    } else {
        height
    };

    let sub_width = graph_width * 2;
    let sub_height = graph_height * 4;

    let mut sampled = vec![0.0f64; sub_width];
    let data_len = data.len();

    if data_len > 0 {
        if data_len >= sub_width {
            let offset = data_len - sub_width;
            for i in 0..sub_width {
                sampled[i] = data[offset + i] as f64;
            }
        } else {
            let offset = sub_width - data_len;
            for i in 0..data_len {
                sampled[offset + i] = data[i] as f64;
            }
        }
    }

    let mut cell_bits = vec![0u8; graph_width * graph_height];

    for (col_sub, &val) in sampled.iter().enumerate() {
        let cell_x = col_sub / 2;
        let sub_x = col_sub % 2;

        let ratio = if max_val > 0.0 {
            (val / max_val).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let mut filled_dots = (ratio * (sub_height as f64)).round() as usize;
        if val > 0.0 && filled_dots == 0 {
            filled_dots = 1;
        }

        for dot_y in 0..filled_dots.min(sub_height) {
            let row_from_top = (sub_height - 1 - dot_y) / 4;
            let dot_in_cell_from_top = (sub_height - 1 - dot_y) % 4;

            let bit = match (sub_x, dot_in_cell_from_top) {
                (0, 0) => 0x01,
                (0, 1) => 0x02,
                (0, 2) => 0x04,
                (0, 3) => 0x40,
                (1, 0) => 0x08,
                (1, 1) => 0x10,
                (1, 2) => 0x20,
                (1, 3) => 0x80,
                _ => 0,
            };

            if row_from_top < graph_height && cell_x < graph_width {
                cell_bits[row_from_top * graph_width + cell_x] |= bit;
            }
        }
    }

    let mut col_colors = Vec::with_capacity(graph_width);
    for col_idx in 0..graph_width {
        let v0 = sampled[col_idx * 2];
        let v1 = sampled[col_idx * 2 + 1];
        let max_col_val = v0.max(v1) as f32;
        col_colors.push(Theme::usage_color_with_palette(p, max_col_val));
    }

    let mut lines = Vec::with_capacity(height);

    for row_idx in 0..graph_height {
        let mut spans = Vec::with_capacity(width);

        if show_axis {
            let (label_text, tick_char) = if row_idx == 0 {
                (" 100%", " ┤")
            } else if row_idx == graph_height / 2 {
                ("  50%", " ┤")
            } else {
                ("     ", " │")
            };

            spans.push(Span::styled(label_text, Style::default().fg(p.text_muted)));
            spans.push(Span::styled(tick_char, Style::default().fg(p.card_border)));
        }

        let row_height_ratio = 1.0 - (row_idx as f32 / graph_height.max(1) as f32);
        let row_gradient_color = Theme::vertical_gradient_color(p, row_height_ratio);

        let row_offset = row_idx * graph_width;
        for col_idx in 0..graph_width {
            let bits = cell_bits[row_offset + col_idx];
            let ch = if bits == 0 {
                ' '
            } else {
                char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
            };

            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(row_gradient_color),
            ));
        }

        lines.push(Line::from(spans));
    }

    if show_axis {
        let base_spans = vec![
            Span::styled("   0%", Style::default().fg(p.text_muted)),
            Span::styled(" ┴", Style::default().fg(p.card_border)),
            Span::styled("─".repeat(graph_width), Style::default().fg(p.card_border)),
        ];
        lines.push(Line::from(base_spans));
    }

    lines
}

/// Renders a dual-series Braille chart for network Rx and Tx traffic.
pub fn render_network_braille_chart(
    p: &ThemeColors,
    rx_data: &[u64],
    tx_data: &[u64],
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let show_axis = width >= 14 && height >= 3;
    let axis_width = if show_axis { 7 } else { 0 };
    let graph_width = width.saturating_sub(axis_width);
    let graph_height = if show_axis {
        height.saturating_sub(1)
    } else {
        height
    };

    let sub_width = graph_width * 2;
    let sub_height = graph_height * 4;

    // Resample Rx and Tx
    let mut rx_sampled = vec![0.0f64; sub_width];
    let mut tx_sampled = vec![0.0f64; sub_width];

    let rx_len = rx_data.len();
    if rx_len > 0 {
        if rx_len >= sub_width {
            let offset = rx_len - sub_width;
            for i in 0..sub_width {
                rx_sampled[i] = rx_data[offset + i] as f64;
            }
        } else {
            let offset = sub_width - rx_len;
            for i in 0..rx_len {
                rx_sampled[offset + i] = rx_data[i] as f64;
            }
        }
    }

    let tx_len = tx_data.len();
    if tx_len > 0 {
        if tx_len >= sub_width {
            let offset = tx_len - sub_width;
            for i in 0..sub_width {
                tx_sampled[i] = tx_data[offset + i] as f64;
            }
        } else {
            let offset = sub_width - tx_len;
            for i in 0..tx_len {
                tx_sampled[offset + i] = tx_data[i] as f64;
            }
        }
    }

    // Dynamic scale: compute max between Rx and Tx with a minimum base of 1024 B/s (1 KiB/s)
    let mut max_rate = 1024.0f64;
    for &v in rx_sampled.iter().chain(tx_sampled.iter()) {
        if v > max_rate {
            max_rate = v;
        }
    }

    let mut rx_bits = vec![0u8; graph_width * graph_height];
    let mut tx_bits = vec![0u8; graph_width * graph_height];

    // Plot Rx
    for (col_sub, &val) in rx_sampled.iter().enumerate() {
        let cell_x = col_sub / 2;
        let sub_x = col_sub % 2;
        let ratio = (val / max_rate).clamp(0.0, 1.0);
        let mut filled_dots = (ratio * (sub_height as f64)).round() as usize;
        if val > 0.0 && filled_dots == 0 {
            filled_dots = 1;
        }

        for dot_y in 0..filled_dots.min(sub_height) {
            let row_from_top = (sub_height - 1 - dot_y) / 4;
            let dot_in_cell = (sub_height - 1 - dot_y) % 4;

            let bit = match (sub_x, dot_in_cell) {
                (0, 0) => 0x01,
                (0, 1) => 0x02,
                (0, 2) => 0x04,
                (0, 3) => 0x40,
                (1, 0) => 0x08,
                (1, 1) => 0x10,
                (1, 2) => 0x20,
                (1, 3) => 0x80,
                _ => 0,
            };

            if row_from_top < graph_height && cell_x < graph_width {
                rx_bits[row_from_top * graph_width + cell_x] |= bit;
            }
        }
    }

    // Plot Tx
    for (col_sub, &val) in tx_sampled.iter().enumerate() {
        let cell_x = col_sub / 2;
        let sub_x = col_sub % 2;
        let ratio = (val / max_rate).clamp(0.0, 1.0);
        let mut filled_dots = (ratio * (sub_height as f64)).round() as usize;
        if val > 0.0 && filled_dots == 0 {
            filled_dots = 1;
        }

        for dot_y in 0..filled_dots.min(sub_height) {
            let row_from_top = (sub_height - 1 - dot_y) / 4;
            let dot_in_cell = (sub_height - 1 - dot_y) % 4;

            let bit = match (sub_x, dot_in_cell) {
                (0, 0) => 0x01,
                (0, 1) => 0x02,
                (0, 2) => 0x04,
                (0, 3) => 0x40,
                (1, 0) => 0x08,
                (1, 1) => 0x10,
                (1, 2) => 0x20,
                (1, 3) => 0x80,
                _ => 0,
            };

            if row_from_top < graph_height && cell_x < graph_width {
                tx_bits[row_from_top * graph_width + cell_x] |= bit;
            }
        }
    }

    let mut lines = Vec::with_capacity(height);

    for row_idx in 0..graph_height {
        let mut spans = Vec::with_capacity(width);

        if show_axis {
            let (label_text, tick_char) = if row_idx == 0 {
                (
                    format!("{:>5}", Theme::format_bytes_compact(max_rate as u64)),
                    " ┤",
                )
            } else if row_idx == graph_height / 2 {
                (
                    format!(
                        "{:>5}",
                        Theme::format_bytes_compact((max_rate / 2.0) as u64)
                    ),
                    " ┤",
                )
            } else {
                ("     ".to_string(), " │")
            };

            spans.push(Span::styled(label_text, Style::default().fg(p.text_muted)));
            spans.push(Span::styled(tick_char, Style::default().fg(p.card_border)));
        }

        let row_offset = row_idx * graph_width;
        for col_idx in 0..graph_width {
            let r_bit = rx_bits[row_offset + col_idx];
            let t_bit = tx_bits[row_offset + col_idx];
            let combined = r_bit | t_bit;

            let ch = if combined == 0 {
                ' '
            } else {
                char::from_u32(0x2800 + combined as u32).unwrap_or(' ')
            };

            let color = if r_bit > 0 && t_bit > 0 {
                p.lavender // Combined Rx and Tx
            } else if r_bit > 0 {
                p.mint // Rx
            } else {
                p.sky // Tx
            };

            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }

        lines.push(Line::from(spans));
    }

    if show_axis {
        let base_spans = vec![
            Span::styled("   0B", Style::default().fg(p.text_muted)),
            Span::styled(" ┴", Style::default().fg(p.card_border)),
            Span::styled("─".repeat(graph_width), Style::default().fg(p.card_border)),
        ];
        lines.push(Line::from(base_spans));
    }

    lines
}
