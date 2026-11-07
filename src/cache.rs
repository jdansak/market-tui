#![allow(dead_code)]
// src/cache.rs
//! High-performance in-memory market cache and continuous asynchronous background pre-fetch worker.

use crate::{
    correlation::{compute_correlation_matrix, CorrelationAnalysis},
    data::{self, MarketSnapshot, NewsItem},
    insider::{fetch_insider_transactions, InsiderTransaction},
    options::{fetch_options_chain, OptionsChain},
    ticker::Ticker,
    timeframe::Timeframe,
    yield_curve::{fetch_yield_curve, YieldCurveSnapshot},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct QuoteUpdate {
    pub current_price: f64,
    pub previous_close: f64,
    pub change: f64,
    pub change_percent: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub enum WorkerUpdate {
    ChartSnapshot {
        symbol: String,
        timeframe: Timeframe,
        snapshot: MarketSnapshot,
    },
    Quotes(HashMap<String, QuoteUpdate>),
    News {
        symbol: String,
        news: Vec<NewsItem>,
    },
    Options {
        symbol: String,
        options: OptionsChain,
    },
    Insiders {
        symbol: String,
        trades: Vec<InsiderTransaction>,
    },
    YieldCurve(YieldCurveSnapshot),
    Correlation(CorrelationAnalysis),
}

#[derive(Debug, Clone)]
pub enum WorkerRequest {
    PrioritizeTicker {
        ticker: Ticker,
        timeframe: Timeframe,
    },
    RefreshAll,
    AddTicker(Ticker),
    RemoveTicker(String),
}

#[derive(Debug, Clone, Default)]
pub struct MarketCache {
    charts: HashMap<(String, Timeframe), (MarketSnapshot, Instant)>,
    quotes: HashMap<String, (QuoteUpdate, Instant)>,
    news: HashMap<String, (Vec<NewsItem>, Instant)>,
    options: HashMap<String, (OptionsChain, Instant)>,
    insiders: HashMap<String, (Vec<InsiderTransaction>, Instant)>,
    yield_curve: Option<(YieldCurveSnapshot, Instant)>,
    correlation: Option<(CorrelationAnalysis, Instant)>,
}

impl MarketCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_chart(&mut self, symbol: String, tf: Timeframe, snap: MarketSnapshot) {
        self.charts.insert((symbol, tf), (snap, Instant::now()));
    }

    pub fn get_chart(&self, symbol: &str, tf: Timeframe) -> Option<&MarketSnapshot> {
        self.charts.get(&(symbol.to_string(), tf)).map(|(s, _)| s)
    }

    pub fn insert_quote(&mut self, symbol: String, quote: QuoteUpdate) {
        self.quotes.insert(symbol, (quote, Instant::now()));
    }

    pub fn get_quote(&self, symbol: &str) -> Option<&QuoteUpdate> {
        self.quotes.get(symbol).map(|(q, _)| q)
    }

    pub fn insert_news(&mut self, symbol: String, news: Vec<NewsItem>) {
        self.news.insert(symbol, (news, Instant::now()));
    }

    pub fn get_news(&self, symbol: &str) -> Option<&Vec<NewsItem>> {
        self.news.get(symbol).map(|(n, _)| n)
    }

    pub fn insert_options(&mut self, symbol: String, opt: OptionsChain) {
        self.options.insert(symbol, (opt, Instant::now()));
    }

    pub fn get_options(&self, symbol: &str) -> Option<&OptionsChain> {
        self.options.get(symbol).map(|(o, _)| o)
    }

    pub fn insert_insiders(&mut self, symbol: String, trades: Vec<InsiderTransaction>) {
        self.insiders.insert(symbol, (trades, Instant::now()));
    }

    pub fn get_insiders(&self, symbol: &str) -> Option<&Vec<InsiderTransaction>> {
        self.insiders.get(symbol).map(|(t, _)| t)
    }

    pub fn insert_yield_curve(&mut self, yc: YieldCurveSnapshot) {
        self.yield_curve = Some((yc, Instant::now()));
    }

    pub fn get_yield_curve(&self) -> Option<&YieldCurveSnapshot> {
        self.yield_curve.as_ref().map(|(y, _)| y)
    }

    pub fn insert_correlation(&mut self, corr: CorrelationAnalysis) {
        self.correlation = Some((corr, Instant::now()));
    }

    pub fn get_correlation(&self) -> Option<&CorrelationAnalysis> {
        self.correlation.as_ref().map(|(c, _)| c)
    }
}

pub fn quote_from_snapshot(snap: &MarketSnapshot) -> QuoteUpdate {
    let change = snap.current_price - snap.previous_close;
    let change_percent = if snap.previous_close.abs() > 0.0001 {
        (change / snap.previous_close) * 100.0
    } else {
        0.0
    };
    QuoteUpdate {
        current_price: snap.current_price,
        previous_close: snap.previous_close,
        change,
        change_percent,
        high: snap.high,
        low: snap.low,
        volume: snap.volume,
    }
}

pub async fn refresh_quotes_batch(watchlist: &[Ticker], tx: &mpsc::UnboundedSender<WorkerUpdate>) {
    let mut futures = Vec::new();
    for t in watchlist {
        let t_clone = t.clone();
        futures.push(async move {
            if let Ok(snap) = data::fetch_chart_data(&t_clone, Timeframe::OneDay).await {
                Some((t_clone.symbol.clone(), quote_from_snapshot(&snap)))
            } else {
                None
            }
        });
    }

    let results = futures::future::join_all(futures).await;
    let mut quotes = HashMap::new();
    for r in results.into_iter().flatten() {
        quotes.insert(r.0, r.1);
    }

    if !quotes.is_empty() {
        let _ = tx.send(WorkerUpdate::Quotes(quotes));
    }
}

pub fn start_background_worker(
    initial_watchlist: Vec<Ticker>,
    mut rx_req: mpsc::UnboundedReceiver<WorkerRequest>,
    tx_update: mpsc::UnboundedSender<WorkerUpdate>,
) {
    tokio::spawn(async move {
        let mut watchlist = initial_watchlist;
        let mut quote_interval = tokio::time::interval(Duration::from_secs(8));
        let mut macro_interval = tokio::time::interval(Duration::from_secs(60));

        // Initial eager pre-fetch of macro data (Yield curve + Correlation)
        let tx_macro = tx_update.clone();
        let top_tickers = watchlist.iter().take(8).cloned().collect::<Vec<_>>();
        tokio::spawn(async move {
            if let Ok(yc) = fetch_yield_curve().await {
                let _ = tx_macro.send(WorkerUpdate::YieldCurve(yc));
            }
            let mut asset_candles = Vec::new();
            for t in top_tickers {
                if let Ok(snap) = data::fetch_chart_data(&t, Timeframe::OneMonth).await {
                    asset_candles.push((t.symbol.clone(), snap.candles));
                }
            }
            if !asset_candles.is_empty() {
                let corr = compute_correlation_matrix(&asset_candles, "SPY");
                let _ = tx_macro.send(WorkerUpdate::Correlation(corr));
            }
        });

        // Initial batch quotes refresh
        let tx_init = tx_update.clone();
        let init_list = watchlist.clone();
        tokio::spawn(async move {
            refresh_quotes_batch(&init_list, &tx_init).await;
        });

        loop {
            tokio::select! {
                // High-priority user request (e.g. user selected new ticker or changed timeframe)
                Some(req) = rx_req.recv() => {
                    match req {
                        WorkerRequest::PrioritizeTicker { ticker, timeframe } => {
                            let tx = tx_update.clone();
                            let t = ticker.clone();
                            // Spawn immediate concurrent fetch for active ticker assets
                            tokio::spawn(async move {
                                // 1. Active chart snapshot (highest priority)
                                if let Ok(snap) = data::fetch_chart_data(&t, timeframe).await {
                                    let _ = tx.send(WorkerUpdate::ChartSnapshot {
                                        symbol: t.symbol.clone(),
                                        timeframe,
                                        snapshot: snap,
                                    });
                                }

                                // 2. News
                                let news = data::fetch_ticker_news(&t.symbol).await;
                                let _ = tx.send(WorkerUpdate::News {
                                    symbol: t.symbol.clone(),
                                    news,
                                });

                                // 3. Options
                                let cur_price = t.current_price;
                                if let Ok(opt) = fetch_options_chain(&t.symbol, cur_price).await {
                                    let _ = tx.send(WorkerUpdate::Options {
                                        symbol: t.symbol.clone(),
                                        options: opt,
                                    });
                                }

                                // 4. Insiders
                                if let Ok(ins) = fetch_insider_transactions(&t.symbol).await {
                                    let _ = tx.send(WorkerUpdate::Insiders {
                                        symbol: t.symbol.clone(),
                                        trades: ins,
                                    });
                                }

                                // 5. Pre-cache all other timeframes for this ticker so 1-6 are instant!
                                for &other_tf in &Timeframe::ALL {
                                    if other_tf != timeframe {
                                        if let Ok(other_snap) = data::fetch_chart_data(&t, other_tf).await {
                                            let _ = tx.send(WorkerUpdate::ChartSnapshot {
                                                symbol: t.symbol.clone(),
                                                timeframe: other_tf,
                                                snapshot: other_snap,
                                            });
                                        }
                                    }
                                }
                            });
                        }
                        WorkerRequest::RefreshAll => {
                            let tx = tx_update.clone();
                            let list = watchlist.clone();
                            tokio::spawn(async move {
                                refresh_quotes_batch(&list, &tx).await;
                            });
                        }
                        WorkerRequest::AddTicker(t) => {
                            if !watchlist.iter().any(|existing| existing.symbol == t.symbol) {
                                watchlist.push(t.clone());
                                let tx = tx_update.clone();
                                tokio::spawn(async move {
                                    if let Ok(snap) = data::fetch_chart_data(&t, Timeframe::OneDay).await {
                                        let mut map = HashMap::new();
                                        map.insert(t.symbol.clone(), quote_from_snapshot(&snap));
                                        let _ = tx.send(WorkerUpdate::Quotes(map));
                                        let _ = tx.send(WorkerUpdate::ChartSnapshot {
                                            symbol: t.symbol.clone(),
                                            timeframe: Timeframe::OneDay,
                                            snapshot: snap,
                                        });
                                    }
                                });
                            }
                        }
                        WorkerRequest::RemoveTicker(symbol) => {
                            watchlist.retain(|t| t.symbol != symbol);
                        }
                    }
                }

                // Routine background quote polling (every 8 seconds)
                _ = quote_interval.tick() => {
                    let tx = tx_update.clone();
                    let list = watchlist.clone();
                    tokio::spawn(async move {
                        refresh_quotes_batch(&list, &tx).await;
                    });
                }

                // Routine background macro polling (every 60 seconds)
                _ = macro_interval.tick() => {
                    let tx = tx_update.clone();
                    let top_tickers = watchlist.iter().take(8).cloned().collect::<Vec<_>>();
                    tokio::spawn(async move {
                        if let Ok(yc) = fetch_yield_curve().await {
                            let _ = tx.send(WorkerUpdate::YieldCurve(yc));
                        }
                        let mut asset_candles = Vec::new();
                        for t in top_tickers {
                            if let Ok(snap) = data::fetch_chart_data(&t, Timeframe::OneMonth).await {
                                asset_candles.push((t.symbol.clone(), snap.candles));
                            }
                        }
                        if !asset_candles.is_empty() {
                            let corr = compute_correlation_matrix(&asset_candles, "SPY");
                            let _ = tx.send(WorkerUpdate::Correlation(corr));
                        }
                    });
                }
            }
        }
    });
}
