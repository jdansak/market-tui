// tests/market_test.rs
//! Comprehensive test suite for tickers, candles, categories, and dense line interpolation.

use market_tui::{
    app::ChartMode,
    data::{interpolate_candles_to_dense_line, Candle},
    ticker::{AssetCategory, Ticker},
    timeframe::Timeframe,
};

#[test]
fn test_ticker_categories_and_normalization() {
    let btc = Ticker::new("BTC");
    assert_eq!(btc.category, AssetCategory::Crypto);
    assert_eq!(btc.symbol, "BTC-USD");
    assert_eq!(btc.display_name, "Bitcoin");

    let tnx = Ticker::new("^TNX");
    assert_eq!(tnx.category, AssetCategory::Bond);
    assert_eq!(tnx.display_name, "US 10-Yr Treasury Yield");

    let dxy = Ticker::new("DXY");
    assert_eq!(dxy.category, AssetCategory::Forex);
    assert_eq!(dxy.symbol, "DX-Y.NYB");

    let gold = Ticker::new("GOLD");
    assert_eq!(gold.category, AssetCategory::Commodity);
    assert_eq!(gold.symbol, "GC=F");

    let nvda = Ticker::new("NVDA");
    assert_eq!(nvda.category, AssetCategory::Stock);
    assert_eq!(nvda.symbol, "NVDA");
}

#[test]
fn test_chart_modes_toggle() {
    let mode = ChartMode::Candlestick;
    assert_eq!(mode.toggle(), ChartMode::SolidLine);
    assert_eq!(mode.toggle().toggle(), ChartMode::Candlestick);
}

#[test]
fn test_dense_line_interpolation() {
    let candles = vec![
        Candle { timestamp: 100.0, open: 10.0, high: 15.0, low: 9.0, close: 12.0, volume: 1000.0 },
        Candle { timestamp: 200.0, open: 12.0, high: 18.0, low: 11.0, close: 16.0, volume: 2000.0 },
    ];

    let points = interpolate_candles_to_dense_line(&candles);
    assert!(points.len() > 2, "Dense interpolation must produce intermediate points");
    assert_eq!(points.first().unwrap().0, 100.0);
    assert_eq!(points.first().unwrap().1, 12.0);
    assert_eq!(points.last().unwrap().0, 200.0);
    assert_eq!(points.last().unwrap().1, 16.0);
}

#[test]
fn test_timeframe_mappings() {
    for tf in Timeframe::ALL {
        assert!(!tf.range().is_empty());
        assert!(!tf.interval().is_empty());
        assert_eq!(tf.next().prev(), tf);
    }
}
