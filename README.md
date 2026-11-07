# Market TUI (`market_tui`)

A high-performance, polished Terminal User Interface (TUI) application written in **Rust** using `ratatui` and `crossterm` for tracking real-time and historical stock and cryptocurrency market data.

---

## Features

- **Stock & Crypto Support**:
  - Equities & ETFs: `AAPL`, `NVDA`, `MSFT`, `TSLA`, `SPY`, etc.
  - Cryptocurrencies: `BTC` (Bitcoin), `ETH` (Ethereum), `SOL` (Solana), `DOGE` (Dogecoin), etc.
- **6 Historical Timeframes**:
  - `[ 1D ]`: 1 Day intraday price action (5m interval)
  - `[ 1W ]`: 1 Week price action (15m interval)
  - `[ 1M ]`: 1 Month daily close
  - `[ 6M ]`: 6 Months daily close
  - `[ YTD ]`: Year-To-Date daily close
  - `[ ALL ]`: Complete asset lifespan (weekly close)
- **High-Contrast, Accessible Visual Palette**:
  - Built to be easily readable against dark terminal themes.
  - **Electric Cyan** (`#00DCFF`) active panel focus borders.
  - **Steel Slate** (`#46586E`) inactive borders.
  - **Vivid Emerald Green** (`#2ED573`) for positive gains and bullish trends.
  - **Bright Coral Red** (`#FF4757`) for negative losses and bearish trends.
  - **Electric Amber / Gold** (`#FFAA00`) for active timeframes and highlights.
  - High-resolution Braille marker curves (`symbols::Marker::Braille`).
- **Real-Time Data & Offline Resilience**:
  - Queries Yahoo Finance v8 API.
  - Gracefully falls back to realistic synthetic market data when offline or rate-limited.
- **Dynamic Watchlist Management**:
  - Add any stock symbol or crypto coin (`/` or `a`).
  - Delete items from watchlist (`d`).

---

## Keyboard Controls

| Key | Action |
| :--- | :--- |
| `Tab` | Switch focus between Watchlist and Chart |
| `↑` / `↓` or `j` / `k` | Navigate items in Watchlist |
| `1` – `6` | Select timeframe (`1`=1D, `2`=1W, `3`=1M, `4`=6M, `5`=YTD, `6`=ALL) |
| `←` / `→` or `h` / `l` | Previous / Next timeframe |
| `/` or `a` | Open modal to add a new ticker |
| `d` | Remove currently selected ticker from watchlist |
| `r` | Refresh current quote and chart data |
| `q` / `Ctrl+C` | Quit |

---

## Quick Start

```bash
# Run in debug mode
cargo run

# Build optimized release binary
cargo build --release
./target/release/market_tui
```
