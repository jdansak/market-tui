// tests/market_test.rs
//! Comprehensive test suite for options max pain, correlation, insider trades, yield curve, and pre-cache.

use market_tui::{
    app::{App, WorkspaceView},
    db::TickerDb,
    cache::{MarketCache, QuoteUpdate, WorkerUpdate},
    correlation::pearson_correlation,
    data::{interpolate_candles_to_dense_line, Candle, MarketSnapshot},
    options::{calculate_max_pain, OptionContract},
    ticker::{AssetCategory, Ticker},
    timeframe::Timeframe,
    yield_curve::calculate_estrella_mishkin_recession_prob,
};
use std::collections::HashMap;

#[test]
fn test_workspace_view_cycling() {
    let v = WorkspaceView::Overview;
    assert_eq!(v.next(), WorkspaceView::Options);
    assert_eq!(v.next().next(), WorkspaceView::Correlation);
    assert_eq!(v.next().next().next(), WorkspaceView::Insiders);
    assert_eq!(v.next().next().next().next(), WorkspaceView::YieldCurve);
    assert_eq!(v.next().next().next().next().next(), WorkspaceView::Overview);
}

#[test]
fn test_options_max_pain_calculation() {
    let calls = vec![
        OptionContract { strike: 90.0, last_price: 11.0, bid: 10.5, ask: 11.5, volume: 100, open_interest: 500, implied_volatility: 30.0, in_the_money: true },
        OptionContract { strike: 100.0, last_price: 3.0, bid: 2.8, ask: 3.2, volume: 200, open_interest: 1500, implied_volatility: 25.0, in_the_money: false },
        OptionContract { strike: 110.0, last_price: 0.5, bid: 0.4, ask: 0.6, volume: 50, open_interest: 800, implied_volatility: 32.0, in_the_money: false },
    ];

    let puts = vec![
        OptionContract { strike: 90.0, last_price: 0.6, bid: 0.5, ask: 0.7, volume: 80, open_interest: 600, implied_volatility: 31.0, in_the_money: false },
        OptionContract { strike: 100.0, last_price: 3.2, bid: 3.0, ask: 3.4, volume: 150, open_interest: 1200, implied_volatility: 26.0, in_the_money: false },
        OptionContract { strike: 110.0, last_price: 10.8, bid: 10.5, ask: 11.1, volume: 90, open_interest: 400, implied_volatility: 34.0, in_the_money: true },
    ];

    let (max_pain, call_oi, put_oi, pc_ratio) = calculate_max_pain(&calls, &puts);
    assert_eq!(max_pain, 100.0, "Max pain should pin around highest concentration strike");
    assert_eq!(call_oi, 2800);
    assert_eq!(put_oi, 2200);
    assert!(pc_ratio > 0.0);
}

#[test]
fn test_pearson_correlation() {
    let x = vec![0.01, 0.02, -0.01, 0.03, 0.01];
    let y = vec![0.02, 0.04, -0.02, 0.06, 0.02];
    let corr = pearson_correlation(&x, &y);
    assert!((corr - 1.0).abs() < 1e-4);

    let inv_y = vec![-0.01, -0.02, 0.01, -0.03, -0.01];
    let inv_corr = pearson_correlation(&x, &inv_y);
    assert!((inv_corr - (-1.0)).abs() < 1e-4);
}

#[test]
fn test_yield_curve_recession_model() {
    let normal_prob = calculate_estrella_mishkin_recession_prob(1.5);
    assert!(normal_prob < 15.0);

    let inverted_prob = calculate_estrella_mishkin_recession_prob(-1.5);
    assert!(inverted_prob > 40.0);
}

#[test]
fn test_ticker_categories_and_normalization() {
    let btc = Ticker::new("BTC");
    assert_eq!(btc.category, AssetCategory::Crypto);
    assert_eq!(btc.symbol, "BTC-USD");

    let tnx = Ticker::new("^TNX");
    assert_eq!(tnx.category, AssetCategory::Bond);

    let dxy = Ticker::new("DXY");
    assert_eq!(dxy.category, AssetCategory::Forex);

    let gold = Ticker::new("GOLD");
    assert_eq!(gold.category, AssetCategory::Commodity);
}

#[test]
fn test_dense_line_interpolation() {
    let candles = vec![
        Candle { timestamp: 100.0, open: 10.0, high: 15.0, low: 9.0, close: 12.0, volume: 1000.0 },
        Candle { timestamp: 200.0, open: 12.0, high: 18.0, low: 11.0, close: 16.0, volume: 2000.0 },
    ];
    let points = interpolate_candles_to_dense_line(&candles);
    assert!(points.len() > 2);
}

#[test]
fn test_market_cache_insert_and_lookup() {
    let mut cache = MarketCache::new();
    let snap = MarketSnapshot {
        candles: vec![Candle { timestamp: 100.0, open: 10.0, high: 12.0, low: 9.0, close: 11.0, volume: 5000.0 }],
        points: vec![(100.0, 11.0)],
        current_price: 11.0,
        previous_close: 10.0,
        high: 12.0,
        low: 9.0,
        volume: 5000.0,
    };

    cache.insert_chart("NVDA".to_string(), Timeframe::OneDay, snap.clone());

    let cached = cache.get_chart("NVDA", Timeframe::OneDay);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().current_price, 11.0);
    assert!(cache.get_chart("NVDA", Timeframe::OneWeek).is_none());
}

#[test]
fn test_app_pre_cache_instant_ticker_switch() {
    let mut app = App::new();
    assert_eq!(app.selected, 0);

    // Insert cached data for AAPL
    let aapl_snap = MarketSnapshot {
        candles: vec![Candle { timestamp: 200.0, open: 220.0, high: 225.0, low: 219.0, close: 224.0, volume: 80000.0 }],
        points: vec![(200.0, 224.0)],
        current_price: 224.0,
        previous_close: 220.0,
        high: 225.0,
        low: 219.0,
        volume: 80000.0,
    };
    app.cache.insert_chart("AAPL".to_string(), Timeframe::OneDay, aapl_snap);

    // Instant switch to index 1 (AAPL) without blocking
    app.select_ticker(1);
    assert_eq!(app.selected, 1);
    assert_eq!(app.candles.len(), 1);
    assert_eq!(app.candles[0].close, 224.0);

    // Apply live quotes update
    let mut quotes = HashMap::new();
    quotes.insert("AAPL".to_string(), QuoteUpdate {
        current_price: 226.5,
        previous_close: 220.0,
        change: 6.5,
        change_percent: 2.95,
        high: 227.0,
        low: 219.0,
        volume: 120000.0,
    });
    app.apply_update(WorkerUpdate::Quotes(quotes));

    assert_eq!(app.watchlist[1].current_price, 226.5);
    assert_eq!(app.watchlist[1].change, 6.5);
}

#[test]
fn test_sqlite_db_seed_add_remove_and_reopen() {
    let db_path = std::env::temp_dir().join(format!(
        "market_tui_test_{}_{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&db_path);

    let defaults = vec![Ticker::new("NVDA"), Ticker::new("BTC")];
    {
        let db = TickerDb::open_path(&db_path).expect("open db");
        let loaded = db.load_or_seed(&defaults).expect("seed defaults");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].symbol, "NVDA");
        assert_eq!(loaded[1].symbol, "BTC-USD");

        db.add_ticker(&Ticker::new("AMD")).expect("add AMD");
        assert!(db.remove_ticker("NVDA").expect("remove NVDA"));
    }

    // Reopen from disk and verify persistence without re-seeding NVDA
    {
        let db = TickerDb::open_path(&db_path).expect("reopen db");
        let loaded = db.load_or_seed(&defaults).expect("load persisted");
        let symbols: Vec<&str> = loaded.iter().map(|t| t.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["BTC-USD", "AMD"]);

        // Removing all tickers should stay empty on next load (not re-seed)
        assert!(db.remove_ticker("BTC-USD").unwrap());
        assert!(db.remove_ticker("AMD").unwrap());
        let empty = db.load_or_seed(&defaults).unwrap();
        assert!(empty.is_empty());
    }

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_app_with_sqlite_db_persists_add_and_delete() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use tokio::sync::mpsc;

    let db = TickerDb::open_in_memory().expect("in-memory db");
    let (tx, _rx) = mpsc::unbounded_channel();
    let mut app = App::with_db_and_channels(Some(db), tx);
    let initial_len = app.watchlist.len();
    assert!(initial_len >= 5);

    // Open add modal, type PLTR, press Enter
    let press = |code| Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Press));
    app.handle_event(press(KeyCode::Char('a'))).unwrap();
    for ch in "PLTR".chars() {
        app.handle_event(press(KeyCode::Char(ch))).unwrap();
    }
    app.handle_event(press(KeyCode::Enter)).unwrap();

    assert_eq!(app.watchlist.len(), initial_len + 1);
    assert_eq!(app.selected_ticker().unwrap().symbol, "PLTR");

    let persisted = app.db.as_ref().unwrap().list_tickers().unwrap();
    assert!(persisted.iter().any(|t| t.symbol == "PLTR"));

    // Delete PLTR via 'd'
    app.handle_event(press(KeyCode::Char('d'))).unwrap();
    assert_eq!(app.watchlist.len(), initial_len);
    let persisted_after = app.db.as_ref().unwrap().list_tickers().unwrap();
    assert!(!persisted_after.iter().any(|t| t.symbol == "PLTR"));
}
