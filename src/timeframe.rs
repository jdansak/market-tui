// src/timeframe.rs
//! Timeframe definitions and Yahoo Finance query parameters.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeframe {
    OneDay,
    OneWeek,
    OneMonth,
    SixMonths,
    YTD,
    All,
}

impl Timeframe {
    pub const ALL: [Timeframe; 6] = [
        Timeframe::OneDay,
        Timeframe::OneWeek,
        Timeframe::OneMonth,
        Timeframe::SixMonths,
        Timeframe::YTD,
        Timeframe::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Timeframe::OneDay => "1D",
            Timeframe::OneWeek => "1W",
            Timeframe::OneMonth => "1M",
            Timeframe::SixMonths => "6M",
            Timeframe::YTD => "YTD",
            Timeframe::All => "ALL",
        }
    }

    pub fn from_digit(c: char) -> Option<Self> {
        match c {
            '1' => Some(Timeframe::OneDay),
            '2' => Some(Timeframe::OneWeek),
            '3' => Some(Timeframe::OneMonth),
            '4' => Some(Timeframe::SixMonths),
            '5' => Some(Timeframe::YTD),
            '6' => Some(Timeframe::All),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn next(self) -> Self {
        use Timeframe::*;
        match self {
            OneDay => OneWeek,
            OneWeek => OneMonth,
            OneMonth => SixMonths,
            SixMonths => YTD,
            YTD => All,
            All => OneDay,
        }
    }

    #[allow(dead_code)]
    pub fn prev(self) -> Self {
        use Timeframe::*;
        match self {
            OneDay => All,
            OneWeek => OneDay,
            OneMonth => OneWeek,
            SixMonths => OneMonth,
            YTD => SixMonths,
            All => YTD,
        }
    }

    pub fn range(self) -> &'static str {
        match self {
            Timeframe::OneDay => "1d",
            Timeframe::OneWeek => "5d",
            Timeframe::OneMonth => "1mo",
            Timeframe::SixMonths => "6mo",
            Timeframe::YTD => "ytd",
            Timeframe::All => "max",
        }
    }

    pub fn interval(self) -> &'static str {
        match self {
            Timeframe::OneDay => "5m",
            Timeframe::OneWeek => "15m",
            Timeframe::OneMonth => "1h",
            Timeframe::SixMonths => "1d",
            Timeframe::YTD => "1d",
            Timeframe::All => "1wk",
        }
    }
}
