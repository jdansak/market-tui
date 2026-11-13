// src/ticker.rs
//! Ticker data models, categories, and metadata.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetCategory {
    Stock,
    Crypto,
    Bond,
    Forex,
    Commodity,
}

impl AssetCategory {
    pub fn from_label(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "CRYPTO" => AssetCategory::Crypto,
            "BOND" => AssetCategory::Bond,
            "FOREX" => AssetCategory::Forex,
            "COMMODITY" => AssetCategory::Commodity,
            _ => AssetCategory::Stock,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AssetCategory::Stock => "STOCK",
            AssetCategory::Crypto => "CRYPTO",
            AssetCategory::Bond => "BOND",
            AssetCategory::Forex => "FOREX",
            AssetCategory::Commodity => "COMMODITY",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    pub symbol: String,
    pub display_name: String,
    pub category: AssetCategory,
    pub current_price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub volume: f64,
}

impl Ticker {
    pub fn new(symbol: &str) -> Self {
        let sym_upper = symbol.trim().to_uppercase();
        let (full_symbol, category, display_name) = match sym_upper.as_str() {
            // Cryptocurrencies
            "BTC" | "BTC-USD" => ("BTC-USD".to_string(), AssetCategory::Crypto, "Bitcoin".to_string()),
            "ETH" | "ETH-USD" => ("ETH-USD".to_string(), AssetCategory::Crypto, "Ethereum".to_string()),
            "SOL" | "SOL-USD" => ("SOL-USD".to_string(), AssetCategory::Crypto, "Solana".to_string()),
            "DOGE" | "DOGE-USD" => ("DOGE-USD".to_string(), AssetCategory::Crypto, "Dogecoin".to_string()),
            "ADA" | "ADA-USD" => ("ADA-USD".to_string(), AssetCategory::Crypto, "Cardano".to_string()),

            // US Treasury Yields & Bonds
            "^TNX" | "TNX" => ("^TNX".to_string(), AssetCategory::Bond, "US 10-Yr Treasury Yield".to_string()),
            "^TYX" | "TYX" => ("^TYX".to_string(), AssetCategory::Bond, "US 30-Yr Treasury Yield".to_string()),
            "^IRX" | "IRX" => ("^IRX".to_string(), AssetCategory::Bond, "US 13-Wk T-Bill Yield".to_string()),

            // Currencies & Forex
            "DX-Y.NYB" | "DXY" => ("DX-Y.NYB".to_string(), AssetCategory::Forex, "US Dollar Index (DXY)".to_string()),
            "EURUSD=X" | "EURUSD" => ("EURUSD=X".to_string(), AssetCategory::Forex, "EUR / USD".to_string()),
            "JPY=X" | "USDJPY" => ("JPY=X".to_string(), AssetCategory::Forex, "USD / JPY".to_string()),
            "GBPUSD=X" | "GBPUSD" => ("GBPUSD=X".to_string(), AssetCategory::Forex, "GBP / USD".to_string()),

            // Commodities
            "GC=F" | "GOLD" => ("GC=F".to_string(), AssetCategory::Commodity, "Gold Futures".to_string()),
            "CL=F" | "OIL" => ("CL=F".to_string(), AssetCategory::Commodity, "Crude Oil (WTI)".to_string()),
            "SI=F" | "SILVER" => ("SI=F".to_string(), AssetCategory::Commodity, "Silver Futures".to_string()),

            // Stocks & Indices
            "AAPL" => ("AAPL".to_string(), AssetCategory::Stock, "Apple Inc.".to_string()),
            "MSFT" => ("MSFT".to_string(), AssetCategory::Stock, "Microsoft Corp.".to_string()),
            "NVDA" => ("NVDA".to_string(), AssetCategory::Stock, "NVIDIA Corp.".to_string()),
            "GOOGL" => ("GOOGL".to_string(), AssetCategory::Stock, "Alphabet Inc.".to_string()),
            "AMZN" => ("AMZN".to_string(), AssetCategory::Stock, "Amazon.com Inc.".to_string()),
            "TSLA" => ("TSLA".to_string(), AssetCategory::Stock, "Tesla Inc.".to_string()),
            "SPY" => ("SPY".to_string(), AssetCategory::Stock, "SPDR S&P 500 ETF".to_string()),
            "QQQ" => ("QQQ".to_string(), AssetCategory::Stock, "Invesco QQQ (Nasdaq 100)".to_string()),

            other => {
                let category = if other.ends_with("-USD") || other.ends_with("-EUR") {
                    AssetCategory::Crypto
                } else if other.starts_with('^') {
                    AssetCategory::Bond
                } else if other.ends_with("=X") {
                    AssetCategory::Forex
                } else if other.ends_with("=F") {
                    AssetCategory::Commodity
                } else {
                    AssetCategory::Stock
                };
                (other.to_string(), category, other.to_string())
            }
        };

        Self {
            symbol: full_symbol,
            display_name,
            category,
            current_price: 0.0,
            change: 0.0,
            change_percent: 0.0,
            high_24h: 0.0,
            low_24h: 0.0,
            volume: 0.0,
        }
    }

    pub fn from_stored(symbol: String, display_name: String, category: AssetCategory) -> Self {
        Self {
            symbol,
            display_name,
            category,
            current_price: 0.0,
            change: 0.0,
            change_percent: 0.0,
            high_24h: 0.0,
            low_24h: 0.0,
            volume: 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn is_crypto(&self) -> bool {
        self.category == AssetCategory::Crypto
    }
}
