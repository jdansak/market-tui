// src/correlation.rs
//! Cross-asset Pearson correlation matrix and rolling Beta analytics.

use crate::data::Candle;

#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationAnalysis {
    pub symbols: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
    pub betas: Vec<(String, f64)>, // (symbol, beta_vs_benchmark)
}

pub fn compute_correlation_matrix(
    assets: &[(String, Vec<Candle>)],
    benchmark_symbol: &str,
) -> CorrelationAnalysis {
    let mut symbols = Vec::new();
    let mut return_series: Vec<Vec<f64>> = Vec::new();

    for (sym, candles) in assets {
        if candles.len() >= 5 {
            let mut returns = Vec::new();
            for window in candles.windows(2) {
                let prev = window[0].close;
                let cur = window[1].close;
                if prev > 0.0 {
                    returns.push((cur - prev) / prev);
                }
            }
            if !returns.is_empty() {
                symbols.push(sym.clone());
                return_series.push(returns);
            }
        }
    }

    let n = symbols.len();
    if n == 0 {
        return CorrelationAnalysis {
            symbols: Vec::new(),
            matrix: Vec::new(),
            betas: Vec::new(),
        };
    }

    // Align length to minimum common returns count
    let min_len = return_series.iter().map(|r| r.len()).min().unwrap_or(0);
    for r in &mut return_series {
        if r.len() > min_len {
            *r = r[r.len() - min_len..].to_vec();
        }
    }

    // Compute NxN Pearson correlation matrix
    let mut matrix = vec![vec![1.0; n]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let corr = pearson_correlation(&return_series[i], &return_series[j]);
            matrix[i][j] = corr;
            matrix[j][i] = corr;
        }
    }

    // Compute Beta vs benchmark (e.g. SPY or first asset)
    let bench_idx = symbols
        .iter()
        .position(|s| s == benchmark_symbol || s.starts_with("SPY"))
        .unwrap_or(0);

    let bench_returns = &return_series[bench_idx];
    let bench_var = variance(bench_returns);

    let mut betas = Vec::new();
    for i in 0..n {
        let beta = if bench_var > 1e-8 {
            covariance(&return_series[i], bench_returns) / bench_var
        } else {
            1.0
        };
        betas.push((symbols[i].clone(), beta));
    }

    CorrelationAnalysis {
        symbols,
        matrix,
        betas,
    }
}

fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().sum::<f64>() / (data.len() as f64)
}

fn variance(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let m = mean(data);
    data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / ((data.len() - 1) as f64)
}

fn covariance(x: &[f64], y: &[f64]) -> f64 {
    let len = x.len().min(y.len());
    if len < 2 {
        return 0.0;
    }
    let mx = mean(&x[..len]);
    let my = mean(&y[..len]);

    let mut cov = 0.0;
    for i in 0..len {
        cov += (x[i] - mx) * (y[i] - my);
    }
    cov / ((len - 1) as f64)
}

pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let len = x.len().min(y.len());
    if len < 2 {
        return 0.0;
    }
    let cov = covariance(x, y);
    let var_x = variance(x);
    let var_y = variance(y);

    if var_x <= 1e-10 || var_y <= 1e-10 {
        return 0.0;
    }

    (cov / (var_x.sqrt() * var_y.sqrt())).clamp(-1.0, 1.0)
}
