use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::ai_insights::{generate_asset_insights, generate_fallback_insights, AssetMetrics};
use crate::analyzer::{StrategyAnalysis, analyze_signals_directory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradePlan {
    pub asset: String,
    pub entry_rules: EntryRules,
    pub exit_rules: ExitRules,
    pub position_sizing: PositionSizing,
    pub conviction: Conviction,
    pub backtest_stats: BacktestStats,
    pub computed_values: ComputedValues,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRules {
    pub primary: String,
    pub alternative: String,
    pub signal_conditions: SignalConditions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConditions {
    pub trend: String,                 // close > MA30
    pub momentum: String,              // MA7 > MA30
    pub rs: String,                    // RS_MA7 > RS_MA30
    pub full_weight_condition: String, // 3/3 signals
    pub half_weight_condition: String, // ≥2/3 AND RS bullish
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitRules {
    pub profit_taking: String,
    pub stop_loss: String,
    pub trailing_stop: String,
    pub hard_exit_conditions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSizing {
    pub full_weight: f64,
    pub half_weight: f64,
    pub risk_cap_percent: f64,
    pub risk_calculation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conviction {
    pub high_conviction: f64,   // 3/3 signals
    pub medium_conviction: f64, // 2/3+RS signals
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestStats {
    pub total_return_percent: f64,
    pub sharpe_ratio: f64,
    pub win_rate_percent: f64,
    pub max_drawdown_percent: f64,
    pub trading_days: usize,
    pub expected_return: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMode {
    pub signal_at_close: bool,
    pub pullback_to_ma30: bool,
    pub extended_threshold: f64, // 10% above MA30
    pub limit_order_duration_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComputedValues {
    // Current market data (from latest signal)
    pub current_price: f64, // Latest close price
    pub ma30: f64,          // 30-day moving average
    pub ma7: f64,           // 7-day moving average
    pub rs_ma7: f64,        // 7-day relative strength vs BTC
    pub rs_ma30: f64,       // 30-day relative strength vs BTC
    pub atr_14: f64,        // 14-day Average True Range
    pub volatility: f64,    // 14-day rolling volatility

    // Signal status
    pub trend_signal: bool,    // close > MA30
    pub momentum_signal: bool, // MA7 > MA30
    pub rs_signal: bool,       // RS_MA7 > RS_MA30
    pub all_signals: bool,     // 3/3 signals (trend + momentum + RS)
    pub partial_signals: bool, // 2/3+RS signals

    // Position sizing calculations
    pub stop_price: f64,             // Calculated stop loss price
    pub risk_per_share: f64,         // Risk per share (entry - stop)
    pub max_shares_by_risk: f64,     // Max shares based on risk cap
    pub max_shares_by_position: f64, // Max shares based on position cap
    pub recommended_shares: u64,     // Final recommended position size
    pub position_value: f64,         // Total position value
    pub position_percent: f64,       // Position as % of portfolio

    // Profit taking calculations
    pub profit_target: f64,         // 2R profit target price
    pub profit_target_percent: f64, // Profit target as % gain
    pub scale_out_shares: u64,      // Shares to sell at profit target
    pub remaining_shares: u64,      // Shares to trail after scale out
    pub scale_out_value: f64,       // Value of shares to scale out

    // Stop loss levels
    pub initial_stop: f64,      // Initial stop loss price
    pub stop_loss_percent: f64, // Stop loss as % loss
    pub trailing_stop: f64,     // Current trailing stop price
    pub stop_distance_atr: f64, // Stop distance in ATR units

    // Risk management
    pub portfolio_risk: f64,    // Total risk as % of portfolio
    pub risk_reward_ratio: f64, // Risk/reward ratio
    pub max_loss: f64,          // Maximum possible loss
    pub max_gain: f64,          // Maximum possible gain (2R)

    // Execution parameters
    pub is_extended: bool,        // Price > 10% above MA30
    pub ma30_pullback_price: f64, // MA30 price for pullback entry
    pub extended_percent: f64,    // How much above MA30 (if extended)
    pub signal_strength: f64,     // Signal strength score (0-1)
}

impl TradePlan {
    /// Create a `TradePlan` from analysis with AI-powered insights
    ///
    /// # Errors
    /// Returns an error if AI insights cannot be generated or if data processing fails.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub async fn from_analysis(analysis: &StrategyAnalysis, rank: usize) -> Result<Self> {
        let asset = analysis.asset().clone();
        let stats = analysis;

        // Determine conviction based on performance metrics
        let (high_conviction, medium_conviction, conviction_rationale) =
            determine_conviction(stats);

        // Determine execution mode based on asset characteristics
        let execution_mode = determine_execution_mode(&asset, stats);

        // Generate initial computed values for risk assessment
        let initial_computed_values =
            generate_computed_values(&asset, stats, &execution_mode, 0.01); // Use default 1% for initial calculation

        // Determine risk cap based on quantitative analysis
        let risk_cap = determine_risk_cap(&asset, stats, &initial_computed_values);

        // Generate final computed values with proper risk cap
        let computed_values = generate_computed_values(&asset, stats, &execution_mode, risk_cap);

        // Generate AI-powered asset-specific notes
        let notes = match generate_asset_notes_ai(&asset, stats, &computed_values).await {
            Ok(ai_notes) => ai_notes,
            Err(e) => {
                println!(
                    "⚠️  AI insights failed for {asset}: {e}. Using fallback analysis."
                );
                generate_asset_notes(&asset, stats, rank)
            }
        };

        Ok(Self {
            asset: asset.clone(),
            entry_rules: EntryRules {
                primary: format!(
                    "Go long EOD when 3/3 signals (trend + momentum + RS). {}",
                    if execution_mode.pullback_to_ma30 {
                        format!("Alt entry: Limit buy at MA30 if price is extended (>{:.0}% above MA30) on signal day.", 
                               execution_mode.extended_threshold * 100.0)
                    } else {
                        "Use signal-at-close execution only.".to_string()
                    }
                ),
                alternative: if execution_mode.pullback_to_ma30 {
                    "Staggered entry: 50% at signal close, 50% at MA30 limit if extended.".to_string()
                } else {
                    "Market-on-close entry preferred due to liquidity constraints.".to_string()
                },
                signal_conditions: SignalConditions {
                    trend: "close > MA30".to_string(),
                    momentum: "MA7 > MA30".to_string(),
                    rs: "RS_MA7 > RS_MA30".to_string(),
                    full_weight_condition: "3/3 signals = 1.00 raw weight".to_string(),
                    half_weight_condition: "≥2/3 AND RS bullish = 0.50 raw weight".to_string(),
                },
            },
            exit_rules: ExitRules {
                profit_taking: "Scale 50% at +2R (R = initial risk from entry to stop), then trail the rest".to_string(),
                stop_loss: "Initial stop: close – 3.0 × ATR14 (fallback: close × (1 − 2.5 × rolling_std14))".to_string(),
                trailing_stop: "Ratchet stop to max(prior stop, close – 3.0 × ATR14) each day".to_string(),
                hard_exit_conditions: "Hard exit if close < MA30 or RS flips bearish (RS_MA7 < RS_MA30)".to_string(),
            },
            position_sizing: PositionSizing {
                full_weight: 1.0,
                half_weight: 0.5,
                risk_cap_percent: risk_cap * 100.0,
                risk_calculation: format!(
                    "R = entry_price − stop_price; units = min(raw_weight_normalized × portfolio_value / entry_price, ({}% × portfolio_value) / R) \
                    Risk per share: ${:.2}, Max shares by risk: {:.0}, Max shares by position: {:.0}, Recommended: {:.0}",
                    risk_cap * 100.0,
                    computed_values.risk_per_share,
                    computed_values.max_shares_by_risk,
                    computed_values.max_shares_by_position,
                    computed_values.recommended_shares
                ),
            },
            conviction: Conviction {
                high_conviction,
                medium_conviction,
                rationale: conviction_rationale,
            },
            backtest_stats: BacktestStats {
                total_return_percent: stats.total_return() * 100.0,
                sharpe_ratio: stats.sharpe_ratio(),
                win_rate_percent: stats.win_rate() * 100.0,
                max_drawdown_percent: stats.max_drawdown() * 100.0,
                trading_days: stats.trading_days(),
                expected_return: format!(
                    "+{:.2}%, Sharpe {:.2}, Win {:.1}%, MaxDD {:.2}%, {} days",
                    stats.total_return() * 100.0,
                    stats.sharpe_ratio(),
                    stats.win_rate() * 100.0,
                    stats.max_drawdown() * 100.0,
                    stats.trading_days()
                ),
            },
            computed_values,
            notes,
        })
    }

    #[must_use]
    pub fn print_playbook(&self, rank: usize) -> Vec<String> {
        let mut playbook: Vec<String> = Vec::new();

        playbook.push(format!("Rank: {rank}"));
        playbook.push(format!("Entry (primary): {}", self.entry_rules.primary));

        if !self.entry_rules.alternative.is_empty() {
            playbook.push(format!("Alt entry: {}", self.entry_rules.alternative));
        }

        playbook.push(format!("Exit: {}", self.exit_rules.profit_taking));
        playbook.push(format!("Stop: {}", self.exit_rules.stop_loss));
        playbook.push(format!("Size: Full (3/3) or Half (2/3+RS). Cap single-name risk at {:.1}% of equity (position = {:.1}% / R).", self.position_sizing.risk_cap_percent * 100.0, self.position_sizing.risk_cap_percent * 100.0));
        playbook.push(format!("Conviction: High ({:.0}%) on 3/3; Medium ({:.0}%) on 2/3+RS.", self.conviction.high_conviction * 100.0, self.conviction.medium_conviction * 100.0));
        playbook.push(format!("Expected: {}", self.backtest_stats.expected_return));
        playbook.push(format!("Notes: {}", self.notes));
        
        println!("{}) {}", rank, self.asset);
        for f in playbook.iter() {
            println!("   • {f}");
        }
        println!();
        
        playbook
    }

    pub fn print_execution(&self, portfolio_value: f64, current_price: f64, atr: f64) {
        println!(
            "📊 EXECUTION for {} (Portfolio: ${:.0}, Price: ${:.2}, ATR: ${:.2})",
            self.asset, portfolio_value, current_price, atr
        );

        let cv = &self.computed_values;

        println!("   • Current Market Data:");
        println!("     - Current Price: ${:.2}", cv.current_price);
        println!("     - MA30: ${:.2}", cv.ma30);
        println!("     - MA7: ${:.2}", cv.ma7);
        println!("     - RS_MA7: {:.3}", cv.rs_ma7);
        println!("     - RS_MA30: {:.3}", cv.rs_ma30);
        println!("     - ATR_14: ${:.4}", cv.atr_14);
        println!("     - Volatility: {:.1}%", cv.volatility * 100.0);

        println!("   • Signal Status:");
        println!(
            "     - Trend Signal: {} (close > MA30: ${:.2} > ${:.2})",
            cv.trend_signal, cv.current_price, cv.ma30
        );
        println!(
            "     - Momentum Signal: {} (MA7 > MA30: ${:.2} > ${:.2})",
            cv.momentum_signal, cv.ma7, cv.ma30
        );
        println!(
            "     - RS Signal: {} (RS_MA7 > RS_MA30: {:.3} > {:.3})",
            cv.rs_signal, cv.rs_ma7, cv.rs_ma30
        );
        println!("     - All Signals (3/3): {}", cv.all_signals);
        println!("     - Partial Signals (2/3+RS): {}", cv.partial_signals);
        println!("     - Signal Strength: {:.0}%", cv.signal_strength * 100.0);

        println!("   • Position Sizing:");
        println!("     - Stop Price: ${:.2}", cv.stop_price);
        println!("     - Risk per Share: ${:.4}", cv.risk_per_share);
        println!("     - Max Shares by Risk: {:.0}", cv.max_shares_by_risk);
        println!(
            "     - Max Shares by Position: {:.0}",
            cv.max_shares_by_position
        );
        println!("     - Recommended Shares: {}", cv.recommended_shares);
        println!("     - Position Value: ${:.2}", cv.position_value);
        println!(
            "     - Position % of Portfolio: {:.1}%",
            cv.position_percent * 100.0
        );

        println!("   • Profit Taking:");
        println!(
            "     - Profit Target: ${:.2} (+{:.1}%)",
            cv.profit_target, cv.profit_target_percent
        );
        println!("     - Scale Out Shares: {}", cv.scale_out_shares);
        println!("     - Scale Out Value: ${:.2}", cv.scale_out_value);
        println!("     - Remaining Shares: {}", cv.remaining_shares);

        println!("   • Stop Loss:");
        println!(
            "     - Initial Stop: ${:.2} (-{:.1}%)",
            cv.initial_stop, cv.stop_loss_percent
        );
        println!("     - Trailing Stop: ${:.2}", cv.trailing_stop);
        println!("     - Stop Distance: {:.1} ATR", cv.stop_distance_atr);

        println!("   • Risk Management:");
        println!("     - Portfolio Risk: {:.2}%", cv.portfolio_risk * 100.0);
        println!("     - Risk/Reward Ratio: {:.1}:1", cv.risk_reward_ratio);
        println!("     - Max Loss: ${:.2}", cv.max_loss);
        println!("     - Max Gain: ${:.2}", cv.max_gain);

        println!("   • Execution Parameters:");
        println!(
            "     - Is Extended: {} ({:.1}% above MA30)",
            cv.is_extended, cv.extended_percent
        );
        println!("     - MA30 Pullback Price: ${:.2}", cv.ma30_pullback_price);

        println!();
    }
}

fn determine_conviction(stats: &StrategyAnalysis) -> (f64, f64, String) {
    let sharpe = stats.sharpe_ratio();
    let win_rate = stats.win_rate();
    let max_dd = stats.max_drawdown();

    if sharpe >= 4.0 && win_rate >= 0.95 && max_dd <= 0.01 {
        (
            0.95,
            0.80,
            "Very High conviction due to exceptional Sharpe ratio and clean performance"
                .to_string(),
        )
    } else if sharpe >= 2.0 && win_rate >= 0.90 && max_dd <= 0.05 {
        (
            0.90,
            0.75,
            "High conviction based on strong risk-adjusted returns".to_string(),
        )
    } else if sharpe >= 1.5 && win_rate >= 0.85 && max_dd <= 0.10 {
        (
            0.85,
            0.70,
            "High conviction with good risk management".to_string(),
        )
    } else if sharpe >= 1.0 && win_rate >= 0.80 {
        (
            0.80,
            0.65,
            "Medium-High conviction with acceptable risk profile".to_string(),
        )
    } else if sharpe >= 0.5 && win_rate >= 0.70 {
        (
            0.75,
            0.60,
            "Medium conviction with moderate risk".to_string(),
        )
    } else {
        (
            0.70,
            0.55,
            "Lower conviction due to risk concerns".to_string(),
        )
    }
}

fn determine_risk_cap(
    _asset: &str,
    stats: &StrategyAnalysis,
    computed_values: &ComputedValues,
) -> f64 {
    // Multi-factor risk assessment based on quantitative metrics
    let base_risk = 0.010; // 1.0% base risk cap

    // Factor 1: Sharpe Ratio Adjustment (risk-adjusted returns)
    let sharpe_adjustment = match stats.sharpe_ratio() {
        s if s >= 3.0 => 1.5, // Excellent risk-adjusted returns
        s if s >= 2.0 => 1.3, // Very good
        s if s >= 1.5 => 1.1, // Good
        s if s >= 1.0 => 1.0, // Average
        s if s >= 0.5 => 0.8, // Below average
        _ => 0.6,             // Poor risk-adjusted returns
    };

    // Factor 2: Drawdown Risk Adjustment
    let drawdown_adjustment = match stats.max_drawdown() {
        d if d <= 0.02 => 1.2, // Very low drawdown
        d if d <= 0.05 => 1.0, // Low drawdown
        d if d <= 0.10 => 0.8, // Moderate drawdown
        d if d <= 0.20 => 0.6, // High drawdown
        _ => 0.4,              // Very high drawdown
    };

    // Factor 3: Win Rate Consistency
    let win_rate_adjustment = match stats.win_rate() {
        w if w >= 0.80 => 1.2, // Very consistent
        w if w >= 0.70 => 1.1, // Consistent
        w if w >= 0.60 => 1.0, // Average
        w if w >= 0.50 => 0.9, // Below average
        _ => 0.7,              // Inconsistent
    };

    // Factor 4: Volatility Assessment
    let volatility_adjustment = match computed_values.volatility {
        v if v <= 20.0 => 1.1, // Low volatility
        v if v <= 40.0 => 1.0, // Moderate volatility
        v if v <= 60.0 => 0.9, // High volatility
        v if v <= 80.0 => 0.8, // Very high volatility
        _ => 0.6,              // Extreme volatility
    };

    // Factor 5: Return Magnitude (extreme returns need tighter risk management)
    let return_magnitude_adjustment = match stats.total_return() {
        r if r <= 10.0 => 1.1,   // Conservative returns
        r if r <= 50.0 => 1.0,   // Moderate returns
        r if r <= 200.0 => 0.9,  // High returns
        r if r <= 1000.0 => 0.8, // Very high returns
        _ => 0.6,                // Extreme returns (need tighter risk)
    };

    // Factor 6: Trading Days (more data = more confidence)
    let data_confidence_adjustment = match stats.trading_days() {
        d if d >= 20 => 1.1, // High confidence
        d if d >= 15 => 1.0, // Good confidence
        d if d >= 10 => 0.9, // Moderate confidence
        d if d >= 5 => 0.8,  // Low confidence
        _ => 0.7,            // Very low confidence
    };

    // Factor 7: Profit Factor (risk-reward efficiency)
    let profit_factor_adjustment = match stats.profit_factor() {
        p if p >= 5.0 => 1.2, // Excellent profit factor
        p if p >= 3.0 => 1.1, // Very good
        p if p >= 2.0 => 1.0, // Good
        p if p >= 1.5 => 0.9, // Average
        p if p >= 1.0 => 0.8, // Below average
        _ => 0.6,             // Poor profit factor
    };

    // Factor 8: Relative Strength Assessment
    let rs_strength = (computed_values.rs_ma7 - computed_values.rs_ma30).abs();
    let rs_adjustment = match rs_strength {
        r if r >= 0.1 => 1.1,  // Strong relative strength signal
        r if r >= 0.05 => 1.0, // Moderate signal
        r if r >= 0.02 => 0.9, // Weak signal
        _ => 0.8,              // Very weak signal
    };

    // Factor 9: Price Extension (avoid overextended positions)
    let price_extension = computed_values.current_price / computed_values.ma30;
    let extension_adjustment = match price_extension {
        e if e <= 1.05 => 1.1, // Not extended
        e if e <= 1.10 => 1.0, // Slightly extended
        e if e <= 1.20 => 0.9, // Moderately extended
        e if e <= 1.30 => 0.8, // Extended
        _ => 0.6,              // Very extended
    };

    // Factor 10: ATR-based Risk Assessment
    let atr_risk_ratio = computed_values.atr_14 / computed_values.current_price;
    let atr_adjustment = match atr_risk_ratio {
        a if a <= 0.02 => 1.1, // Low ATR risk
        a if a <= 0.05 => 1.0, // Moderate ATR risk
        a if a <= 0.10 => 0.9, // High ATR risk
        a if a <= 0.15 => 0.8, // Very high ATR risk
        _ => 0.6,              // Extreme ATR risk
    };

    // Calculate weighted risk cap
    let adjustments: [f64; 10] = [
        sharpe_adjustment,
        drawdown_adjustment,
        win_rate_adjustment,
        volatility_adjustment,
        return_magnitude_adjustment,
        data_confidence_adjustment,
        profit_factor_adjustment,
        rs_adjustment,
        extension_adjustment,
        atr_adjustment,
    ];

    // Use geometric mean for more balanced risk assessment
    let geometric_mean =
        adjustments.iter().map(|&x| x.ln()).sum::<f64>() / adjustments.len() as f64;
    let combined_adjustment = geometric_mean.exp();

    // Apply bounds to prevent extreme risk caps
    let risk_cap = (base_risk * combined_adjustment).clamp(0.002_f64, 0.025_f64);

    // Round to reasonable precision
    (risk_cap * 1000.0).round() / 1000.0
}

fn determine_execution_mode(_asset: &str, stats: &StrategyAnalysis) -> ExecutionMode {
    // Determine execution mode based on quantitative metrics rather than asset names

    // Factor 1: Sharpe Ratio - higher Sharpe indicates more reliable signals
    let sharpe_factor = if stats.sharpe_ratio() >= 2.0 {
        1.0
    } else {
        0.5
    };

    // Factor 2: Win Rate - higher win rate indicates more consistent performance
    let win_rate_factor = if stats.win_rate() >= 0.80 {
        1.0
    } else if stats.win_rate() >= 0.60 {
        0.8
    } else {
        0.4
    };

    // Factor 3: Drawdown - lower drawdown indicates more stable performance
    let drawdown_factor = if stats.max_drawdown() <= 0.05 {
        1.0
    } else if stats.max_drawdown() <= 0.15 {
        0.7
    } else {
        0.3
    };

    // Factor 4: Trading Days - more data points indicate more reliable patterns
    let data_factor = if stats.trading_days() >= 15 {
        1.0
    } else if stats.trading_days() >= 10 {
        0.8
    } else {
        0.5
    };

    // Factor 5: Profit Factor - higher profit factor indicates better risk-reward
    let profit_factor = if stats.profit_factor() >= 3.0 {
        1.0
    } else if stats.profit_factor() >= 2.0 {
        0.8
    } else {
        0.5
    };

    // Calculate combined confidence score
    let confidence_score =
        (sharpe_factor + win_rate_factor + drawdown_factor + data_factor + profit_factor) / 5.0;

    // Allow pullback execution for high-confidence strategies
    let pullback_allowed = confidence_score >= 0.7;

    // Adjust extended threshold based on volatility and performance
    let extended_threshold = if stats.max_drawdown() <= 0.05 && stats.sharpe_ratio() >= 1.5 {
        0.15 // 15% for very stable, high-performing assets
    } else if stats.max_drawdown() <= 0.10 && stats.sharpe_ratio() >= 1.0 {
        0.10 // 10% for good performing assets
    } else {
        0.05 // 5% for volatile or lower-performing assets
    };

    // Adjust limit order duration based on confidence
    let limit_duration = if confidence_score >= 0.8 {
        72
    } else if confidence_score >= 0.6 {
        48
    } else {
        24
    };

    ExecutionMode {
        signal_at_close: true,
        pullback_to_ma30: pullback_allowed,
        extended_threshold,
        limit_order_duration_hours: limit_duration,
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn generate_computed_values(
    _asset: &str,
    stats: &StrategyAnalysis,
    execution_mode: &ExecutionMode,
    risk_cap: f64,
) -> ComputedValues {
    // Get the latest signal data for actual market values
    let signals = stats.signals();
    if signals.is_empty() {
        return ComputedValues::default();
    }

    let latest = &signals[signals.len() - 1];
    let current_price = latest.close();
    let ma30 = latest.ma_long().unwrap_or(current_price);
    let ma7 = latest.ma_short().unwrap_or(current_price);
    let rs_ma7 = latest.rs_ma_short().unwrap_or(1.0);
    let rs_ma30 = latest.rs_ma_long().unwrap_or(1.0);

    // Calculate ATR and volatility from recent signals
    let atr_14 = calculate_atr(signals, 14);
    let volatility = calculate_volatility(signals, 14);

    // Signal status
    let trend_signal = current_price > ma30;
    let momentum_signal = ma7 > ma30;
    let rs_signal = rs_ma7 > rs_ma30;
    let all_signals = trend_signal && momentum_signal && rs_signal;
    let partial_signals = rs_signal && (trend_signal || momentum_signal);

    // Position sizing calculations (assuming $100k portfolio for now)
    let portfolio_value = 100_000.0;
    let stop_price = 3.0f64.mul_add(-atr_14, current_price);
    let risk_per_share = current_price - stop_price;
    let max_shares_by_risk = (portfolio_value * risk_cap) / risk_per_share;
    let max_position_percent = risk_cap / (risk_per_share / current_price).max(0.01);
    let max_shares_by_position = (portfolio_value * max_position_percent.min(1.0)) / current_price;
    let recommended_shares = max_shares_by_risk.min(max_shares_by_position).floor() as u64;
    let position_value = recommended_shares as f64 * current_price;
    let position_percent = position_value / portfolio_value;

    // Profit taking calculations
    let profit_target = 2.0f64.mul_add(risk_per_share, current_price);
    let profit_target_percent = (profit_target / current_price - 1.0) * 100.0;
    let scale_out_shares = (recommended_shares as f64 * 0.5) as u64;
    let remaining_shares = recommended_shares - scale_out_shares;
    let scale_out_value = scale_out_shares as f64 * profit_target;

    // Stop loss levels
    let initial_stop = stop_price;
    let stop_loss_percent = (1.0 - stop_price / current_price) * 100.0;
    let trailing_stop = stop_price; // Will be updated daily
    let stop_distance_atr = 3.0;

    // Risk management
    let portfolio_risk = (recommended_shares as f64 * risk_per_share) / portfolio_value;
    let risk_reward_ratio = (profit_target - current_price) / risk_per_share;
    let max_loss = recommended_shares as f64 * risk_per_share;
    let max_gain = recommended_shares as f64 * (profit_target - current_price);

    // Execution parameters
    let is_extended = current_price > ma30 * (1.0 + execution_mode.extended_threshold);
    let ma30_pullback_price = ma30;
    let extended_percent = if is_extended {
        (current_price / ma30 - 1.0) * 100.0
    } else {
        0.0
    };
    let signal_strength = if all_signals {
        1.0
    } else if partial_signals {
        0.5
    } else {
        0.0
    };

    ComputedValues {
        // Current market data
        current_price,
        ma30,
        ma7,
        rs_ma7,
        rs_ma30,
        atr_14,
        volatility,

        // Signal status
        trend_signal,
        momentum_signal,
        rs_signal,
        all_signals,
        partial_signals,

        // Position sizing calculations
        stop_price,
        risk_per_share,
        max_shares_by_risk,
        max_shares_by_position,
        recommended_shares,
        position_value,
        position_percent,

        // Profit taking calculations
        profit_target,
        profit_target_percent,
        scale_out_shares,
        remaining_shares,
        scale_out_value,

        // Stop loss levels
        initial_stop,
        stop_loss_percent,
        trailing_stop,
        stop_distance_atr,

        // Risk management
        portfolio_risk,
        risk_reward_ratio,
        max_loss,
        max_gain,

        // Execution parameters
        is_extended,
        ma30_pullback_price,
        extended_percent,
        signal_strength,
    }
}

impl Default for ComputedValues {
    fn default() -> Self {
        Self {
            current_price: 0.0,
            ma30: 0.0,
            ma7: 0.0,
            rs_ma7: 0.0,
            rs_ma30: 0.0,
            atr_14: 0.0,
            volatility: 0.0,
            trend_signal: false,
            momentum_signal: false,
            rs_signal: false,
            all_signals: false,
            partial_signals: false,
            stop_price: 0.0,
            risk_per_share: 0.0,
            max_shares_by_risk: 0.0,
            max_shares_by_position: 0.0,
            recommended_shares: 0,
            position_value: 0.0,
            position_percent: 0.0,
            profit_target: 0.0,
            profit_target_percent: 0.0,
            scale_out_shares: 0,
            remaining_shares: 0,
            scale_out_value: 0.0,
            initial_stop: 0.0,
            stop_loss_percent: 0.0,
            trailing_stop: 0.0,
            stop_distance_atr: 0.0,
            portfolio_risk: 0.0,
            risk_reward_ratio: 0.0,
            max_loss: 0.0,
            max_gain: 0.0,
            is_extended: false,
            ma30_pullback_price: 0.0,
            extended_percent: 0.0,
            signal_strength: 0.0,
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn calculate_atr(signals: &[crate::analyzer::SignalRow], period: usize) -> f64 {
    if signals.len() < 2 {
        return 0.0;
    }

    let mut true_ranges = Vec::new();
    for i in 1..signals.len() {
        let current = &signals[i];
        let previous = &signals[i - 1];

        let high_low = current.close() - current.close(); // Simplified - would need high/low data
        let high_close = (current.close() - previous.close()).abs();
        let low_close = (current.close() - previous.close()).abs();

        let true_range = high_low.max(high_close).max(low_close);
        true_ranges.push(true_range);
    }

    if true_ranges.len() < period {
        return true_ranges.iter().sum::<f64>() / true_ranges.len() as f64;
    }

    // Calculate ATR as simple moving average of true ranges
    let recent_ranges = &true_ranges[true_ranges.len() - period..];
    recent_ranges.iter().sum::<f64>() / period as f64
}

#[allow(clippy::cast_precision_loss)]
fn calculate_volatility(signals: &[crate::analyzer::SignalRow], period: usize) -> f64 {
    if signals.len() < 2 {
        return 0.0;
    }

    let mut returns = Vec::new();
    for i in 1..signals.len() {
        let current = &signals[i];
        let previous = &signals[i - 1];
        let ret = (current.close() / previous.close() - 1.0).ln();
        returns.push(ret);
    }

    if returns.len() < period {
        return 0.0;
    }

    let recent_returns = &returns[returns.len() - period..];
    let mean = recent_returns.iter().sum::<f64>() / period as f64;
    let variance = recent_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / period as f64;

    variance.sqrt() * (252.0_f64).sqrt() // Annualized volatility
}

async fn generate_asset_notes_ai(
    asset: &str,
    stats: &StrategyAnalysis,
    computed_values: &ComputedValues,
) -> Result<String> {
    // Try to generate AI insights, fall back to basic metrics if API fails
    let metrics = AssetMetrics {
        asset: asset.to_string(),
        total_return: stats.total_return(),
        sharpe_ratio: stats.sharpe_ratio(),
        win_rate: stats.win_rate() * 100.0,
        max_drawdown: stats.max_drawdown() * 100.0,
        trading_days: stats.trading_days() as u32,
        profit_factor: stats.profit_factor(),
        current_price: computed_values.current_price,
        ma30: computed_values.ma30,
        ma7: computed_values.ma7,
        rs_ma7: computed_values.rs_ma7,
        rs_ma30: computed_values.rs_ma30,
        atr_14: computed_values.atr_14,
        volatility: computed_values.volatility,
    };
    
    match generate_asset_insights(&metrics).await {
        Ok(insights) => {
            let mut notes = Vec::new();
            notes.extend(insights.trading_notes);
            notes.push(format!("Risk: {}", insights.risk_assessment));
            notes.extend(insights.execution_recommendations);
            notes.push(format!("Context: {}", insights.market_context));
            Ok(notes.join("; "))
        }
        Err(e) => {
            println!(
                "⚠️  AI insights unavailable for {asset}: {e}. Using fallback analysis."
            );
            let fallback = generate_fallback_insights(
                asset,
                stats.total_return(),
                stats.sharpe_ratio(),
                stats.win_rate() * 100.0,
                stats.max_drawdown() * 100.0,
            );
            Ok(format!(
                "{}; Risk: {}; Recommendations: {}",
                fallback.trading_notes.join("; "),
                fallback.risk_assessment,
                fallback.execution_recommendations.join("; ")
            ))
        }
    }
}

fn generate_asset_notes(_asset: &str, stats: &StrategyAnalysis, _rank: usize) -> String {
    // Fallback for synchronous context - use basic performance-based notes
    let mut notes = Vec::new();

    // Performance-based notes
    if stats.total_return() > 10.0 {
        notes.push("High-momentum outlier; widen slippage buffer and enforce risk caps");
    }

    if stats.sharpe_ratio() > 4.0 {
        notes.push("Best risk-adjusted name in the strategy set");
    }

    if stats.max_drawdown() > 0.05 {
        notes.push("Higher volatility; consider earlier partials to reduce giveback");
    }

    if notes.is_empty() {
        "Standard execution with risk management".to_string()
    } else {
        notes.join("; ")
    }
}

/// Generate the top 10 trading playbooks from signal files.
///
/// # Errors
/// Returns an error if signal files cannot be read or processed.
///
/// # Panics
/// Panics if `partial_cmp` returns `None` when sorting by total return.
#[allow(clippy::cast_possible_truncation)]
pub async fn generate_top_10_playbooks(signals_dir: &str) -> Result<Vec<TradePlan>> {
    let analyses = analyze_signals_directory(signals_dir)?;

    // Filter profitable strategies and sort by total return
    let mut profitable: Vec<_> = analyses.iter().filter(|a| a.is_profitable()).collect();

    profitable.sort_by(|a, b| b.total_return().partial_cmp(&a.total_return()).unwrap());

    // Take top 10
    let mut top_10 = Vec::new();
    for (i, analysis) in profitable.iter().take(10).enumerate() {
        let playbook = TradePlan::from_analysis(analysis, i + 1).await.unwrap();
        top_10.push(playbook);
    }

    Ok(top_10)
}

pub fn print_top_10_playbooks(playbooks: &[TradePlan]) {
    println!("⸻");
    println!("Top-10 Playbooks");
    println!();
    println!("Shared definitions (from the ruleset):");
    println!("   • Signals");
    println!("     • Trend: close > MA30");
    println!("     • Momentum: MA7 > MA30");
    println!("     • RS (vs BTC): RS_MA7 > RS_MA30");
    println!("   • Position sizing");
    println!(
        "     • Full when 3/3 signals = 1.00 raw weight (equal-weighted across all qualifying assets that day)"
    );
    println!("     • Half when ≥2/3 AND RS bullish = 0.50 raw weight");
    println!(
        "     • Your portfolio normalizes across all \"raw>0\" names daily; optional BTC-short hedge if BTC bear state"
    );
    println!("   • Stops / targets");
    println!(
        "     • Initial stop: close – 3.0 × ATR14 (fallback: close × (1 − 2.5 × rolling_std14))"
    );
    println!("     • Trailing: ratchet stop to max(prior stop, close – 3.0 × ATR14) each day");
    println!(
        "     • Profit-taking: scale 50% at +2R (R = initial risk from entry to stop), then trail the rest until:"
    );
    println!("     • Hard exit if close < MA30 or RS flips bearish (RS_MA7 < RS_MA30)");
    println!();
    println!("\"Expected return\" below. Treat as historical sample, not forward projection.");
    println!();
    println!("⸻");
    println!();

    for (i, playbook) in playbooks.iter().enumerate() {
        let _ = playbook.print_playbook(i + 1);
    }

    println!("Execution detail");
    println!("   • Entry decision from analyzer output:");
    println!("     • if (trend && momentum && rs) -> full_weight");
    println!("     • else if (rs && (trend || momentum)) -> half_weight");
    println!(
        "   • For extended names (close / MA30 > 1.10): place GTC limit at MA30 for 24–48h; if unfilled but signals persist, promote to market-on-close next day."
    );
    println!("   • Position sizing (risk-based):");
    println!("     • R = entry_price − stop_price");
    println!(
        "     • units = min(raw_weight_normalized × portfolio_value / entry_price, (risk_cap × portfolio_value) / R)"
    );
    println!("     • Suggested risk caps above per asset (0.75–1.25%).");
    println!("   • Exit mechanics:");
    println!(
        "     • Scale: when close >= entry + 2R, sell 50%; raise stop to entry or entry + 0.5R."
    );
    println!(
        "     • Trend/RS fail (daily close): exit remainder at next bar open (or EOD close, to match backtest granularity)."
    );
    println!(
        "   • Conflict resolver (SOL vs wrappers): on any day where multiple SOL-linked tokens qualify, select the single highest RS_MA7/RS_MA30 spread; keep only that exposure."
    );
}

/// Save playbooks to a JSON file.
///
/// # Errors
/// Returns an error if the file cannot be written or serialized.
pub fn save_playbooks_to_json(playbooks: &[TradePlan], output_path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(
        &playbooks
            .iter()
            .enumerate()
            .map(|(i, p)| (p.asset.clone(), p.computed_values.clone(), p.print_playbook(i+1)))
            .collect::<Vec<_>>(),
    )?;
    fs::write(output_path, json)?;
    println!("Playbooks saved to: {output_path}");
    Ok(())
}

/// Execute the trading analysis and generate playbooks.
///
/// # Errors
/// Returns an error if signal files cannot be processed or if output files cannot be written.
pub async fn execute(signals_dir: &str, output_json: Option<&str>) -> Result<()> {
    println!("🎯 Generating Top-10 Trading Playbooks");
    println!("Analyzing signals from: {signals_dir}");
    println!();

    let playbooks = generate_top_10_playbooks(signals_dir).await?;

    if playbooks.is_empty() {
        println!("❌ No profitable strategies found to generate playbooks!");
        return Ok(());
    }

    print_top_10_playbooks(&playbooks);

    if let Some(json_path) = output_json {
        save_playbooks_to_json(&playbooks, json_path)?;
    }

    Ok(())
}
