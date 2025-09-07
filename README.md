# Crypto Strategy

A momentum-based cryptocurrency trading strategy derived from the Crypto Trends Data that maximizes profit by combining momentum signals, relative strength analysis, and risk management techniques.

## Overview

This strategy leverages 7-day and 30-day moving averages along with relative performance metrics against Bitcoin to generate trading signals. The approach focuses on identifying altcoins that are outperforming the market leader while maintaining robust risk management through position sizing and stop-loss mechanisms.

## Strategy Logic

### 1. Core Signal Definition

#### Buy Entry Conditions

- Price > 30-day MA (medium-term bullish trend)
- Relative performance vs BTC is widening positive (token outperforming BTC)
- 7-day MA either confirms (above 30d) or is crossing upward

#### Sell/Short Entry Conditions

- Price < 30-day MA (medium-term bearish trend)
- Relative performance vs BTC is widening negative (token underperforming BTC)
- 7-day MA confirms (below 30d) or crossing downward

### 2. Filtering Noise

- Only take trades where at least 2 out of 3 conditions (30d trend, 7d momentum, relative performance) align
- Avoid tokens with conflicting signals (e.g., above 30d MA but underperforming BTC)

### 3. Position Sizing

- **Strong conviction** = all 3 bullish signals aligned → full allocation
- **Partial conviction** = 2/3 aligned → half allocation
- Hedge with BTC or ETH shorts if overall market trend (BTC & ETH < 30d MA) is bearish

### 4. Profit Taking & Stop Loss

- Take profit when relative performance vs BTC starts shrinking
- Use trailing stop = 7-day ATR (Average True Range)
- Hard stop if price closes below 30-day MA (for longs) or above 30-day MA (for shorts)

## Example Applications

Based on the report analysis:

| Token                | 30d MA | 7d MA | vs BTC          | Signal      | Allocation |
| -------------------- | ------ | ----- | --------------- | ----------- | ---------- |
| **Mantle (MNT)**     | Above  | Above | Outperforming   | Strong Buy  | Full       |
| **Chainlink (LINK)** | Above  | Above | Outperforming   | Strong Buy  | Full       |
| **Cardano (ADA)**    | Above  | Below | Outperforming   | Buy         | Partial    |
| **BNB**              | Above  | Below | Underperforming | Speculative | Minimal    |
| **Solana (SOL)**     | Below  | Below | Underperforming | Avoid/Short | None       |

## Strategy Analysis

### Pros and Cons

| Strategy Element                     | Pros                                                                  | Cons                                              |
| ------------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------- |
| **Trend + Momentum (30d vs 7d MAs)** | Captures medium-term reversals and sustained rallies                  | Whipsaws in sideways markets                      |
| **Relative Performance vs BTC**      | Focuses on tokens that outperform the market leader                   | Can miss absolute gains in correlated markets     |
| **2-out-of-3 Filter**                | Reduces false signals from short-term noise                           | May skip early entries before full confirmation   |
| **Position Sizing by Conviction**    | Scales exposure with signal strength, improving risk-adjusted returns | More complex execution than equal-weight strategy |
| **Trailing ATR Stops**               | Locks in profits during rallies                                       | Volatility spikes may trigger premature exits     |
| **Hedging with BTC/ETH**             | Provides downside protection in broad selloffs                        | Reduces net upside if market recovers quickly     |

## Strategy Summary

The optimal strategy is a **relative strength momentum strategy** that:

- Buys altcoins trending above their 30-day moving average and outperforming Bitcoin
- Avoids or shorts tokens below both averages and underperforming BTC
- Uses a 2-out-of-3 signal filter to reduce noise and ensure entries are based on both trend and strength
- Protects profits with ATR-based trailing stops
- Scales position size with signal alignment

## Implementation Details

### Design Rationale

- Signals mirror chart logic exactly: Trend (close > MA30), Momentum (MA7 > MA30), RS widening (RS MA7 > RS MA30)
- Position size scales with conviction
- Stops use ATR when high/low exist; otherwise a volatility proxy from rolling return std
- Portfolio is daily-rebalanced to equal-weight among qualifying longs
- Optional BTC short hedge engages only when BTC is clearly bearish (price<MA30 and MA7<MA30)
- Outputs are portable CSVs for analysis in Python, Rust plotters, or BI tools

## Usage

### Quick Start

1. **Prepare CSV files** with headers: `date,open,high,low,close` (high/low optional; if missing, the stop uses return-vol instead of ATR)
2. **Include one BTC CSV** (for relative strength calculation)
3. **Run the strategy**:

```bash
cargo run --release -- \
  --btc ./data/BTC.csv \
  --assets ./data/LINK.csv ./data/MNT.csv ./data/ADA.csv \
  --out ./backtest_out \
  --short-alts false --btc-hedge 0.3
```

### Output Files

The strategy generates:

- `./backtest_out/signals_<ASSET>.csv` - Daily signals and positions
- `./backtest_out/equity_curve.csv` - Portfolio equity curve
- `./backtest_out/metrics.txt` - Performance metrics (CAGR, Sharpe, MDD, etc.)

## Data Collection

### OHLC Data Exporter

The project includes an advanced OHLC (Open, High, Low, Close) data exporter that fetches historical cryptocurrency data from CoinGecko's Pro API with support for incremental updates, scheduling, and production deployment.

#### Key Features

**Extended Capabilities**:

- **Idempotent Resume**: Automatically detects last CSV date and only fetches missing days
- **Scheduling Support**: Built-in daemon mode or system cron/systemd integration
- **Atomic Writes**: Safe concurrent operation with file locking
- **Incremental Updates**: Efficient daily data collection without re-downloading existing data

#### Quick Start

1. **Set up the environment**:

   ```bash
   # Set your CoinGecko Pro API key
   export CG_PRO_API_KEY="YOUR_KEY"
   ```

2. **One-shot backfill (cron/systemd friendly)**:

   ```bash
   # First run (fresh backfill) - defaults to last 30 days up to today
   cargo run --release --bin ohlc_exporter -- \
     --out ./data \
     --top-n 100 \
     --vs usd

   # Incremental runs (append only missing days)
   cargo run --release --bin ohlc_exporter -- \
     --out ./data \
     --top-n 100 \
     --vs usd \
     --resume \
     --lock-file ./data/exporter.lock
   ```

3. **Built-in daily scheduler (no system cron needed)**:
   ```bash
   cargo run --release --bin ohlc_exporter -- \
     --out ./data \
     --top-n 100 \
     --vs usd \
     --resume \
     --daily-at 05:10 \
     --lock-file ./data/exporter.lock
   ```

#### Production Deployment

**System Cron (Linux)**:

```bash
# Run daily at 05:10 local time
10 5 * * * cd /opt/cg_ohlc_exporter && CG_PRO_API_KEY=YOUR_KEY ./target/release/ohlc_exporter \
  --out /data/cg \
  --top-n 100 \
  --vs usd \
  --resume \
  --lock-file /data/cg/exporter.lock >> /var/log/cg_exporter.log 2>&1
```

**Systemd Timer**:

```ini
# cg-exporter.service
[Unit]
Description=CoinGecko OHLC Exporter

[Service]
Type=oneshot
ExecStart=/opt/cg_ohlc_exporter/target/release/ohlc_exporter --out /data/cg --top-n 100 --vs usd --resume --lock-file /data/cg/exporter.lock
User=cg_exporter

# cg-exporter.timer
[Unit]
Description=Run CoinGecko exporter daily

[Timer]
OnCalendar=*-*-* 05:10:00
Persistent=true
Unit=cg-exporter.service

[Install]
WantedBy=timers.target
```

**macOS Launchd**:
Use `StartCalendarInterval` with `Hour=5, Minute=10` for daily execution.

#### Command Line Options

| Option               | Description                                     | Default     |
| -------------------- | ----------------------------------------------- | ----------- |
| `--out`              | Output directory for CSVs                       | `./out`     |
| `--api-key`          | CoinGecko Pro API key (or set `CG_PRO_API_KEY`) | Required    |
| `--top-n`            | Number of top coins by market cap               | `100`       |
| `--vs`               | Quote currency (usd, eur, krw, etc.)            | `usd`       |
| `--start`            | Start date (YYYY-MM-DD)                         | 30 days ago |
| `--end`              | End date (YYYY-MM-DD)                           | Today       |
| `--concurrency`      | Parallel request limit                          | `6`         |
| `--request-delay-ms` | Delay between requests (ms)                     | `250`       |
| `--write-manifest`   | Write manifest.json with metadata               | `true`      |
| `--resume`           | Resume mode: append missing days only           | `false`     |
| `--daily-at`         | Daily schedule time (HH:MM)                     | None        |
| `--lock-file`        | File lock path for concurrency control          | None        |
| `--skip-btc`         | Skip Bitcoin baseline (if run separately)       | `false`     |

#### Output Files

The exporter creates:

- `./data/BTC.csv` - Bitcoin baseline data
- `./data/<COIN_SYMBOL>_<COIN_ID>.csv` - Individual coin data files
- `./data/manifest.json` - Coin metadata (optional)
- `./data/exporter.lock` - Process lock file (optional)

All CSV files use the schema: `date,open,high,low,close`

#### Design Rationale

**Data Source Strategy**:

- **Top-N Selection**: Uses `/coins/markets` with `order=market_cap_desc` and pagination
- **OHLC Data**: Sources daily OHLC from `/coins/{id}/ohlc/range` with `interval=daily`
- **API Limitations**: Handles 180-day candle limit by chunking and stitching results
- **Authentication**: Uses Pro API header `x-cg-pro-api-key`

**Resume Mode**:

- Per-asset detection of last CSV date
- Fetches only missing days (last+1 to end)
- Atomic writes via temp file + rename
- Prevents duplicate data on overlapping runs

**Concurrency Control**:

- Single instance lock (advisory file locking)
- Prevents concurrent runs from cron + daemon
- Safe for overlapping cron executions

#### Implementation Notes

**Rate Limiting**:

- Automatic throttling and backoff on 429 responses
- Respects `Retry-After` headers
- Configurable concurrency and delays

**File Naming**:

- Includes both symbol and ID to avoid conflicts
- BTC baseline always written as `BTC.csv`
- Atomic writes prevent partial file corruption

**Data Quality**:

- Daily deduplication by date (keeps last candle per day)
- Timestamp handling respects candle close times
- Defensive overlap filtering in resume mode

**Production Considerations**:

- File locking prevents concurrent execution
- Logging integration for monitoring
- Graceful error handling and retries
- Memory-efficient streaming for large datasets
