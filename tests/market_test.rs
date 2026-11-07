// tests/market_test.rs
//! Comprehensive test suite for options max pain, correlation, insider trades, and yield curve.

use market_tui::{
    app::WorkspaceView,
    correlation::pearson_correlation,
    data::{interpolate_candles_to_dense_line, Candle},
    options::{calculate_max_pain, OptionContract},
    ticker::{AssetCategory, Ticker},
    yield_curve::calculate_estrella_mishkin_recession_prob,
};

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
