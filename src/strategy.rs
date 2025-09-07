use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use csv::{ReaderBuilder, WriterBuilder};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use statrs::statistics::Statistics;
use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::StrategyArgs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    date: NaiveDate,
    #[serde(default)]
    open: Option<f64>,
    #[serde(default)]
    high: Option<f64>,
    #[serde(default)]
    low: Option<f64>,
    close: f64,
}

#[derive(Clone)]
pub struct Series {
    dates: Vec<NaiveDate>,
    close: Vec<f64>,
    high: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
}

pub fn read_series(path: &PathBuf) -> Result<Series> {
    let mut rdr = ReaderBuilder::new().trim(csv::Trim::All).from_path(path)?;
    let mut dates = Vec::new();
    let mut close = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();

    for rec in rdr.deserialize::<Row>() {
        let r = rec?;
        dates.push(r.date);
        close.push(r.close);
        high.push(r.high);
        low.push(r.low);
    }
    Ok(Series {
        dates,
        close,
        high,
        low,
    })
}

pub fn rolling_ma(x: &[f64], w: usize) -> Vec<Option<f64>> {
    if w == 0 {
        return vec![None; x.len()];
    }
    let mut out = vec![None; x.len()];
    let mut sum = 0.0;
    for i in 0..x.len() {
        sum += x[i];
        if i >= w {
            sum -= x[i - w];
        }
        if i + 1 >= w {
            out[i] = Some(sum / w as f64);
        }
    }
    out
}

pub fn true_range(high: f64, low: f64, prev_close: f64) -> f64 {
    (high - low)
        .abs()
        .max((high - prev_close).abs())
        .max((low - prev_close).abs())
}

pub fn rolling_atr(
    high: &[Option<f64>],
    low: &[Option<f64>],
    close: &[f64],
    w: usize,
) -> Vec<Option<f64>> {
    let mut out = vec![None; close.len()];
    let mut trs: Vec<f64> = Vec::with_capacity(close.len());
    for i in 0..close.len() {
        if i == 0 {
            trs.push(match (high[i], low[i]) {
                (Some(h), Some(l)) => (h - l).abs(),
                _ => 0.0,
            });
        } else {
            match (high[i], low[i]) {
                (Some(h), Some(l)) => trs.push(true_range(h, l, close[i - 1])),
                _ => trs.push((close[i] - close[i - 1]).abs()),
            }
        }
        if i + 1 >= w {
            let start = i + 1 - w;
            let slice = &trs[start..=i];
            out[i] = Some(slice.iter().sum::<f64>() / w as f64);
        }
    }
    out
}

pub fn rolling_std(returns: &[f64], w: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; returns.len()];
    for i in 0..returns.len() {
        if i + 1 >= w {
            let s = &returns[i + 1 - w..=i];
            let mean = s.mean();
            let var = s.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (s.len() as f64).max(1.0);
            out[i] = Some(var.sqrt());
        }
    }
    out
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DailySignal {
    date: NaiveDate,
    price: f64,
    ma_short: Option<f64>,
    ma_long: Option<f64>,
    rs: Option<f64>,
    rs_ma_short: Option<f64>,
    rs_ma_long: Option<f64>,
    trend_bull: bool,
    mom_bull: bool,
    rs_bull: bool,
    score: usize,
    raw_weight: f64,
    stop_level: Option<f64>,
}

pub fn intersect_dates(series: &[Series]) -> Vec<NaiveDate> {
    use std::collections::BTreeSet;
    if series.is_empty() {
        return vec![];
    }
    let sets: Vec<BTreeSet<NaiveDate>> = series
        .iter()
        .map(|s| s.dates.iter().cloned().collect::<BTreeSet<_>>())
        .collect();
    let mut iter = sets.into_iter();
    let mut base = if let Some(first) = iter.next() {
        first
    } else {
        return vec![];
    };
    for s in iter {
        base = base.intersection(&s).cloned().collect();
    }
    base.into_iter().collect()
}

pub fn execute(args: &StrategyArgs) -> Result<()> {
    let out_dir = args.out.as_ref().unwrap();
    fs::create_dir_all(out_dir).context("create out dir")?;

    let btc_path = args.btc.as_ref().unwrap();
    let btc = read_series(btc_path).context("read BTC")?;
    let assets_paths = args.assets.as_ref().unwrap();
    let min_required_days = args.ma_long.unwrap() + 10;
    let mut assets: Vec<(String, Series)> = Vec::new();
    
    for p in assets_paths {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let series = read_series(p)?;
        if series.dates.len() >= min_required_days {
            assets.push((name, series));
        } else {
            println!("Skipping {} (only {} days, need {})", name, series.dates.len(), min_required_days);
        }
    }
    
    println!("Using {} assets with sufficient data", assets.len());

    // Build common date index across BTC + all assets
    let mut all = vec![btc.clone()];
    all.extend(assets.iter().map(|(_, s)| s.clone()));
    let dates = intersect_dates(&all);
    let ma_long = args.ma_long.unwrap();
    if dates.len() < ma_long + 10 {
        bail!("Not enough overlapping data after alignment.");
    }

    // Index maps
    let btc_idx: BTreeMap<NaiveDate, usize> =
        btc.dates.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let btc_close: Vec<f64> = dates
        .iter()
        .map(|d| btc.close[*btc_idx.get(d).unwrap()])
        .collect();
    let ma_short = args.ma_short.unwrap();
    let btc_ma_s = rolling_ma(&btc_close, ma_short);
    let btc_ma_l = rolling_ma(&btc_close, ma_long);
    let btc_mkt_bear: Vec<bool> = dates
        .iter()
        .enumerate()
        .map(|(i, _)| match (btc_ma_s[i], btc_ma_l[i]) {
            (Some(s), Some(l)) => btc_close[i] < l && s < l,
            _ => false,
        })
        .collect();

    // For portfolio aggregation
    let mut daily_port_ret: Vec<f64> = vec![0.0; dates.len()];
    let mut daily_port_poscount: Vec<usize> = vec![0; dates.len()];
    let mut per_asset_signals: BTreeMap<String, Vec<DailySignal>> = BTreeMap::new();

    for (name, ser) in assets.iter() {
        // Map to aligned series
        let idx: BTreeMap<NaiveDate, usize> =
            ser.dates.iter().enumerate().map(|(i, d)| (*d, i)).collect();
        let a_close: Vec<f64> = dates
            .iter()
            .map(|d| ser.close[*idx.get(d).unwrap()])
            .collect();
        let a_high: Vec<Option<f64>> = dates
            .iter()
            .map(|d| ser.high[*idx.get(d).unwrap()])
            .collect();
        let a_low: Vec<Option<f64>> = dates
            .iter()
            .map(|d| ser.low[*idx.get(d).unwrap()])
            .collect();

        // MAs
        let a_ma_s = rolling_ma(&a_close, ma_short);
        let a_ma_l = rolling_ma(&a_close, ma_long);

        // Relative strength line and its MAs
        let rs: Vec<f64> = a_close
            .iter()
            .zip(btc_close.iter())
            .map(|(a, b)| a / b)
            .collect();
        let rs_ma_s = rolling_ma(&rs, ma_short);
        let rs_ma_l = rolling_ma(&rs, ma_long);

        // Stops (ATR if possible else vol of returns)
        let stop_lookback = args.stop_lookback.unwrap();
        let atr = rolling_atr(&a_high, &a_low, &a_close, stop_lookback);
        let daily_ret: Vec<f64> = std::iter::once(&a_close[0])
            .chain(a_close.iter().skip(1))
            .tuple_windows()
            .map(|(prev, next)| (next - prev) / prev)
            .collect::<Vec<_>>();
        let ret_std = rolling_std(&daily_ret, stop_lookback);

        let mut signals = Vec::with_capacity(dates.len());
        for i in 0..dates.len() {
            let trend_bull = a_ma_l[i].map(|l| a_close[i] > l).unwrap_or(false);
            let mom_bull = match (a_ma_s[i], a_ma_l[i]) {
                (Some(s), Some(l)) => s > l,
                _ => false,
            };
            let rs_bull = match (rs_ma_s[i], rs_ma_l[i]) {
                (Some(s), Some(l)) => s > l,
                _ => false,
            };
            let score = [trend_bull, mom_bull, rs_bull]
                .iter()
                .filter(|x| **x)
                .count();

            // raw weight: +1 for 3/3, +0.5 for >= min_signals with rs_bull, else 0 (or -1 on 3/3 bear if short_alts)
            let mut raw = 0.0;
            if score == 3 {
                raw = 1.0;
            } else if score >= args.min_signals.unwrap() && rs_bull {
                raw = 0.5;
            } else if args.short_alts.unwrap_or(false) {
                // full-bear: 3/3 bearish
                let trend_bear = a_ma_l[i].map(|l| a_close[i] < l).unwrap_or(false);
                let mom_bear = match (a_ma_s[i], a_ma_l[i]) {
                    (Some(s), Some(l)) => s < l,
                    _ => false,
                };
                let rs_bear = match (rs_ma_s[i], rs_ma_l[i]) {
                    (Some(s), Some(l)) => s < l,
                    _ => false,
                };
                if trend_bear && mom_bear && rs_bear {
                    raw = -1.0;
                }
            }

            // Stop level
            let stop = atr[i]
                .filter(|&atrv| atrv > 0.0)
                .map(|atrv| a_close[i] - args.atr_mult.unwrap() * atrv)
                .or_else(|| {
                    if i > 0 {
                        ret_std[i - 1].map(|sd| a_close[i] * (1.0 - args.vol_mult.unwrap() * sd))
                    } else {
                        None
                    }
                });

            signals.push(DailySignal {
                date: dates[i],
                price: a_close[i],
                ma_short: a_ma_s[i],
                ma_long: a_ma_l[i],
                rs: Some(rs[i]),
                rs_ma_short: rs_ma_s[i],
                rs_ma_long: rs_ma_l[i],
                trend_bull,
                mom_bull,
                rs_bull,
                score,
                raw_weight: raw,
                stop_level: stop,
            });
        }

        // Export signals CSV
        fs::create_dir_all(out_dir)?;
        let mut wtr =
            WriterBuilder::new().from_path(out_dir.join(format!("signals_{}.csv", name)))?;
        wtr.write_record([
            "date",
            "close",
            "ma_short",
            "ma_long",
            "rs",
            "rs_ma_short",
            "rs_ma_long",
            "trend_bull",
            "mom_bull",
            "rs_bull",
            "score",
            "raw_weight",
            "stop_level",
        ])?;
        for s in &signals {
            wtr.write_record(&[
                s.date.to_string(),
                format!("{:.8}", s.price),
                s.ma_short.map(|v| format!("{:.8}", v)).unwrap_or_default(),
                s.ma_long.map(|v| format!("{:.8}", v)).unwrap_or_default(),
                s.rs.map(|v| format!("{:.8}", v)).unwrap_or_default(),
                s.rs_ma_short
                    .map(|v| format!("{:.8}", v))
                    .unwrap_or_default(),
                s.rs_ma_long
                    .map(|v| format!("{:.8}", v))
                    .unwrap_or_default(),
                s.trend_bull.to_string(),
                s.mom_bull.to_string(),
                s.rs_bull.to_string(),
                s.score.to_string(),
                format!("{:.4}", s.raw_weight),
                s.stop_level
                    .map(|v| format!("{:.8}", v))
                    .unwrap_or_default(),
            ])?;
        }
        wtr.flush()?;
        per_asset_signals.insert(name.clone(), signals);
    }

    // Portfolio construction: normalize long weights daily, optional BTC hedge on market-bear
    // If all raw weights <=0 (no longs), portfolio goes to cash unless btc_hedge triggers a short BTC hedge.
    // Daily portfolio return is sum_i(weight_i * asset_return_i) + hedge
    // We also enforce stop: if close < stop on the day, set that asset's weight to 0 for that day.
    let mut equity: Vec<f64> = vec![1.0; dates.len()];
    for i in 1..dates.len() {
        // Gather candidate longs
        let mut longs: Vec<(String, f64)> = Vec::new();
        for (name, sigs) in per_asset_signals.iter() {
            let s_prev = &sigs[i - 1]; // enter based on prev day’s signal
            let s_now = &sigs[i];
            // stop trigger
            let stopped =
                matches!((s_prev.stop_level, Some(s_now.price)), (Some(stp), Some(px)) if px < stp);
            let w = if stopped {
                0.0
            } else {
                s_prev.raw_weight.max(0.0)
            };
            if w > 0.0 {
                longs.push((name.clone(), w));
            }
        }
        let long_sum: f64 = longs.iter().map(|(_, w)| *w).sum();
        let mut weights: BTreeMap<String, f64> = BTreeMap::new();
        if long_sum > 0.0 {
            for (name, w) in longs {
                weights.insert(name, w / long_sum);
            }
        }

        // BTC hedge
        let mut hedge_ret = 0.0;
        if args.btc_hedge.unwrap() > 0.0 && btc_mkt_bear[i - 1] {
            // short BTC @ weight = btc_hedge, P&L = -hedge * btc_return
            let r_btc = (btc_close[i] - btc_close[i - 1]) / btc_close[i - 1];
            hedge_ret += -args.btc_hedge.unwrap() * r_btc;
        }

        // Compute daily return
        let mut port_ret = hedge_ret;
        for (name, w) in weights.iter() {
            let sigs = per_asset_signals.get(name).unwrap();
            let r = (sigs[i].price - sigs[i - 1].price) / sigs[i - 1].price;
            port_ret += w * r;
        }

        equity[i] = equity[i - 1] * (1.0 + port_ret);
        daily_port_ret[i] = port_ret;
        daily_port_poscount[i] = weights.len();
    }

    // Write equity curve
    let mut wtr_eq = WriterBuilder::new().from_path(out_dir.join("equity_curve.csv"))?;
    wtr_eq.write_record(["date", "equity", "port_ret", "num_positions", "btc_close"])?;
    for i in 0..dates.len() {
        wtr_eq.write_record(&[
            dates[i].to_string(),
            format!("{:.8}", equity[i]),
            format!("{:.8}", daily_port_ret[i]),
            daily_port_poscount[i].to_string(),
            format!("{:.2}", btc_close[i]),
        ])?;
    }
    wtr_eq.flush()?;

    // Metrics
    let total_ret = equity.last().unwrap() - 1.0;
    let n_days = dates.len().max(1);
    let years = (n_days as f64) / 365.25;
    let cagr = if years > 0.0 {
        equity.last().unwrap().powf(1.0 / years) - 1.0
    } else {
        0.0
    };
    // Sharpe (daily, then annualize sqrt(365))
    let rets: Vec<f64> = daily_port_ret
        .iter()
        .cloned()
        .filter(|x| x.is_finite() && *x != 0.0)
        .collect();
    let mean = if rets.is_empty() {
        0.0
    } else {
        rets.clone().mean()
    };
    let sd = if rets.len() > 1 {
        let m = mean;
        (rets.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (rets.len() as f64 - 1.0)).sqrt()
    } else {
        0.0
    };
    let sharpe = if sd > 0.0 {
        (mean / sd) * 365.25_f64.sqrt()
    } else {
        0.0
    };

    // Max drawdown
    let mut peak = f64::MIN;
    let mut mdd = 0.0;
    for &e in &equity {
        if e > peak {
            peak = e;
        }
        let dd = 1.0 - (e / peak);
        if dd > mdd {
            mdd = dd;
        }
    }

    // Win rate
    let wins = rets.iter().filter(|r| **r > 0.0).count() as f64;
    let wr = if !rets.is_empty() {
        wins / (rets.len() as f64)
    } else {
        0.0
    };

    let metrics = format!(
        "Days: {}\nTotal Return: {:.2}%\nCAGR: {:.2}%\nSharpe (ann.): {:.2}\nMax Drawdown: {:.2}%\nWin Rate: {:.2}%\n",
        n_days,
        total_ret * 100.0,
        cagr * 100.0,
        sharpe,
        mdd * 100.0,
        wr * 100.0
    );
    fs::write(out_dir.join("metrics.txt"), metrics.clone())?;
    println!("{}", metrics);

    Ok(())
}
