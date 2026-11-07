// src/ui.rs
//! Polished, high-contrast Bloomberg Terminal TUI workstation renderer.

use crate::{
    app::{App, ChartMode, Focus, WorkspaceView},
    data::Candle,
    ticker::AssetCategory,
    timeframe::Timeframe,
};
use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, BorderType, Borders, Chart, Clear, Dataset, List, ListItem, Paragraph,
        Row, Table, Widget,
    },
    Frame,
};

pub mod palette {
    use ratatui::style::Color;
    pub const ACTIVE_BORDER: Color = Color::Rgb(0, 220, 255); // Electric Cyan
    pub const INACTIVE_BORDER: Color = Color::Rgb(70, 88, 110); // Steel Slate
    pub const BULLISH: Color = Color::Rgb(46, 213, 115); // Vivid Emerald Green
    pub const BEARISH: Color = Color::Rgb(255, 71, 87); // Bright Coral Red
    pub const ACCENT: Color = Color::Rgb(255, 170, 0); // Electric Amber / Gold
    pub const TEXT_PRIMARY: Color = Color::Rgb(255, 255, 255); // Crisp Bright White
    pub const TEXT_MUTED: Color = Color::Rgb(165, 180, 198); // Cool Ice Grey

    // Category Tags
    pub const TAG_STOCK: Color = Color::Rgb(75, 123, 236);
    pub const TAG_CRYPTO: Color = Color::Rgb(165, 94, 234);
    pub const TAG_BOND: Color = Color::Rgb(43, 203, 186);
    pub const TAG_FOREX: Color = Color::Rgb(253, 150, 68);
    pub const TAG_COMMODITY: Color = Color::Rgb(235, 94, 40);
}

pub fn draw_ui(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Workstation Tabs
            Constraint::Min(12),   // Active Workspace View
            Constraint::Length(3), // Bottom Command Bar
        ])
        .split(area);

    draw_top_nav(f, app, chunks[0]);

    match app.current_view {
        WorkspaceView::Overview => draw_overview_view(f, app, chunks[1]),
        WorkspaceView::Options => draw_options_view(f, app, chunks[1]),
        WorkspaceView::Correlation => draw_correlation_view(f, app, chunks[1]),
        WorkspaceView::Insiders => draw_insiders_view(f, app, chunks[1]),
        WorkspaceView::YieldCurve => draw_yield_curve_view(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);

    if app.show_add_modal {
        draw_add_modal(f, app, area);
    }
}

fn draw_top_nav(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette::INACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" MARKET TERMINAL ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" [Switch Tabs: 'w' or F1-F5] ", Style::default().fg(palette::TEXT_MUTED)),
        ]));

    let views = [
        WorkspaceView::Overview,
        WorkspaceView::Options,
        WorkspaceView::Correlation,
        WorkspaceView::Insiders,
        WorkspaceView::YieldCurve,
    ];

    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    for v in views {
        let is_active = v == app.current_view;
        if is_active {
            spans.push(Span::styled(
                format!("  {}  ", v.label()),
                Style::default()
                    .bg(palette::ACCENT)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!("  {}  ", v.label()),
                Style::default().fg(palette::TEXT_MUTED),
            ));
        }
        spans.push(Span::raw(" "));
    }

    let p = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(p, area);
}

// ---------------------------------------------------------------------------
// VIEW 1: OVERVIEW (WATCHLIST, CANDLESTICK/SOLID CHART, SCRUBBER, LIVE NEWS)
// ---------------------------------------------------------------------------

fn draw_overview_view(f: &mut Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(40)])
        .split(area);

    draw_watchlist(f, app, body_chunks[0]);
    draw_right_panel(f, app, body_chunks[1]);
}

fn draw_watchlist(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == Focus::Watchlist {
        palette::ACTIVE_BORDER
    } else {
        palette::INACTIVE_BORDER
    };

    let cat_title = match app.category_filter {
        Some(cat) => format!(" Watchlist [{}] ", cat.label()),
        None => " Watchlist [ALL] ".to_string(),
    };

    let title = Line::from(vec![
        Span::styled(cat_title, Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("({})", app.watchlist.len()), Style::default().fg(palette::TEXT_MUTED)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.focus == Focus::Watchlist { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color))
        .title(title);

    let filtered_indices = app.filtered_indices();

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .map(|&i| {
            let t = &app.watchlist[i];
            let is_selected = i == app.selected;

            let (tag_label, tag_color) = match t.category {
                AssetCategory::Stock => ("STK", palette::TAG_STOCK),
                AssetCategory::Crypto => ("CRP", palette::TAG_CRYPTO),
                AssetCategory::Bond => ("BND", palette::TAG_BOND),
                AssetCategory::Forex => ("FRX", palette::TAG_FOREX),
                AssetCategory::Commodity => ("CMD", palette::TAG_COMMODITY),
            };

            let cursor = if is_selected { "▶ " } else { "  " };

            let change_color = if t.change >= 0.0 {
                palette::BULLISH
            } else {
                palette::BEARISH
            };

            let sign = if t.change >= 0.0 { "+" } else { "" };
            let price_str = if t.current_price > 0.0 {
                if t.category == AssetCategory::Bond {
                    format!("{:.3}%", t.current_price)
                } else if t.current_price >= 1000.0 {
                    format!("${:.1}", t.current_price)
                } else {
                    format!("${:.2}", t.current_price)
                }
            } else {
                "---".to_string()
            };

            let change_str = if t.current_price > 0.0 {
                format!("{}{:.1}%", sign, t.change_percent)
            } else {
                "".to_string()
            };

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[{}] ", tag_label), Style::default().fg(tag_color)),
                Span::styled(format!("{:<8} ", t.symbol), Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>7} ", price_str), Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(change_str, Style::default().fg(change_color).add_modifier(Modifier::BOLD)),
            ]);

            let mut style = Style::default();
            if is_selected {
                style = style.bg(Color::Rgb(20, 36, 56));
            }

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_right_panel(f: &mut Frame, app: &App, area: Rect) {
    let constraints = if app.show_news {
        vec![
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(9),
            Constraint::Length(6),
        ]
    } else {
        vec![
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(12),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_header_card(f, app, chunks[0]);
    draw_timeframe_bar(f, app, chunks[1]);
    draw_chart_panel(f, app, chunks[2]);

    if app.show_news {
        draw_news_panel(f, app, chunks[3]);
    }
}

fn draw_header_card(f: &mut Frame, app: &App, area: Rect) {
    let border_color = palette::INACTIVE_BORDER;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let active_ticker = app.selected_ticker();
    let (sym, name, price, change, change_pct, high, low, vol, cat) = match active_ticker {
        Some(t) => (
            t.symbol.as_str(),
            t.display_name.as_str(),
            t.current_price,
            t.change,
            t.change_percent,
            t.high_24h,
            t.low_24h,
            t.volume,
            t.category,
        ),
        None => ("--", "--", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, AssetCategory::Stock),
    };

    let change_color = if change >= 0.0 { palette::BULLISH } else { palette::BEARISH };
    let arrow = if change >= 0.0 { "▲" } else { "▼" };
    let sign = if change >= 0.0 { "+" } else { "" };

    let price_formatted = if cat == AssetCategory::Bond {
        format!("{:.3}%", price)
    } else {
        format!("${:.2}", price)
    };

    let line1 = Line::from(vec![
        Span::styled(format!("  {}  ", sym), Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", name), Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("  {} ", price_formatted), Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(" {} {}${:.2} ({}{:.2}%)", arrow, sign, change.abs(), sign, change_pct),
            Style::default().fg(change_color).add_modifier(Modifier::BOLD),
        ),
    ]);

    let line2 = Line::from(vec![
        Span::styled("    24h High: ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("${:.2}   ", high), Style::default().fg(palette::TEXT_PRIMARY)),
        Span::styled("24h Low: ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("${:.2}   ", low), Style::default().fg(palette::TEXT_PRIMARY)),
        Span::styled("Volume: ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("{:.0}", vol), Style::default().fg(palette::TEXT_PRIMARY)),
        Span::styled("   Category: ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("{}", cat.label()), Style::default().fg(palette::ACCENT)),
    ]);

    let paragraph = Paragraph::new(vec![Line::from(""), line1, line2]).block(block);
    f.render_widget(paragraph, area);
}

fn draw_timeframe_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette::INACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" Timeframe: Keys 1-6 | Walk Timeline: ←/→ | Mode: ", Style::default().fg(palette::TEXT_MUTED)),
            Span::styled(format!("[m: {}] ", app.chart_mode.label()), Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        ]));

    let mut spans = Vec::new();
    spans.push(Span::raw("   "));

    for tf in Timeframe::ALL {
        let is_active = tf == app.timeframe;
        if is_active {
            spans.push(Span::styled(
                format!(" [ {} ] ", tf.label()),
                Style::default()
                    .bg(palette::ACCENT)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!("   {}   ", tf.label()),
                Style::default().fg(palette::TEXT_MUTED),
            ));
        }
        spans.push(Span::raw("  "));
    }

    let p = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(p, area);
}

fn draw_chart_panel(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == Focus::Chart {
        palette::ACTIVE_BORDER
    } else {
        palette::INACTIVE_BORDER
    };

    let mut title_spans = vec![
        Span::styled(" Market Chart ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}] ", app.timeframe.label()), Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
    ];

    if let Some(c) = app.inspected_candle() {
        let diff = c.close - c.open;
        let diff_sign = if diff >= 0.0 { "+" } else { "" };
        let diff_color = if diff >= 0.0 { palette::BULLISH } else { palette::BEARISH };
        let cursor_label = if app.chart_cursor.is_some() { "INSPECT" } else { "LATEST" };

        title_spans.push(Span::styled(
            format!("[{}: {}] ", cursor_label, format_ts(c.timestamp)),
            Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            format!("O: ${:.2}  H: ${:.2}  L: ${:.2}  C: ${:.2} ", c.open, c.high, c.low, c.close),
            Style::default().fg(palette::TEXT_PRIMARY),
        ));
        title_spans.push(Span::styled(
            format!("({}{:.2}) ", diff_sign, diff),
            Style::default().fg(diff_color).add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            format!("Vol: {:.0} (←/→ scrub)", c.volume),
            Style::default().fg(palette::TEXT_MUTED),
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.focus == Focus::Chart { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.candles.is_empty() {
        let empty_msg = Paragraph::new("Loading market data...")
            .style(Style::default().fg(palette::TEXT_MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty_msg, inner);
        return;
    }

    match app.chart_mode {
        ChartMode::Candlestick => {
            let candle_widget = CandlestickWidget {
                candles: &app.candles,
                cursor_idx: app.chart_cursor,
            };
            f.render_widget(candle_widget, inner);
        }
        ChartMode::SolidLine => {
            draw_dense_solid_line_chart(f, app, inner);
        }
    }
}

struct CandlestickWidget<'a> {
    candles: &'a [Candle],
    cursor_idx: Option<usize>,
}

impl<'a> Widget for CandlestickWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 12 || area.height < 4 || self.candles.is_empty() {
            return;
        }

        let chart_height = (area.height.saturating_sub(2)) as usize;
        let chart_width = area.width.saturating_sub(10) as usize;

        let candle_count = self.candles.len();
        if candle_count == 0 || chart_width == 0 {
            return;
        }

        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;
        let mut max_vol = 1.0;

        for c in self.candles {
            if c.low < min_price { min_price = c.low; }
            if c.high > max_price { max_price = c.high; }
            if c.volume > max_vol { max_vol = c.volume; }
        }

        if (max_price - min_price).abs() < 0.0001 {
            max_price += 1.0;
            min_price -= 1.0;
        }

        let price_range = max_price - min_price;

        let top_label = format!("${:.2}", max_price);
        let mid_label = format!("${:.2}", (max_price + min_price) / 2.0);
        let bot_label = format!("${:.2}", min_price);

        buf.set_string(area.x + area.width - 9, area.y, &top_label, Style::default().fg(palette::TEXT_MUTED));
        buf.set_string(area.x + area.width - 9, area.y + (chart_height as u16 / 2), &mid_label, Style::default().fg(palette::TEXT_MUTED));
        buf.set_string(area.x + area.width - 9, area.y + chart_height as u16 - 1, &bot_label, Style::default().fg(palette::TEXT_MUTED));

        // When many candles exist, fit at most chart_width candles
        let max_fit = chart_width;
        let (visible_candles, offset) = if candle_count > max_fit {
            let start = candle_count - max_fit;
            (&self.candles[start..], start)
        } else {
            (self.candles, 0)
        };

        let visible_count = visible_candles.len();
        if visible_count == 0 {
            return;
        }

        // Determine dynamic candle width and spacing so the candles always span 100% of chart_width
        let avg_space = chart_width as f64 / visible_count as f64;
        let body_width = if avg_space >= 4.0 {
            3
        } else if avg_space >= 2.5 {
            2
        } else {
            1
        };

        let active_cur_global = self.cursor_idx.unwrap_or(self.candles.len().saturating_sub(1));

        for (idx, candle) in visible_candles.iter().enumerate() {
            let global_idx = offset + idx;
            let is_inspected = global_idx == active_cur_global;

            // Evenly distribute columns across the full chart_width
            let col = if visible_count > 1 {
                let span = (chart_width.saturating_sub(body_width)) as f64;
                area.x + ((idx as f64 / (visible_count - 1) as f64) * span).round() as u16
            } else {
                area.x + (chart_width / 2) as u16
            };

            if col >= area.x + chart_width as u16 {
                break;
            }

            let is_bullish = candle.close >= candle.open;
            let candle_color = if is_inspected {
                palette::ACTIVE_BORDER
            } else if is_bullish {
                palette::BULLISH
            } else {
                palette::BEARISH
            };

            let style = Style::default().fg(candle_color).add_modifier(Modifier::BOLD);

            let high_row = ((max_price - candle.high) / price_range * (chart_height - 1) as f64).round() as usize;
            let low_row = ((max_price - candle.low) / price_range * (chart_height - 1) as f64).round() as usize;

            let body_top_price = candle.open.max(candle.close);
            let body_bot_price = candle.open.min(candle.close);

            let body_top_row = ((max_price - body_top_price) / price_range * (chart_height - 1) as f64).round() as usize;
            let body_bot_row = ((max_price - body_bot_price) / price_range * (chart_height - 1) as f64).round() as usize;

            let wick_col = col + (body_width as u16 / 2);

            if is_inspected {
                for r in 0..chart_height {
                    let cell_y = area.y + r as u16;
                    buf.set_string(wick_col, cell_y, "┊", Style::default().fg(Color::Rgb(40, 60, 85)));
                }
            }

            for r in high_row..body_top_row {
                let cell_y = area.y + r as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(wick_col, cell_y, "│", style);
                }
            }

            let body_char = match body_width {
                1 => "█",
                2 => "██",
                _ => "███",
            };

            let doji_char = match body_width {
                1 => "┼",
                2 => "┼┼",
                _ => "┼┼┼",
            };

            if body_top_row == body_bot_row {
                let cell_y = area.y + body_top_row as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(col, cell_y, doji_char, style);
                }
            } else {
                for r in body_top_row..=body_bot_row {
                    let cell_y = area.y + r as u16;
                    if cell_y < area.y + chart_height as u16 {
                        buf.set_string(col, cell_y, body_char, style);
                    }
                }
            }

            for r in (body_bot_row + 1)..=low_row {
                let cell_y = area.y + r as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(wick_col, cell_y, "│", style);
                }
            }

            let vol_row = area.y + chart_height as u16;
            let vol_frac = (candle.volume / max_vol).clamp(0.0, 1.0);
            let single_vol = match (vol_frac * 8.0).round() as u8 {
                0 => " ",
                1 => " ",
                2 => "▂",
                3 => "▃",
                4 => "▄",
                5 => "▅",
                6 => "▆",
                7 => "▇",
                _ => "█",
            };
            let vol_str = single_vol.repeat(body_width);
            buf.set_string(col, vol_row, &vol_str, Style::default().fg(candle_color));

            if is_inspected {
                let marker_row = area.y + area.height - 1;
                buf.set_string(wick_col, marker_row, "▲", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD));
            }
        }

        let time_row = area.y + area.height - 1;
        if let Some(first) = visible_candles.first() {
            buf.set_string(area.x, time_row, format_ts(first.timestamp), Style::default().fg(palette::TEXT_MUTED));
        }
        if let Some(last) = visible_candles.last() {
            let label = format_ts(last.timestamp);
            let x = (area.x + chart_width as u16).saturating_sub(label.len() as u16);
            buf.set_string(x, time_row, label, Style::default().fg(palette::TEXT_MUTED));
        }
    }
}

fn draw_dense_solid_line_chart(f: &mut Frame, app: &App, area: Rect) {
    let first_price = app.chart_data.first().map(|p| p.1).unwrap_or(0.0);
    let last_price = app.chart_data.last().map(|p| p.1).unwrap_or(0.0);
    let is_bullish = last_price >= first_price;
    let line_color = if is_bullish { palette::BULLISH } else { palette::BEARISH };

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(line_color))
        .data(&app.chart_data);

    let x_min = app.chart_data.first().map(|p| p.0).unwrap_or(0.0);
    let x_max = app.chart_data.last().map(|p| p.0).unwrap_or(1.0);

    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for &(_, val) in &app.chart_data {
        if val < y_min { y_min = val; }
        if val > y_max { y_max = val; }
    }
    if y_min == f64::MAX {
        y_min = 0.0;
        y_max = 1.0;
    }

    let pad = (y_max - y_min) * 0.05;
    let y_bounds = [y_min - pad, y_max + pad];

    let x_labels = vec![
        Span::styled(format_ts(x_min), Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format_ts((x_min + x_max) / 2.0), Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format_ts(x_max), Style::default().fg(palette::TEXT_MUTED)),
    ];

    let y_labels = vec![
        Span::styled(format!("${:.2}", y_min), Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("${:.2}", (y_min + y_max) / 2.0), Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(format!("${:.2}", y_max), Style::default().fg(palette::TEXT_MUTED)),
    ];

    let chart = Chart::new(vec![dataset])
        .x_axis(Axis::default().bounds([x_min, x_max]).labels(x_labels))
        .y_axis(Axis::default().bounds(y_bounds).labels(y_labels));

    f.render_widget(chart, area);
}

fn draw_news_panel(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == Focus::News {
        palette::ACTIVE_BORDER
    } else {
        palette::INACTIVE_BORDER
    };

    let ticker_sym = app.selected_ticker().map(|t| t.symbol.as_str()).unwrap_or("--");
    let title = Line::from(vec![
        Span::styled(" Live Market News & Filings ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}] ", ticker_sym), Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("[Enter/o: Open in Browser | ↑/↓: Select] ", Style::default().fg(palette::ACTIVE_BORDER)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.focus == Focus::News { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color))
        .title(title);

    let items: Vec<ListItem> = app
        .news
        .iter()
        .enumerate()
        .take(5)
        .map(|(i, item)| {
            let is_selected = app.focus == Focus::News && i == app.news_selected;
            let relative_time = format_relative_time(item.published_time);

            let cursor = if is_selected { "▶ " } else { "  " };
            let cursor_style = if is_selected {
                Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::TEXT_MUTED)
            };

            let title_style = if is_selected {
                Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::TEXT_PRIMARY)
            };

            let line = Line::from(vec![
                Span::styled(cursor, cursor_style),
                Span::styled(format!("{}: ", item.publisher), Style::default().fg(palette::ACCENT)),
                Span::styled(format!("{:<60} ", truncate(&item.title, 65)), title_style),
                Span::styled(format!("({})", relative_time), Style::default().fg(palette::TEXT_MUTED)),
            ]);

            let mut style = Style::default();
            if is_selected {
                style = style.bg(Color::Rgb(25, 45, 70));
            }

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

// ---------------------------------------------------------------------------
// VIEW 2: OPTIONS CHAIN & MAX PAIN
// ---------------------------------------------------------------------------

fn draw_options_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(palette::ACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" LIVE OPTIONS CHAIN & MAX PAIN RADAR ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("[OVME] ", Style::default().fg(palette::ACCENT)),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(opt) = &app.options_chain {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(8)])
            .split(inner);

        let header_line1 = Line::from(vec![
            Span::styled(format!(" Symbol: {} ", opt.symbol), Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" Spot Price: ${:.2}  ", opt.current_price), Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" Expiration: {}  ", opt.expiration_date), Style::default().fg(palette::TEXT_MUTED)),
            Span::styled(
                format!(" ★ MAX PAIN STRIKE: ${:.2} ★ ", opt.max_pain_strike),
                Style::default().bg(palette::ACCENT).fg(Color::Black).add_modifier(Modifier::BOLD),
            ),
        ]);

        let header_line2 = Line::from(vec![
            Span::styled(format!(" Total Call OI: {}   ", opt.total_call_oi), Style::default().fg(palette::BULLISH)),
            Span::styled(format!(" Total Put OI: {}   ", opt.total_put_oi), Style::default().fg(palette::BEARISH)),
            Span::styled(format!(" Put/Call OI Ratio: {:.2} ", opt.put_call_ratio), Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
            Span::styled(
                if opt.put_call_ratio > 1.0 { " (Bearish Hedging Skew)" } else { " (Bullish Call Bias)" },
                Style::default().fg(palette::TEXT_MUTED),
            ),
        ]);

        let header_p = Paragraph::new(vec![Line::from(""), header_line1, header_line2]);
        f.render_widget(header_p, v_chunks[0]);

        let header_row = Row::new(vec![
            "CALL BID", "CALL ASK", "CALL VOL", "CALL OI", "CALL IV",
            "  STRIKE  ",
            "PUT BID", "PUT ASK", "PUT VOL", "PUT OI", "PUT IV",
        ])
        .style(Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = opt
            .calls
            .iter()
            .zip(opt.puts.iter())
            .map(|(c, p)| {
                let is_max_pain = (c.strike - opt.max_pain_strike).abs() < 0.01;
                let strike_str = if is_max_pain {
                    format!("★ ${:.2} ★", c.strike)
                } else {
                    format!("  ${:.2}  ", c.strike)
                };

                let strike_style = if is_max_pain {
                    Style::default().bg(palette::ACCENT).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
                };

                Row::new(vec![
                    Span::styled(format!("${:.2}", c.bid), Style::default().fg(palette::BULLISH)),
                    Span::styled(format!("${:.2}", c.ask), Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(format!("{}", c.volume), Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(format!("{}", c.open_interest), Style::default().fg(palette::TEXT_PRIMARY)),
                    Span::styled(format!("{:.1}%", c.implied_volatility), Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(strike_str, strike_style),
                    Span::styled(format!("${:.2}", p.bid), Style::default().fg(palette::BEARISH)),
                    Span::styled(format!("${:.2}", p.ask), Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(format!("{}", p.volume), Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(format!("{}", p.open_interest), Style::default().fg(palette::TEXT_PRIMARY)),
                    Span::styled(format!("{:.1}%", p.implied_volatility), Style::default().fg(palette::TEXT_MUTED)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(9),
                Constraint::Percentage(9),
                Constraint::Percentage(8),
                Constraint::Percentage(8),
                Constraint::Percentage(8),
                Constraint::Percentage(16),
                Constraint::Percentage(9),
                Constraint::Percentage(9),
                Constraint::Percentage(8),
                Constraint::Percentage(8),
                Constraint::Percentage(8),
            ],
        )
        .header(header_row);

        f.render_widget(table, v_chunks[1]);
    } else {
        let p = Paragraph::new("Loading Options Chain and Max Pain statistics...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette::TEXT_MUTED));
        f.render_widget(p, inner);
    }
}

// ---------------------------------------------------------------------------
// VIEW 3: CROSS-ASSET CORRELATION MATRIX & ROLLING BETA
// ---------------------------------------------------------------------------

fn draw_correlation_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(palette::ACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" CROSS-ASSET PEARSON CORRELATION MATRIX & ROLLING BETA ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("[CORR] (Benchmark: SPY)", Style::default().fg(palette::ACCENT)),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(corr) = &app.correlation {
        let n = corr.symbols.len();
        if n == 0 {
            return;
        }

        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(5)])
            .split(inner);

        let mut header_cells = vec!["ASSET".to_string()];
        header_cells.extend(corr.symbols.iter().cloned());
        let header_row = Row::new(header_cells).style(Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD));

        let mut table_rows = Vec::new();
        for i in 0..n {
            let mut row_spans = Vec::new();
            row_spans.push(Span::styled(format!("{:<8}", corr.symbols[i]), Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)));

            for j in 0..n {
                let val = corr.matrix[i][j];
                let val_str = format!("{:>6.2}", val);

                let cell_style = if i == j {
                    Style::default().fg(palette::TEXT_MUTED)
                } else if val >= 0.70 {
                    Style::default().fg(palette::BULLISH).add_modifier(Modifier::BOLD)
                } else if val >= 0.30 {
                    Style::default().fg(Color::Rgb(100, 220, 140))
                } else if val <= -0.70 {
                    Style::default().fg(palette::BEARISH).add_modifier(Modifier::BOLD)
                } else if val <= -0.30 {
                    Style::default().fg(Color::Rgb(255, 120, 120))
                } else {
                    Style::default().fg(palette::TEXT_MUTED)
                };

                row_spans.push(Span::styled(val_str, cell_style));
            }
            table_rows.push(Row::new(row_spans));
        }

        let col_constraints = vec![Constraint::Length(9); n + 1];
        let table = Table::new(table_rows, col_constraints).header(header_row);
        f.render_widget(table, v_chunks[0]);

        let beta_title = Line::from(vec![
            Span::styled(" ROLLING BETA (30D vs S&P 500 SPY): ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("(Beta > 1.0 = Aggressive/High Volatility | Beta < 0.0 = Inverse Hedge)", Style::default().fg(palette::TEXT_MUTED)),
        ]);

        let mut beta_spans = Vec::new();
        for (sym, beta) in &corr.betas {
            let col = if *beta > 1.3 {
                palette::BULLISH
            } else if *beta < 0.0 {
                palette::BEARISH
            } else {
                palette::TEXT_PRIMARY
            };

            beta_spans.push(Span::styled(format!("  {}: ", sym), Style::default().fg(palette::TEXT_MUTED)));
            beta_spans.push(Span::styled(format!("{:.2} ", beta), Style::default().fg(col).add_modifier(Modifier::BOLD)));
        }

        let beta_p = Paragraph::new(vec![Line::from(""), beta_title, Line::from(beta_spans)]);
        f.render_widget(beta_p, v_chunks[1]);
    } else {
        let p = Paragraph::new("Calculating Cross-Asset Pearson Correlation Heatmap...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette::TEXT_MUTED));
        f.render_widget(p, inner);
    }
}

// ---------------------------------------------------------------------------
// VIEW 4: SEC FORM 4 SMART MONEY INSIDERS
// ---------------------------------------------------------------------------

fn draw_insiders_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(palette::ACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" SEC FORM 4 SMART MONEY INSIDER TRACKER ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("[INSIDER] (Official SEC EDGAR Stream)", Style::default().fg(palette::ACCENT)),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(inner);

    let header_line = Line::from(vec![
        Span::styled(" INSIDER CONVICTION RADAR: ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Screening out 10b5-1 automated compensation sales to highlight genuine discretionary open-market cash buys.", Style::default().fg(palette::TEXT_MUTED)),
    ]);
    f.render_widget(Paragraph::new(header_line), v_chunks[0]);

    let header_row = Row::new(vec![
        "DATE", "TICKER", "INSIDER NAME", "OFFICER TITLE", "TRANSACTION TYPE", "SHARES", "PRICE", "TOTAL VALUE",
    ])
    .style(Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .insider_trades
        .iter()
        .map(|trade| {
            let type_style = if trade.is_cluster_buy {
                Style::default().bg(palette::BULLISH).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else if trade.is_buy {
                Style::default().fg(palette::BULLISH).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::BEARISH)
            };

            Row::new(vec![
                Span::styled(&trade.filing_date, Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(&trade.symbol, Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(&trade.insider_name, Style::default().fg(palette::TEXT_PRIMARY)),
                Span::styled(&trade.title, Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(&trade.transaction_type, type_style),
                Span::styled(format!("{}", trade.shares), Style::default().fg(palette::TEXT_PRIMARY)),
                Span::styled(format!("${:.2}", trade.price), Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(format!("${:.0}", trade.total_value), Style::default().fg(if trade.is_buy { palette::BULLISH } else { palette::TEXT_MUTED }).add_modifier(Modifier::BOLD)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(8),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(10),
            Constraint::Percentage(8),
            Constraint::Percentage(10),
        ],
    )
    .header(header_row);

    f.render_widget(table, v_chunks[1]);
}

// ---------------------------------------------------------------------------
// VIEW 5: US TREASURY YIELD CURVE & RECESSION RADAR
// ---------------------------------------------------------------------------

fn draw_yield_curve_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(palette::ACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" US SOVEREIGN TREASURY YIELD CURVE & RECESSION PROBABILITY ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("[YCRV] (Estrella-Mishkin Model)", Style::default().fg(palette::ACCENT)),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(yc) = &app.yield_curve {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .split(inner);

        let inv_10y_2y_status = if yc.is_inverted_10y_2y {
            Span::styled(" [INVERTED ⚠️] ", Style::default().bg(palette::BEARISH).fg(Color::Black).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [NORMAL] ", Style::default().bg(palette::BULLISH).fg(Color::Black).add_modifier(Modifier::BOLD))
        };

        let inv_10y_3m_status = if yc.is_inverted_10y_3m {
            Span::styled(" [INVERTED ⚠️] ", Style::default().bg(palette::BEARISH).fg(Color::Black).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [NORMAL] ", Style::default().bg(palette::BULLISH).fg(Color::Black).add_modifier(Modifier::BOLD))
        };

        let filled_bars = (yc.recession_probability / 5.0).round() as usize;
        let empty_bars = 20_usize.saturating_sub(filled_bars);
        let prob_gauge = format!(
            "[{}{}] {:.1}%",
            "█".repeat(filled_bars),
            "░".repeat(empty_bars),
            yc.recession_probability
        );

        let prob_color = if yc.recession_probability > 50.0 {
            palette::BEARISH
        } else if yc.recession_probability > 30.0 {
            palette::ACCENT
        } else {
            palette::BULLISH
        };

        let line1 = Line::from(vec![
            Span::styled(" 10Y - 2Y SPREAD: ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.3}% ", yc.spread_10y_2y), Style::default().fg(if yc.is_inverted_10y_2y { palette::BEARISH } else { palette::BULLISH }).add_modifier(Modifier::BOLD)),
            inv_10y_2y_status,
            Span::raw("    "),
            Span::styled("10Y - 3M SPREAD: ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.3}% ", yc.spread_10y_3m), Style::default().fg(if yc.is_inverted_10y_3m { palette::BEARISH } else { palette::BULLISH }).add_modifier(Modifier::BOLD)),
            inv_10y_3m_status,
        ]);

        let line2 = Line::from(vec![
            Span::styled(" ESTRELLA-MISHKIN 12-MONTH RECESSION PROBABILITY: ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled(prob_gauge, Style::default().fg(prob_color).add_modifier(Modifier::BOLD)),
            Span::styled("  (P(Recession) = Φ(-0.5333 - 0.6330 * (10Y-3M)))", Style::default().fg(palette::TEXT_MUTED)),
        ]);

        let summary_p = Paragraph::new(vec![Line::from(""), line1, line2]);
        f.render_widget(summary_p, v_chunks[0]);

        let curve_points: Vec<(f64, f64)> = yc
            .tenors
            .iter()
            .enumerate()
            .map(|(i, &(_, yield_pct))| (i as f64, yield_pct))
            .collect();

        let dataset = Dataset::default()
            .name("US Sovereign Yield Curve")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(palette::ACTIVE_BORDER))
            .data(&curve_points);

        let min_yield = yc.tenors.iter().map(|t| t.1).fold(f64::MAX, f64::min) - 0.2;
        let max_yield = yc.tenors.iter().map(|t| t.1).fold(f64::MIN, f64::max) + 0.2;

        let x_labels: Vec<Span> = yc
            .tenors
            .iter()
            .map(|t| Span::styled(t.0, Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)))
            .collect();

        let chart = Chart::new(vec![dataset])
            .x_axis(Axis::default().bounds([0.0, 10.0]).labels(x_labels))
            .y_axis(
                Axis::default()
                    .bounds([min_yield, max_yield])
                    .labels(vec![
                        Span::styled(format!("{:.2}%", min_yield), Style::default().fg(palette::TEXT_MUTED)),
                        Span::styled(format!("{:.2}%", (min_yield + max_yield) / 2.0), Style::default().fg(palette::TEXT_MUTED)),
                        Span::styled(format!("{:.2}%", max_yield), Style::default().fg(palette::TEXT_MUTED)),
                    ]),
            );

        f.render_widget(chart, v_chunks[1]);

        let tenor_names: Vec<&str> = yc.tenors.iter().map(|t| t.0).collect();
        let tenor_yields: Vec<String> = yc.tenors.iter().map(|t| format!("{:.3}%", t.1)).collect();

        let header = Row::new(tenor_names).style(Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD));
        let data_row = Row::new(tenor_yields).style(Style::default().fg(palette::TEXT_PRIMARY));

        let table = Table::new(vec![data_row], vec![Constraint::Percentage(9); 11]).header(header);
        f.render_widget(table, v_chunks[2]);
    } else {
        let p = Paragraph::new("Loading US Sovereign Treasury Yield Curve data...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette::TEXT_MUTED));
        f.render_widget(p, inner);
    }
}

// ---------------------------------------------------------------------------
// FOOTER & MODALS
// ---------------------------------------------------------------------------

fn draw_footer(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette::INACTIVE_BORDER));

    let help_line = Line::from(vec![
        Span::styled(" [w / F1-F5] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Tabs  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[Tab] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Focus  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[←/→] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("Scrub Timeline  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[m] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("Mode  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[c] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("Category  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[1-6] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Timeframe  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[/] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Add  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[r] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Refresh  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[q] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Quit", Style::default().fg(palette::TEXT_MUTED)),
    ]);

    let p = Paragraph::new(help_line).alignment(Alignment::Center).block(block);
    f.render_widget(p, area);
}

fn draw_add_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(54, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(palette::ACTIVE_BORDER))
        .title(Line::from(vec![
            Span::styled(" Add Financial Asset / Ticker ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        ]));

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Enter symbol (e.g. AAPL, NVDA, BTC, SOL, ^TNX, GC=F, EURUSD):", Style::default().fg(palette::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}_", app.input_buffer), Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Enter] Confirm   [Esc] Cancel", Style::default().fg(palette::TEXT_MUTED)),
        ]),
    ];

    let p = Paragraph::new(content).block(block);
    f.render_widget(p, popup_area);
}

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

fn format_ts(ts: f64) -> String {
    let secs = ts as i64;
    if let Some(dt) = DateTime::from_timestamp(secs, 0) {
        let local: DateTime<Local> = DateTime::from(dt);
        local.format("%m/%d %H:%M").to_string()
    } else {
        "--".to_string()
    }
}

fn format_relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_len {
        let truncated: String = chars[..max_len].iter().collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
