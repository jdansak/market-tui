// src/ui.rs
//! Polished, high-contrast UI rendering with Candlestick and Solid Line charting engines.

use crate::{
    app::{App, ChartMode, Focus},
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
        Widget,
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
    pub const TAG_STOCK: Color = Color::Rgb(75, 123, 236); // Royal Blue
    pub const TAG_CRYPTO: Color = Color::Rgb(165, 94, 234); // Soft Purple
    pub const TAG_BOND: Color = Color::Rgb(43, 203, 186); // Teal Turquoise
    pub const TAG_FOREX: Color = Color::Rgb(253, 150, 68); // Warm Orange
    pub const TAG_COMMODITY: Color = Color::Rgb(235, 94, 40); // Copper / Bronze
}

pub fn draw_ui(f: &mut Frame, app: &App) {
    let area = f.area();

    // Vertical layout: Main content vs Bottom Help Bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(3)])
        .split(area);

    // Horizontal layout: Left Watchlist vs Right Body
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(40)])
        .split(main_chunks[0]);

    draw_watchlist(f, app, body_chunks[0]);
    draw_right_panel(f, app, body_chunks[1]);
    draw_footer(f, app, main_chunks[1]);

    if app.show_add_modal {
        draw_add_modal(f, app, area);
    }
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
            Constraint::Length(5), // Header & Quote stats
            Constraint::Length(3), // Timeframe pills & Chart Mode
            Constraint::Min(9),    // Chart
            Constraint::Length(6), // Live News Feed
        ]
    } else {
        vec![
            Constraint::Length(5), // Header & Quote stats
            Constraint::Length(3), // Timeframe pills & Chart Mode
            Constraint::Min(12),   // Chart full height
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
            Span::styled(" Timeframe (1-6) | Chart Mode: ", Style::default().fg(palette::TEXT_MUTED)),
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

    let title = Line::from(vec![
        Span::styled(" Market Chart ", Style::default().fg(palette::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("[{}] ", app.timeframe.label()),
            Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Mode: {} (Press 'm' to toggle)", app.chart_mode.label()),
            Style::default().fg(palette::TEXT_MUTED),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.focus == Focus::Chart { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color))
        .title(title);

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
            };
            f.render_widget(candle_widget, inner);
        }
        ChartMode::SolidLine => {
            draw_dense_solid_line_chart(f, app, inner);
        }
    }
}

/// Custom Terminal Candlestick Widget rendering real wicks and solid bodies.
struct CandlestickWidget<'a> {
    candles: &'a [Candle],
}

impl<'a> Widget for CandlestickWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 4 || self.candles.is_empty() {
            return;
        }

        // Reserve 2 rows at the bottom for Volume Histogram + Time Labels
        let chart_height = (area.height.saturating_sub(2)) as usize;
        let chart_width = area.width.saturating_sub(10) as usize; // reserve 10 cols for price scale

        // Calculate global min and max prices
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

        // Render Y-Axis Price Scale on the right
        let top_label = format!("${:.2}", max_price);
        let mid_label = format!("${:.2}", (max_price + min_price) / 2.0);
        let bot_label = format!("${:.2}", min_price);

        buf.set_string(area.x + area.width - 9, area.y, &top_label, Style::default().fg(palette::TEXT_MUTED));
        buf.set_string(area.x + area.width - 9, area.y + (chart_height as u16 / 2), &mid_label, Style::default().fg(palette::TEXT_MUTED));
        buf.set_string(area.x + area.width - 9, area.y + chart_height as u16 - 1, &bot_label, Style::default().fg(palette::TEXT_MUTED));

        // Determine candle spacing: 2 columns per candle (1 candle char + 1 space)
        let candle_stride = 2;
        let max_visible_candles = chart_width / candle_stride;
        let visible_candles = if self.candles.len() > max_visible_candles {
            &self.candles[self.candles.len() - max_visible_candles..]
        } else {
            self.candles
        };

        // Render each candlestick
        for (idx, candle) in visible_candles.iter().enumerate() {
            let col = area.x + (idx * candle_stride) as u16;
            if col >= area.x + chart_width as u16 {
                break;
            }

            let is_bullish = candle.close >= candle.open;
            let candle_color = if is_bullish { palette::BULLISH } else { palette::BEARISH };
            let style = Style::default().fg(candle_color).add_modifier(Modifier::BOLD);

            // Map prices to row indices (row 0 = top/high price, row chart_height - 1 = bottom/low price)
            let high_row = ((max_price - candle.high) / price_range * (chart_height - 1) as f64).round() as usize;
            let low_row = ((max_price - candle.low) / price_range * (chart_height - 1) as f64).round() as usize;

            let body_top_price = candle.open.max(candle.close);
            let body_bot_price = candle.open.min(candle.close);

            let body_top_row = ((max_price - body_top_price) / price_range * (chart_height - 1) as f64).round() as usize;
            let body_bot_row = ((max_price - body_bot_price) / price_range * (chart_height - 1) as f64).round() as usize;

            // Draw upper wick (from high down to body top)
            for r in high_row..body_top_row {
                let cell_y = area.y + r as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(col, cell_y, "│", style);
                }
            }

            // Draw candle body (from body top down to body bot)
            if body_top_row == body_bot_row {
                // Doji / flat candle
                let cell_y = area.y + body_top_row as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(col, cell_y, "┼", style);
                }
            } else {
                for r in body_top_row..=body_bot_row {
                    let cell_y = area.y + r as u16;
                    if cell_y < area.y + chart_height as u16 {
                        buf.set_string(col, cell_y, "█", style);
                    }
                }
            }

            // Draw lower wick (from body bot down to low)
            for r in (body_bot_row + 1)..=low_row {
                let cell_y = area.y + r as u16;
                if cell_y < area.y + chart_height as u16 {
                    buf.set_string(col, cell_y, "│", style);
                }
            }

            // Draw Volume Bar at the bottom row of chart
            let vol_row = area.y + chart_height as u16;
            let vol_frac = (candle.volume / max_vol).clamp(0.0, 1.0);
            let vol_char = match (vol_frac * 8.0).round() as u8 {
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
            buf.set_string(col, vol_row, vol_char, Style::default().fg(candle_color));
        }

        // Draw Time labels at the very bottom
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
        Span::styled("('n' to toggle)", Style::default().fg(palette::TEXT_MUTED)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if app.focus == Focus::News { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color))
        .title(title);

    let items: Vec<ListItem> = app
        .news
        .iter()
        .take(4)
        .map(|item| {
            let relative_time = format_relative_time(item.published_time);
            let line = Line::from(vec![
                Span::styled(" • ", Style::default().fg(palette::ACCENT)),
                Span::styled(format!("{}: ", item.publisher), Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(format!("{:<60} ", truncate(&item.title, 65)), Style::default().fg(palette::TEXT_PRIMARY)),
                Span::styled(format!("({})", relative_time), Style::default().fg(palette::TEXT_MUTED)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette::INACTIVE_BORDER));

    let help_line = Line::from(vec![
        Span::styled(" [Tab] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Focus  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[m] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("Chart Mode  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[c] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("Category  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[n] ", Style::default().fg(palette::ACTIVE_BORDER).add_modifier(Modifier::BOLD)),
        Span::styled("News  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[1-6] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Timeframe  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[/] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Add  ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled("[d] ", Style::default().fg(palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Delete  ", Style::default().fg(palette::TEXT_MUTED)),
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
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
