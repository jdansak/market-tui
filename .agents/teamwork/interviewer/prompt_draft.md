Send this to Max for work. Create a TUI Rust application that pulls the latest stock ticker, displays historical charts across multiple intervals, tracks cryptocurrencies (such as Bitcoin, SOL, Ethereum), and ensures an easy-to-read, polished display with high-contrast colors.
[OPEN: What is the primary purpose and target audience for the application?]

Working directory: [OPEN: Where should the project codebase be located?]
Integrity mode: [OPEN: Which restrictions should apply to the team's implementation?]

## Requirements

### R1. Stock Ticker and Historical Charts
The application must fetch the latest ticker quotes for stocks and generate historical charts at 1 day, 1 week, 1 month, 6 months, year to date (YTD), and the life of the stock.
[OPEN: What data source or market API should be used for stock and crypto data?]

### R2. Cryptocurrency Tracking
The application must track cryptocurrencies including Bitcoin, SOL, and Ethereum, displaying current prices and historical performance.

### R3. Visual Clarity and TUI Polish
The TUI must be easy to read and well polished, ensuring that colors are not too close together and remain visually distinguishable across terminal displays.

## Acceptance Criteria

### Functionality
- [ ] Fetches and displays latest price tickers for user-queried stocks.
- [ ] Displays historical charts selectable across all requested intervals: 1 day, 1 week, 1 month, 6 months, year-to-date, and life of the stock.
- [ ] Tracks and displays market data for Bitcoin, SOL, and Ethereum.
- [ ] Automated verification: [OPEN: Are there existing test suites or reference data to verify against?]

### User Interface
- [ ] Visual color scheme maintains distinct, non-overlapping color values for adjacent chart series, labels, and UI panels.
- [ ] TUI renders cleanly and remains fully legible in standard terminal emulators without overlapping text.
