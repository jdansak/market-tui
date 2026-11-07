// src/app.rs
//! Application state, workstation views, chart modes, and keyboard navigation.

use crate::{
    correlation::{compute_correlation_matrix, CorrelationAnalysis},
    data::{self, Candle, NewsItem},
    insider::{fetch_insider_transactions, InsiderTransaction},
    options::{fetch_options_chain, OptionsChain},
    ticker::{AssetCategory, Ticker},
    timeframe::Timeframe,
    yield_curve::{fetch_yield_curve, YieldCurveSnapshot},
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceView {
    Overview,    // Tab 1: Watchlist, Candlesticks, News
    Options,     // Tab 2: Options Chain & Max Pain
    Correlation, // Tab 3: NxN Correlation Matrix & Beta
    Insiders,    // Tab 4: SEC Form 4 Smart Money Tracker
    YieldCurve,  // Tab 5: US Treasury Yield Curve & Recession Radar
}

impl WorkspaceView {
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceView::Overview => "1: Market Overview",
            WorkspaceView::Options => "2: Options & Max Pain",
            WorkspaceView::Correlation => "3: Cross-Asset Correlation",
            WorkspaceView::Insiders => "4: SEC Form 4 Insiders",
            WorkspaceView::YieldCurve => "5: Yield Curve & Recession",
        }
    }

    pub fn next(self) -> Self {
        match self {
            WorkspaceView::Overview => WorkspaceView::Options,
            WorkspaceView::Options => WorkspaceView::Correlation,
            WorkspaceView::Correlation => WorkspaceView::Insiders,
            WorkspaceView::Insiders => WorkspaceView::YieldCurve,
            WorkspaceView::YieldCurve => WorkspaceView::Overview,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            WorkspaceView::Overview => WorkspaceView::YieldCurve,
            WorkspaceView::Options => WorkspaceView::Overview,
            WorkspaceView::Correlation => WorkspaceView::Options,
            WorkspaceView::Insiders => WorkspaceView::Correlation,
            WorkspaceView::YieldCurve => WorkspaceView::Insiders,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Watchlist,
    Chart,
    News,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartMode {
    Candlestick,
    SolidLine,
}

impl ChartMode {
    pub fn label(self) -> &'static str {
        match self {
            ChartMode::Candlestick => "Candlesticks (OHLC)",
            ChartMode::SolidLine => "Solid Line (Dense)",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            ChartMode::Candlestick => ChartMode::SolidLine,
            ChartMode::SolidLine => ChartMode::Candlestick,
        }
    }
}

pub struct App {
    pub current_view: WorkspaceView,
    pub watchlist: Vec<Ticker>,
    pub selected: usize,
    pub candles: Vec<Candle>,
    pub chart_data: Vec<(f64, f64)>,
    pub news: Vec<NewsItem>,
    pub news_selected: usize,
    pub chart_cursor: Option<usize>,
    pub timeframe: Timeframe,
    pub focus: Focus,
    pub chart_mode: ChartMode,
    pub show_news: bool,
    pub category_filter: Option<AssetCategory>,
    pub last_refresh: Instant,
    pub should_quit: bool,
    pub show_add_modal: bool,
    pub input_buffer: String,

    // Institutional Analytics State
    pub options_chain: Option<OptionsChain>,
    pub correlation: Option<CorrelationAnalysis>,
    pub insider_trades: Vec<InsiderTransaction>,
    pub yield_curve: Option<YieldCurveSnapshot>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_view: WorkspaceView::Overview,
            watchlist: vec![
                Ticker::new("NVDA"),
                Ticker::new("AAPL"),
                Ticker::new("MSFT"),
                Ticker::new("TSLA"),
                Ticker::new("SPY"),
                Ticker::new("QQQ"),
                Ticker::new("BTC"),
                Ticker::new("ETH"),
                Ticker::new("SOL"),
                Ticker::new("^TNX"),
                Ticker::new("^TYX"),
                Ticker::new("DX-Y.NYB"),
                Ticker::new("EURUSD=X"),
                Ticker::new("GC=F"),
                Ticker::new("CL=F"),
            ],
            selected: 0,
            candles: Vec::new(),
            chart_data: Vec::new(),
            news: Vec::new(),
            news_selected: 0,
            chart_cursor: None,
            timeframe: Timeframe::OneDay,
            focus: Focus::Watchlist,
            chart_mode: ChartMode::Candlestick,
            show_news: true,
            category_filter: None,
            last_refresh: Instant::now() - Duration::from_secs(60),
            should_quit: false,
            show_add_modal: false,
            input_buffer: String::new(),
            options_chain: None,
            correlation: None,
            insider_trades: Vec::new(),
            yield_curve: None,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        self.watchlist
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                if let Some(cat) = self.category_filter {
                    t.category == cat
                } else {
                    true
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected_ticker(&self) -> Option<&Ticker> {
        self.watchlist.get(self.selected)
    }

    pub fn inspected_candle(&self) -> Option<&Candle> {
        if self.candles.is_empty() {
            return None;
        }
        let idx = self.chart_cursor.unwrap_or(self.candles.len().saturating_sub(1));
        self.candles.get(idx)
    }

    pub async fn refresh_active(&mut self) -> Result<()> {
        if let Some(ticker) = self.watchlist.get(self.selected).cloned() {
            let symbol = ticker.symbol.clone();

            // 1. Fetch Chart Snapshot (Candles + Dense Line)
            if let Ok(snap) = data::fetch_chart_data(&ticker, self.timeframe).await {
                self.candles = snap.candles.clone();
                self.chart_data = snap.points;
                self.chart_cursor = None;

                if let Some(t) = self.watchlist.get_mut(self.selected) {
                    t.current_price = snap.current_price;
                    t.change = snap.current_price - snap.previous_close;
                    if snap.previous_close.abs() > 0.0001 {
                        t.change_percent = (t.change / snap.previous_close) * 100.0;
                    }
                    t.high_24h = snap.high;
                    t.low_24h = snap.low;
                    t.volume = snap.volume;
                }
            }

            // 2. Fetch Live Market News
            self.news = data::fetch_ticker_news(&symbol).await;
            self.news_selected = 0;

            // 3. Fetch Options Chain & Max Pain
            let cur_price = self.selected_ticker().map(|t| t.current_price).unwrap_or(150.0);
            if let Ok(opt) = fetch_options_chain(&symbol, cur_price).await {
                self.options_chain = Some(opt);
            }

            // 4. Fetch SEC Form 4 Insider Trades
            if let Ok(trades) = fetch_insider_transactions(&symbol).await {
                self.insider_trades = trades;
            }

            // 5. Fetch US Treasury Yield Curve
            if let Ok(yc) = fetch_yield_curve().await {
                self.yield_curve = Some(yc);
            }

            // 6. Compute Cross-Asset Correlation Matrix
            self.update_correlation_analysis().await;
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    pub async fn update_correlation_analysis(&mut self) {
        let mut asset_candles = Vec::new();
        // Take top 6 active tickers for a clean, readable correlation matrix
        for t in self.watchlist.iter().take(8) {
            if let Ok(snap) = data::fetch_chart_data(t, Timeframe::OneMonth).await {
                asset_candles.push((t.symbol.clone(), snap.candles));
            }
        }
        if !asset_candles.is_empty() {
            self.correlation = Some(compute_correlation_matrix(&asset_candles, "SPY"));
        }
    }

    pub async fn refresh_all_quotes(&mut self) {
        for t in &mut self.watchlist {
            if let Ok(snap) = data::fetch_chart_data(t, Timeframe::OneDay).await {
                t.current_price = snap.current_price;
                t.change = snap.current_price - snap.previous_close;
                if snap.previous_close.abs() > 0.0001 {
                    t.change_percent = (t.change / snap.previous_close) * 100.0;
                }
                t.high_24h = snap.high;
                t.low_24h = snap.low;
                t.volume = snap.volume;
            }
        }
    }

    pub async fn handle_event(&mut self, ev: Event) -> Result<()> {
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            if self.show_add_modal {
                self.handle_modal_key(key).await?;
                return Ok(());
            }

            // Workstation Tab Switching: 'w' cycles views, F1-F5 jumps directly
            match key.code {
                KeyCode::Char('w') => {
                    self.current_view = self.current_view.next();
                    return Ok(());
                }
                KeyCode::Char('W') => {
                    self.current_view = self.current_view.prev();
                    return Ok(());
                }
                KeyCode::F(1) => {
                    self.current_view = WorkspaceView::Overview;
                    return Ok(());
                }
                KeyCode::F(2) => {
                    self.current_view = WorkspaceView::Options;
                    return Ok(());
                }
                KeyCode::F(3) => {
                    self.current_view = WorkspaceView::Correlation;
                    return Ok(());
                }
                KeyCode::F(4) => {
                    self.current_view = WorkspaceView::Insiders;
                    return Ok(());
                }
                KeyCode::F(5) => {
                    self.current_view = WorkspaceView::YieldCurve;
                    return Ok(());
                }
                KeyCode::Char('m') => {
                    self.chart_mode = self.chart_mode.toggle();
                    return Ok(());
                }
                KeyCode::Char('n') => {
                    self.show_news = !self.show_news;
                    return Ok(());
                }
                KeyCode::Char('c') => {
                    self.cycle_category_filter();
                    return Ok(());
                }
                _ => {}
            }

            // Route keys based on active view and focus
            match self.current_view {
                WorkspaceView::Overview => match self.focus {
                    Focus::Watchlist => self.handle_watchlist_key(key).await?,
                    Focus::Chart => self.handle_chart_key(key).await?,
                    Focus::News => self.handle_news_key(key).await?,
                },
                _ => self.handle_non_overview_key(key).await?,
            }
        }
        Ok(())
    }

    fn cycle_category_filter(&mut self) {
        use AssetCategory::*;
        self.category_filter = match self.category_filter {
            None => Some(Stock),
            Some(Stock) => Some(Crypto),
            Some(Crypto) => Some(Bond),
            Some(Bond) => Some(Forex),
            Some(Forex) => Some(Commodity),
            Some(Commodity) => None,
        };

        let indices = self.filtered_indices();
        if !indices.contains(&self.selected) {
            if let Some(&first) = indices.first() {
                self.selected = first;
            }
        }
    }

    async fn handle_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.show_add_modal = false;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                let trimmed = self.input_buffer.trim();
                if !trimmed.is_empty() {
                    let new_ticker = Ticker::new(trimmed);
                    self.watchlist.push(new_ticker);
                    self.selected = self.watchlist.len() - 1;
                    self.input_buffer.clear();
                    self.show_add_modal = false;
                    self.refresh_active().await.ok();
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                if self.input_buffer.len() < 14 {
                    self.input_buffer.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_watchlist_key(&mut self, key: KeyEvent) -> Result<()> {
        let indices = self.filtered_indices();
        let current_pos = indices.iter().position(|&idx| idx == self.selected).unwrap_or(0);

        match key.code {
            KeyCode::Tab => {
                self.focus = Focus::Chart;
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if current_pos + 1 < indices.len() {
                    self.selected = indices[current_pos + 1];
                    self.refresh_active().await.ok();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if current_pos > 0 {
                    self.selected = indices[current_pos - 1];
                    self.refresh_active().await.ok();
                }
            }
            KeyCode::Char('d') => {
                if !self.watchlist.is_empty() && self.selected < self.watchlist.len() {
                    self.watchlist.remove(self.selected);
                    if self.selected >= self.watchlist.len() && self.selected > 0 {
                        self.selected -= 1;
                    }
                    self.refresh_active().await.ok();
                }
            }
            KeyCode::Char('r') => {
                self.refresh_active().await.ok();
            }
            KeyCode::Char('/') | KeyCode::Char('a') => {
                self.show_add_modal = true;
                self.input_buffer.clear();
            }
            KeyCode::Char(c) if ('1'..='6').contains(&c) => {
                if let Some(tf) = Timeframe::from_digit(c) {
                    self.timeframe = tf;
                    self.refresh_active().await.ok();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_chart_key(&mut self, key: KeyEvent) -> Result<()> {
        let candle_count = self.candles.len();
        let max_idx = candle_count.saturating_sub(1);
        let cur = self.chart_cursor.unwrap_or(max_idx);

        match key.code {
            KeyCode::Tab => {
                self.focus = if self.show_news { Focus::News } else { Focus::Watchlist };
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if cur > 0 {
                    self.chart_cursor = Some(cur - 1);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if cur < max_idx {
                    self.chart_cursor = Some(cur + 1);
                }
            }
            KeyCode::Home => {
                self.chart_cursor = Some(0);
            }
            KeyCode::End | KeyCode::Esc => {
                self.chart_cursor = Some(max_idx);
            }
            KeyCode::Char(c) if ('1'..='6').contains(&c) => {
                if let Some(tf) = Timeframe::from_digit(c) {
                    self.timeframe = tf;
                    self.refresh_active().await.ok();
                }
            }
            KeyCode::Char('r') => {
                self.refresh_active().await.ok();
            }
            KeyCode::Char('/') | KeyCode::Char('a') => {
                self.show_add_modal = true;
                self.input_buffer.clear();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_news_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.focus = Focus::Watchlist;
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.news_selected > 0 {
                    self.news_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.news.is_empty() && self.news_selected + 1 < self.news.len() {
                    self.news_selected += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some(item) = self.news.get(self.news_selected) {
                    if !item.link.is_empty() {
                        open_url_in_browser(&item.link);
                    }
                }
            }
            KeyCode::Char('r') => {
                self.refresh_active().await.ok();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_non_overview_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('r') => {
                self.refresh_active().await.ok();
            }
            KeyCode::Tab => {
                self.current_view = self.current_view.next();
            }
            KeyCode::Esc => {
                self.current_view = WorkspaceView::Overview;
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn open_url_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
}
