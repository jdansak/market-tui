use anyhow::Result;
use market_tui::app::{App, ChartMode, Focus, WorkspaceView};
use market_tui::timeframe::Timeframe;
use market_tui::ui;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use serde::Serialize;
use std::fs::File;
use std::io::Write;

#[derive(Serialize)]
struct ExportCell {
    x: u16,
    y: u16,
    ch: String,
    fg: [u8; 3],
}

#[derive(Serialize)]
struct FrameDump {
    name: String,
    width: u16,
    height: u16,
    cells: Vec<ExportCell>,
}

fn color_to_rgb(c: Color) -> [u8; 3] {
    match c {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Black => [13, 17, 23],
        Color::Red => [255, 71, 87],
        Color::Green => [46, 213, 115],
        Color::Yellow => [255, 170, 0],
        Color::Blue => [30, 144, 255],
        Color::Magenta => [232, 65, 24],
        Color::Cyan => [0, 220, 255],
        Color::Gray => [139, 155, 180],
        Color::DarkGray => [70, 88, 110],
        Color::LightRed => [255, 107, 129],
        Color::LightGreen => [123, 237, 159],
        Color::LightYellow => [255, 204, 0],
        Color::LightBlue => [79, 195, 247],
        Color::LightMagenta => [255, 121, 198],
        Color::LightCyan => [128, 237, 255],
        Color::White => [241, 245, 249],
        _ => [139, 155, 180],
    }
}

fn dump_terminal(terminal: &Terminal<TestBackend>, name: &str) -> FrameDump {
    let buf = terminal.backend().buffer();
    let width = buf.area.width;
    let height = buf.area.height;
    let mut cells = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if let Some(c) = buf.cell((x, y)) {
                let sym = c.symbol();
                if !sym.trim().is_empty() {
                    cells.push(ExportCell {
                        x,
                        y,
                        ch: sym.to_string(),
                        fg: color_to_rgb(c.fg),
                    });
                }
            }
        }
    }

    FrameDump {
        name: name.to_string(),
        width,
        height,
        cells,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let width = 114;
    let height = 34;

    let mut frames = Vec::new();

    // 1. Overview BTC-USD Candlesticks (default)
    let mut app = App::new();
    let _ = app.refresh_active().await;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "01_overview_btc_candlesticks"));

    // 2. Timeline scrubber inspecting candle
    app.focus = Focus::Chart;
    app.chart_cursor = Some(app.candles.len().saturating_sub(12));
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "02_timeline_scrubber"));

    // 3. 1M Timeframe on NVDA showing 100% full width dynamic stretching
    app.focus = Focus::Watchlist;
    app.selected = 1; // NVDA
    app.timeframe = Timeframe::OneMonth;
    app.chart_cursor = None;
    let _ = app.refresh_active().await;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "03_overview_nvda_1month_full_width"));

    // 4. Solid Line interpolation on AAPL
    app.selected = 2; // AAPL
    app.chart_mode = ChartMode::SolidLine;
    app.timeframe = Timeframe::OneWeek;
    let _ = app.refresh_active().await;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "04_overview_aapl_solid_line"));

    // 5. Workstation Tab 2: Options Chain & Max Pain
    app.current_view = WorkspaceView::Options;
    let _ = app.refresh_active().await;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "05_options_max_pain"));

    // 6. Workstation Tab 3: Cross-Asset Correlation Heatmap & Rolling Beta
    app.current_view = WorkspaceView::Correlation;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "06_correlation_heatmap"));

    // 7. Workstation Tab 4: SEC Form 4 Smart Money Tracker
    app.current_view = WorkspaceView::Insiders;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "07_sec_insider_trades"));

    // 8. Workstation Tab 5: US Treasury Yield Curve & Recession Radar
    app.current_view = WorkspaceView::YieldCurve;
    terminal.draw(|f| ui::draw_ui(f, &app))?;
    frames.push(dump_terminal(&terminal, "08_yield_curve_recession"));

    let json_bytes = serde_json::to_vec_pretty(&frames)?;
    let mut file = File::create("assets/frames.json")?;
    file.write_all(&json_bytes)?;
    println!("Successfully exported {} frames to assets/frames.json", frames.len());

    Ok(())
}
