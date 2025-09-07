use anyhow::Result;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRow {
    date: NaiveDate,
    close: f64,
    ma_short: Option<f64>,
    ma_long: Option<f64>,
    rs: Option<f64>,
    rs_ma_short: Option<f64>,
    rs_ma_long: Option<f64>,
    trend_bull: bool,
    mom_bull: bool,
    rs_bull: bool,
    score: f64,
    raw_weight: f64,
    stop_level: Option<f64>,
}

impl SignalRow {
    // Getter methods for trade module
    pub fn close(&self) -> f64 {
        self.close
    }
    pub fn ma_short(&self) -> Option<f64> {
        self.ma_short
    }
    pub fn ma_long(&self) -> Option<f64> {
        self.ma_long
    }
    pub fn rs_ma_short(&self) -> Option<f64> {
        self.rs_ma_short
    }
    pub fn rs_ma_long(&self) -> Option<f64> {
        self.rs_ma_long
    }
}

#[derive(Debug, Clone)]
pub struct StrategyAnalysis {
    asset: String,
    total_days: usize,
    trading_days: usize,
    total_return: f64,
    max_return: f64,
    min_return: f64,
    win_rate: f64,
    avg_win: f64,
    avg_loss: f64,
    profit_factor: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    signals: Vec<SignalRow>,
}

impl StrategyAnalysis {
    pub fn new(asset: String, signals: Vec<SignalRow>) -> Self {
        let total_days = signals.len();
        let trading_days = signals.iter().filter(|s| s.raw_weight.abs() > 1e-6).count();

        // Calculate returns
        let mut returns = Vec::new();
        let mut cumulative_return = 1.0;
        let mut max_cumulative = 1.0;
        let mut max_drawdown = 0.0;

        for signal in &signals {
            if signal.raw_weight.abs() > 1e-6 {
                let daily_return =
                    signal.raw_weight * (signal.close - signals[0].close) / signals[0].close;
                cumulative_return *= 1.0 + daily_return;
                returns.push(daily_return);

                if cumulative_return > max_cumulative {
                    max_cumulative = cumulative_return;
                }
                let drawdown = (max_cumulative - cumulative_return) / max_cumulative;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        let total_return = cumulative_return - 1.0;
        let max_return = returns.iter().fold(0.0f64, |acc, &x| acc.max(x));
        let min_return = returns.iter().fold(0.0f64, |acc, &x| acc.min(x));

        // Calculate win rate and profit factor
        let wins: Vec<f64> = returns.iter().filter(|&&x| x > 0.0).cloned().collect();
        let losses: Vec<f64> = returns.iter().filter(|&&x| x < 0.0).cloned().collect();

        let win_rate = if returns.is_empty() {
            0.0
        } else {
            wins.len() as f64 / returns.len() as f64
        };
        let avg_win = if wins.is_empty() {
            0.0
        } else {
            wins.iter().sum::<f64>() / wins.len() as f64
        };
        let avg_loss = if losses.is_empty() {
            0.0
        } else {
            losses.iter().sum::<f64>() / losses.len() as f64
        };

        let total_wins = wins.iter().sum::<f64>();
        let total_losses = losses.iter().sum::<f64>().abs();
        let profit_factor = if total_losses == 0.0 {
            f64::INFINITY
        } else {
            total_wins / total_losses
        };

        // Calculate Sharpe ratio (simplified)
        let mean_return = if returns.is_empty() {
            0.0
        } else {
            returns.iter().sum::<f64>() / returns.len() as f64
        };
        let variance = if returns.len() <= 1 {
            0.0
        } else {
            let mean = mean_return;
            returns.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (returns.len() - 1) as f64
        };
        let sharpe_ratio = if variance == 0.0 {
            0.0
        } else {
            mean_return / variance.sqrt()
        };

        Self {
            asset,
            total_days,
            trading_days,
            total_return,
            max_return,
            min_return,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown,
            sharpe_ratio,
            signals,
        }
    }

    pub fn is_profitable(&self) -> bool {
        self.total_return > 0.0 && self.win_rate > 0.5 && self.profit_factor > 1.0
    }

    // Getter methods for trade module
    pub fn asset(&self) -> &String {
        &self.asset
    }
    pub fn total_return(&self) -> f64 {
        self.total_return
    }
    pub fn sharpe_ratio(&self) -> f64 {
        self.sharpe_ratio
    }
    pub fn win_rate(&self) -> f64 {
        self.win_rate
    }
    pub fn max_drawdown(&self) -> f64 {
        self.max_drawdown
    }
    pub fn trading_days(&self) -> usize {
        self.trading_days
    }
    pub fn signals(&self) -> &Vec<SignalRow> {
        &self.signals
    }
    pub fn profit_factor(&self) -> f64 {
        self.profit_factor
    }

    pub fn print_summary(&self) {
        println!("📊 {} Analysis", self.asset);
        println!("   Total Days: {}", self.total_days);
        println!("   Trading Days: {}", self.trading_days);
        println!("   Total Return: {:.2}%", self.total_return * 100.0);
        println!("   Max Return: {:.2}%", self.max_return * 100.0);
        println!("   Min Return: {:.2}%", self.min_return * 100.0);
        println!("   Win Rate: {:.1}%", self.win_rate * 100.0);
        println!("   Avg Win: {:.2}%", self.avg_win * 100.0);
        println!("   Avg Loss: {:.2}%", self.avg_loss * 100.0);
        println!("   Profit Factor: {:.2}", self.profit_factor);
        println!("   Max Drawdown: {:.2}%", self.max_drawdown * 100.0);
        println!("   Sharpe Ratio: {:.2}", self.sharpe_ratio);
        println!();
    }

    pub fn print_detailed_signals(&self) {
        println!("🔍 Detailed Signals for {}", self.asset);
        println!(
            "{:<12} {:<10} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8}",
            "Date",
            "Close",
            "MA_S",
            "MA_L",
            "RS",
            "RS_MA_S",
            "RS_MA_L",
            "Trend",
            "Mom",
            "RS",
            "Score",
            "Weight",
            "Stop"
        );
        println!("{}", "-".repeat(120));

        for signal in &self.signals {
            if signal.raw_weight.abs() > 1e-6 {
                println!(
                    "{:<12} {:<10.2} {:<8.2} {:<8.2} {:<8.2} {:<8.2} {:<8.2} {:<8} {:<8} {:<8} {:<8.2} {:<8.2} {:<8.2}",
                    signal.date.format("%Y-%m-%d"),
                    signal.close,
                    signal.ma_short.unwrap_or(0.0),
                    signal.ma_long.unwrap_or(0.0),
                    signal.rs.unwrap_or(0.0),
                    signal.rs_ma_short.unwrap_or(0.0),
                    signal.rs_ma_long.unwrap_or(0.0),
                    if signal.trend_bull { "✓" } else { "✗" },
                    if signal.mom_bull { "✓" } else { "✗" },
                    if signal.rs_bull { "✓" } else { "✗" },
                    signal.score,
                    signal.raw_weight,
                    signal.stop_level.unwrap_or(0.0)
                );
            }
        }
        println!();
    }
}

pub fn read_signals_file(path: &PathBuf) -> Result<Vec<SignalRow>> {
    let mut signals = Vec::new();
    let mut rdr = ReaderBuilder::new().trim(csv::Trim::All).from_path(path)?;

    for result in rdr.deserialize::<SignalRow>() {
        let signal = result?;
        signals.push(signal);
    }

    Ok(signals)
}

pub fn analyze_signals_directory(signals_dir: &str) -> Result<Vec<StrategyAnalysis>> {
    let mut analyses = Vec::new();
    let entries = fs::read_dir(signals_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().unwrap_or_default() == "csv" {
            let filename = path.file_name().unwrap().to_string_lossy();
            if filename.starts_with("signals_") {
                let asset = filename
                    .strip_prefix("signals_")
                    .unwrap()
                    .strip_suffix(".csv")
                    .unwrap()
                    .to_string();

                match read_signals_file(&path) {
                    Ok(signals) => {
                        let analysis = StrategyAnalysis::new(asset, signals);
                        analyses.push(analysis);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", filename, e);
                    }
                }
            }
        }
    }

    Ok(analyses)
}

pub fn print_profitable_strategies(analyses: &[StrategyAnalysis]) {
    let profitable: Vec<_> = analyses.iter().filter(|a| a.is_profitable()).collect();

    if profitable.is_empty() {
        println!("❌ No profitable strategies found!");
        return;
    }

    println!("🎯 PROFITABLE TRADING STRATEGIES SUMMARY");
    println!("{}", "=".repeat(80));
    println!(
        "Found {} profitable strategies out of {} total strategies",
        profitable.len(),
        analyses.len()
    );
    println!();

    // Sort by total return (descending)
    let mut sorted = profitable.clone();
    sorted.sort_by(|a, b| b.total_return.partial_cmp(&a.total_return).unwrap());

    println!("📈 TOP PERFORMING STRATEGIES (by Total Return)");
    println!(
        "{:<25} {:<12} {:<10} {:<10} {:<10} {:<10} {:<10}",
        "Asset", "Total Ret%", "Win Rate%", "Profit Factor", "Sharpe", "Max DD%", "Trading Days"
    );
    println!("{}", "-".repeat(90));

    for analysis in &sorted {
        println!(
            "{:<25} {:<12.2} {:<10.1} {:<10.2} {:<10.2} {:<10.2} {:<10}",
            analysis.asset,
            analysis.total_return * 100.0,
            analysis.win_rate * 100.0,
            analysis.profit_factor,
            analysis.sharpe_ratio,
            analysis.max_drawdown * 100.0,
            analysis.trading_days
        );
    }
    println!();

    // Sort by Sharpe ratio (descending)
    sorted.sort_by(|a, b| b.sharpe_ratio.partial_cmp(&a.sharpe_ratio).unwrap());

    println!("⚡ TOP RISK-ADJUSTED STRATEGIES (by Sharpe Ratio)");
    println!(
        "{:<25} {:<12} {:<10} {:<10} {:<10} {:<10} {:<10}",
        "Asset", "Sharpe", "Total Ret%", "Win Rate%", "Profit Factor", "Max DD%", "Trading Days"
    );
    println!("{}", "-".repeat(90));

    for analysis in &sorted {
        println!(
            "{:<25} {:<12.2} {:<10.2} {:<10.1} {:<10.2} {:<10.2} {:<10}",
            analysis.asset,
            analysis.sharpe_ratio,
            analysis.total_return * 100.0,
            analysis.win_rate * 100.0,
            analysis.profit_factor,
            analysis.max_drawdown * 100.0,
            analysis.trading_days
        );
    }
    println!();

    // Overall statistics
    let total_strategies = analyses.len();
    let profitable_count = profitable.len();
    let avg_return: f64 =
        profitable.iter().map(|a| a.total_return).sum::<f64>() / profitable_count as f64;
    let avg_win_rate: f64 =
        profitable.iter().map(|a| a.win_rate).sum::<f64>() / profitable_count as f64;
    let avg_sharpe: f64 =
        profitable.iter().map(|a| a.sharpe_ratio).sum::<f64>() / profitable_count as f64;

    println!("📊 OVERALL STATISTICS");
    println!("   Total Strategies Analyzed: {}", total_strategies);
    println!(
        "   Profitable Strategies: {} ({:.1}%)",
        profitable_count,
        profitable_count as f64 / total_strategies as f64 * 100.0
    );
    println!("   Average Return (Profitable): {:.2}%", avg_return * 100.0);
    println!(
        "   Average Win Rate (Profitable): {:.1}%",
        avg_win_rate * 100.0
    );
    println!("   Average Sharpe (Profitable): {:.2}", avg_sharpe);
    println!();
}

pub fn print_detailed_analysis(analyses: &[StrategyAnalysis], asset: &str) {
    if let Some(analysis) = analyses.iter().find(|a| a.asset == asset) {
        analysis.print_summary();
        analysis.print_detailed_signals();
    } else {
        println!("❌ Asset '{}' not found in analysis results", asset);
    }
}

pub fn execute(signals_dir: &str, detailed_asset: Option<&str>) -> Result<()> {
    println!("🔍 Analyzing trading strategies from: {}", signals_dir);
    println!();

    let analyses = analyze_signals_directory(signals_dir)?;

    if analyses.is_empty() {
        println!("❌ No signal files found in {}", signals_dir);
        return Ok(());
    }

    print_profitable_strategies(&analyses);

    if let Some(asset) = detailed_asset {
        print_detailed_analysis(&analyses, asset);
    }

    Ok(())
}
