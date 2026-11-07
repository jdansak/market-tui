// src/app.rs
//! Application state, chart modes, category filters, and keyboard navigation.

use crate::{
    data::{self, Candle, NewsItem},
    ticker::{AssetCategory, Ticker},
    timeframe::Timeframe,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::{Duration, Instant};

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
    pub watchlist: Vec<Ticker>,
    pub selected: usize,
    pub candles: Vec<Candle>,
    pub chart_data: Vec<(f64, f64)>,
    pub news: Vec<NewsItem>,
    pub timeframe: Timeframe,
    pub focus: Focus,
    pub chart_mode: ChartMode,
    pub show_news: bool,
    pub category_filter: Option<AssetCategory>,
    pub last_refresh: Instant,
    pub should_quit: bool,
    pub show_add_modal: bool,
    pub input_buffer: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            watchlist: vec![
                // Crypto
                Ticker::new("BTC"),
                Ticker::new("ETH"),
                Ticker::new("SOL"),
                // Equities
                Ticker::new("NVDA"),
                Ticker::new("AAPL"),
                Ticker::new("MSFT"),
                Ticker::new("TSLA"),
                Ticker::new("SPY"),
                Ticker::new("QQQ"),
                // Macro & Yields
                Ticker::new("^TNX"),
                Ticker::new("^TYX"),
                // Currencies & Commodities
                Ticker::new("DX-Y.NYB"),
                Ticker::new("EURUSD=X"),
                Ticker::new("GC=F"),
                Ticker::new("CL=F"),
            ],
            selected: 0,
            candles: Vec::new(),
            chart_data: Vec::new(),
            news: Vec::new(),
            timeframe: Timeframe::OneDay,
            focus: Focus::Watchlist,
            chart_mode: ChartMode::Candlestick,
            show_news: true,
            category_filter: None,
            last_refresh: Instant::now() - Duration::from_secs(60),
            should_quit: false,
            show_add_modal: false,
            input_buffer: String::new(),
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

    pub async fn refresh_active(&mut self) -> Result<()> {
        if let Some(ticker) = self.watchlist.get(self.selected).cloned() {
            let symbol = ticker.symbol.clone();

            // Fetch Chart Snapshot (Candles + Dense Interpolated Line)
            if let Ok(snap) = data::fetch_chart_data(&ticker, self.timeframe).await {
                self.candles = snap.candles;
                self.chart_data = snap.points;

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

            // Fetch Real-time News
            self.news = data::fetch_ticker_news(&symbol).await;
        }
        self.last_refresh = Instant::now();
        Ok(())
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

            // Global Shortcuts
            match key.code {
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

            match self.focus {
                Focus::Watchlist => self.handle_watchlist_key(key).await?,
                Focus::Chart => self.handle_chart_key(key).await?,
                Focus::News => self.handle_news_key(key).await?,
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

        // Ensure selected index is in filtered range
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
        match key.code {
            KeyCode::Tab => {
                self.focus = if self.show_news { Focus::News } else { Focus::Watchlist };
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.timeframe = self.timeframe.prev();
                self.refresh_active().await.ok();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.timeframe = self.timeframe.next();
                self.refresh_active().await.ok();
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
            KeyCode::Char('r') => {
                self.refresh_active().await.ok();
            }
            _ => {}
        }
        Ok(())
    }
}
