pub mod ohlc;
pub mod strategy;

use clap::Parser;
use std::path::PathBuf;

/// CLI args
#[derive(Parser, Debug, Clone, Default)]
#[command(
    version,
    about = "CoinGecko OHLC CSV exporter (top-N by mcap) with resume + simple scheduler"
)]
pub struct OhlcArgs {
    /// Output directory for CSVs
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Your CoinGecko Pro API key (or set CG_PRO_API_KEY env)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Number of top coins by market cap to export (excludes BTC baseline which is always added unless --skip-btc)
    #[arg(long)]
    pub top_n: Option<usize>,

    /// VS currency (e.g., usd, eur, krw)
    #[arg(long)]
    pub vs: Option<String>,

    /// Start date (inclusive), format YYYY-MM-DD
    #[arg(long)]
    pub start: Option<String>,

    /// End date (inclusive), format YYYY-MM-DD
    #[arg(long)]
    pub end: Option<String>,

    /// Concurrency for OHLC fetches (be mindful of plan limits)
    #[arg(long)]
    pub concurrency: Option<usize>,

    /// Delay (ms) between requests to avoid bursts
    #[arg(long)]
    pub request_delay_ms: Option<u64>,

    /// If true, also write a combined manifest.json with coin metadata
    #[arg(long)]
    pub write_manifest: Option<bool>,

    /// Resume mode: append only missing days per-asset (auto-detect last CSV date)
    #[arg(long)]
    pub resume: Option<bool>,

    /// Daily schedule: run every day at this local time (HH:MM). Example: --daily-at 05:10
    /// If not set, program runs once and exits (suitable for cron/systemd).
    #[arg(long)]
    pub daily_at: Option<String>,

    /// Optional lock file path to prevent concurrent runs
    #[arg(long)]
    pub lock_file: Option<PathBuf>,

    /// Skip pulling BTC baseline (useful if you run it separately)
    #[arg(long)]
    pub skip_btc: Option<bool>,
}

/// Backtests a relative-strength + trend strategy over daily OHLCV CSVs.
#[derive(Parser, Debug, Clone, Default)]
#[command(version, about)]
pub struct StrategyArgs {
    /// Path to BTC CSV (used for relative strength baseline)
    #[arg(long)]
    pub btc: Option<PathBuf>,
    /// Paths to asset CSVs
    #[arg(long, num_args=1..)]
    pub assets: Option<Vec<PathBuf>>,
    /// Output directory
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Lookbacks (days)
    #[arg(long)]
    pub ma_short: Option<usize>,
    #[arg(long)]
    pub ma_long: Option<usize>,

    /// Require 2-of-3 vs 3-of-3 for entry
    #[arg(long)]
    pub min_signals: Option<usize>,

    /// Allow shorting alts on full-bearish (3/3) downside
    #[arg(long)]
    pub short_alts: Option<bool>,

    /// BTC hedge weight when BTC is bear (price<MA30 && MA7<MA30). 0.0..1.0
    #[arg(long)]
    pub btc_hedge: Option<f64>,

    /// Stop configuration
    #[arg(long)]
    pub stop_lookback: Option<usize>,
    /// ATR multiple for stop (if high/low available)
    #[arg(long)]
    pub atr_mult: Option<f64>,
    /// Vol-based stop (if no H/L): k * rolling std of daily returns
    #[arg(long)]
    pub vol_mult: Option<f64>,
}
