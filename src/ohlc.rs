use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use csv::{ReaderBuilder, WriterBuilder};
use itertools::Itertools;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashSet, env, fs, io::Write, path::Path, time::Duration};
use tokio::time::sleep;
use tracing::{error, info};

use fs2::FileExt; // for file locking
use std::fs::OpenOptions;
use tempfile::NamedTempFile;

use crate::OhlcArgs;

/// Market coin
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketCoin {
    id: String,
    symbol: String,
    name: String,
    market_cap_rank: Option<u32>,
}

/// OHLC row: [timestamp_ms, open, high, low, close]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcRaw(
    #[serde(deserialize_with = "de_f64_or_i64")] f64,
    f64,
    f64,
    f64,
    f64,
);

// Helper for timestamp that may arrive as f64 or i64
pub fn de_f64_or_i64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Num {
        F(f64),
        I(i64),
    }
    match Num::deserialize(deserializer)? {
        Num::F(v) => Ok(v),
        Num::I(v) => Ok(v as f64),
    }
}

/// In-memory normalized daily row
#[derive(Clone, Debug)]
pub struct DailyBar {
    date: NaiveDate,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

pub async fn execute(args: &OhlcArgs) -> Result<()> {
    let api_key = match args.api_key.clone() {
        Some(key) => key,
        None => env::var("CG_PRO_API_KEY").ok().unwrap(),
    };

    let out_dir = args.out.as_ref().unwrap();
    fs::create_dir_all(out_dir)
        .context("create output dir")
        .unwrap();

    // Optional single-instance lock (covers daemon & cron)
    let _lock_guard = args
        .lock_file
        .as_ref()
        .map(|lock_path| acquire_lock(lock_path).unwrap());

    let client = mk_client(&api_key).unwrap();

    // Default end date to yesterday if not provided (to avoid "future date" API error)
    let end = if let Some(end_str) = &args.end {
        NaiveDate::parse_from_str(end_str, "%Y-%m-%d")
            .context("invalid --end")
            .unwrap()
    } else {
        chrono::Local::now().date_naive() - chrono::Duration::days(1)
    };

    // Default start date to 30 days ago if not provided
    let start = if let Some(start_str) = &args.start {
        NaiveDate::parse_from_str(start_str, "%Y-%m-%d")
            .context("invalid --start")
            .unwrap()
    } else {
        // Ensure start is at least 1 day before end
        let default_start = end - chrono::Duration::days(30);
        if default_start >= end {
            end - chrono::Duration::days(1)
        } else {
            default_start
        }
    };

    if let Some(hhmm) = args.daily_at.clone() {
        // Daemon mode: run now (once), then sleep until next HH:MM each day
        let hhmm = parse_hhmm(&hhmm)
            .context("invalid --daily-at (expected HH:MM)")
            .unwrap();
        loop {
            run_once(&client, args, start, end).await.unwrap();
            // Sleep to next occurrence of hh:mm local time
            let dur = duration_until_next_local(hhmm).unwrap();
            info!("sleeping until next daily run: {}s", dur.as_secs());
            sleep(dur).await;
        }
    } else {
        // One-shot (use with cron/systemd/launchd)
        run_once(&client, args, start, end).await.unwrap();
    }
    // (unreachable in daemon loop)
    // lock guard drops here automatically

    Ok(())
}

pub async fn run_once(
    client: &Client,
    args: &OhlcArgs,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<()> {
    info!(
        "starting run (resume={}, start={}, end={})",
        args.resume.unwrap_or(false),
        start,
        end
    );
    let start_ts = Utc
        .from_utc_datetime(&start.and_hms_opt(0, 0, 0).unwrap())
        .timestamp();
    let end_ts = Utc
        .from_utc_datetime(&end.and_hms_opt(23, 59, 59).unwrap())
        .timestamp();
    if end_ts <= start_ts {
        bail!("end must be after start");
    }

    // Gather coins: always include BTC unless skipped
    let mut coins: Vec<MarketCoin> = if args.skip_btc.unwrap_or(false) {
        vec![]
    } else {
        vec![MarketCoin {
            id: "bitcoin".into(),
            symbol: "btc".into(),
            name: "Bitcoin".into(),
            market_cap_rank: Some(1),
        }]
    };

    let vs = args.vs.as_ref().unwrap();
    let top_n = args.top_n.unwrap();
    let top = fetch_top_by_mcap(client, vs, top_n).await?;
    let _coins = coins.clone();
    let existing: HashSet<&str> = _coins.iter().map(|c| c.id.as_str()).collect();
    coins.extend(
        top.into_iter()
            .filter(|c| !existing.contains(c.id.as_str())),
    );

    if args.write_manifest.unwrap_or(true) {
        let out_dir = args.out.as_ref().unwrap();
        fs::write(
            out_dir.join("manifest.json"),
            serde_json::to_string_pretty(&coins)?,
        )?;
    }

    // BTC first (optional)
    if !args.skip_btc.unwrap_or(false) {
        let out_dir = args.out.as_ref().unwrap();
        let path = out_dir.join("BTC.csv");
        let request_delay = args.request_delay_ms.unwrap();
        let resume = args.resume.unwrap_or(false);
        update_csv_for_coin(
            client,
            vs,
            "bitcoin",
            "BTC",
            &path,
            start_ts,
            end_ts,
            request_delay,
            resume,
        )
        .await?;
    }

    // Parallel fetch with bounded concurrency
    use tokio::sync::Semaphore;
    let concurrency = args.concurrency.unwrap();
    let sem = std::sync::Arc::new(Semaphore::new(concurrency));
    let mut tasks = vec![];
    for c in coins.into_iter().filter(|c| c.id != "bitcoin") {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let vs = vs.clone();
        let out = args.out.as_ref().unwrap().clone();
        let sym = c.symbol.to_uppercase();
        let id = c.id.clone();
        let delay = args.request_delay_ms.unwrap();
        let resume = args.resume.unwrap_or(false);

        let task = tokio::spawn(async move {
            let _p = permit;
            let path = out.join(format!("{}_{}.csv", sym, id));
            if let Err(e) = update_csv_for_coin(
                &client, &vs, &id, &sym, &path, start_ts, end_ts, delay, resume,
            )
            .await
            {
                error!("failed {} ({}): {}", sym, id, e);
            }
        });
        tasks.push(task);
    }
    for t in tasks {
        let _ = t.await;
    }

    info!("run complete");
    Ok(())
}

/// Acquire an exclusive file lock; keep the file handle alive to hold the lock.
pub fn acquire_lock(lock_path: &Path) -> Result<std::fs::File> {
    fs::create_dir_all(lock_path.parent().unwrap_or(Path::new("."))).ok();
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(lock_path)?;
    file.lock_exclusive()?;
    Ok(file)
}

/// Parse HH:MM to NaiveTime
pub fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    let parts: Vec<_> = s.split(':').collect();
    if parts.len() != 2 {
        bail!("bad time format");
    }
    let h: u32 = parts[0].parse()?;
    let m: u32 = parts[1].parse()?;
    NaiveTime::from_hms_opt(h, m, 0).context("invalid hh:mm")
}

/// Duration until the next local occurrence of time `t`
pub fn duration_until_next_local(t: NaiveTime) -> Result<Duration> {
    use chrono::Local;
    let now_local = Local::now();
    let today_target = now_local.date_naive().and_time(t);
    let next = if now_local.naive_local() < today_target {
        Local.from_local_datetime(&today_target).unwrap()
    } else {
        // tomorrow
        let tomorrow = now_local.date_naive().succ_opt().unwrap();
        Local.from_local_datetime(&tomorrow.and_time(t)).unwrap()
    };
    let dur = next - now_local;
    Ok(Duration::from_millis(dur.num_milliseconds().max(0) as u64))
}

/// Make an HTTP client with Pro key header
pub fn mk_client(api_key: &str) -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert("x-cg-pro-api-key", header::HeaderValue::from_str(api_key)?);
    let client = Client::builder()
        .default_headers(headers)
        .user_agent("cg_ohlc_exporter/0.2 (rust)")
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?;
    Ok(client)
}

/// Fetch top-N by market cap
pub async fn fetch_top_by_mcap(client: &Client, vs: &str, top_n: usize) -> Result<Vec<MarketCoin>> {
    let base = "https://pro-api.coingecko.com/api/v3/coins/markets";
    let mut page = 1usize;
    let mut out = vec![];
    while out.len() < top_n {
        let per = min(250, top_n - out.len());
        let url = reqwest::Url::parse_with_params(
            base,
            &[
                ("vs_currency", vs),
                ("order", "market_cap_desc"),
                ("per_page", &per.to_string()),
                ("page", &page.to_string()),
                ("price_change_percentage", "24h"),
                ("sparkline", "false"),
            ],
        )?;
        let resp = do_get_json::<Vec<serde_json::Value>>(client, url).await?;
        let mut batch = vec![];
        for v in resp {
            let mc = MarketCoin {
                id: v
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                symbol: v
                    .get("symbol")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                name: v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                market_cap_rank: v
                    .get("market_cap_rank")
                    .and_then(|x| x.as_u64())
                    .map(|x| x as u32),
            };
            if !mc.id.is_empty() {
                batch.push(mc);
            }
        }
        if batch.is_empty() {
            break;
        }
        out.extend(batch);
        page += 1;
    }
    out.sort_by_key(|c| c.market_cap_rank.unwrap_or(u32::MAX));
    out.truncate(top_n);
    Ok(out)
}

/// Build URL for chunked OHLC range
pub fn ohlc_range_url(coin_id: &str, vs: &str, from_ts: i64, to_ts: i64) -> reqwest::Url {
    let base = format!(
        "https://pro-api.coingecko.com/api/v3/coins/{}/ohlc/range",
        coin_id
    );
    reqwest::Url::parse_with_params(
        &base,
        &[
            ("vs_currency", vs.to_string()),
            ("from", from_ts.to_string()),
            ("to", to_ts.to_string()),
            ("interval", "daily".into()),
        ],
    )
    .unwrap()
}

/// Core HTTP GET with retry/backoff (+Retry-After)
pub async fn do_get_json<T: for<'de> serde::Deserialize<'de>>(
    client: &Client,
    url: reqwest::Url,
) -> Result<T> {
    let mut attempt = 0usize;
    loop {
        let resp = client.get(url.clone()).send().await?;
        if resp.status().is_success() {
            return Ok(resp.json::<T>().await?);
        }
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        attempt += 1;
        if attempt > 6 {
            let txt = resp.text().await.unwrap_or_default();
            bail!("HTTP {} after retries; body: {}", status, txt);
        }
        let backoff_ms = retry_after
            .map(|s| s * 1000)
            .unwrap_or(300 * attempt as u64);
        info!("{} -> retrying in {}ms", status, backoff_ms);
        sleep(Duration::from_millis(backoff_ms)).await;
    }
}

/// Idempotent CSV update: fetch missing rows and append atomically.
/// If !resume or file doesn't exist: write fresh file.
/// Ensures daily dedupe by date.
#[allow(clippy::too_many_arguments)]
pub async fn update_csv_for_coin(
    client: &Client,
    vs: &str,
    coin_id: &str,
    symbol: &str,
    out_path: &Path,
    start_ts: i64,
    end_ts: i64,
    delay_ms: u64,
    resume: bool,
) -> Result<()> {
    fs::create_dir_all(out_path.parent().unwrap_or(Path::new("."))).ok();

    // Determine per-asset effective start using CSV last date (if resume)
    let mut eff_start_ts = start_ts;
    let last_date = if resume {
        read_last_csv_date(out_path).ok().flatten()
    } else {
        None
    };
    if let Some(ld) = last_date {
        let next = ld.succ_opt().unwrap();
        eff_start_ts = Utc
            .from_utc_datetime(&next.and_hms_opt(0, 0, 0).unwrap())
            .timestamp();
        if eff_start_ts > end_ts {
            info!("{} up-to-date through {}; skipping", symbol, ld);
            return Ok(());
        }
    }

    // Fetch chunked OHLC rows
    let mut rows = fetch_ohlc_rows(client, vs, coin_id, eff_start_ts, end_ts, delay_ms).await?;

    // If resume and file exists, drop any overlapping dates (defensive)
    if resume
        && out_path.exists()
        && let Some(ld) = last_date
    {
        rows.retain(|r| r.date > ld);
    }

    if rows.is_empty() {
        info!("{} no new rows", symbol);
        return Ok(());
    }

    // Append or create, atomically
    if out_path.exists() && resume {
        // append without headers
        let mut f = OpenOptions::new().append(true).open(out_path)?;
        for r in rows {
            writeln!(
                f,
                "{},{:.8},{:.8},{:.8},{:.8}",
                r.date.format("%Y-%m-%d"),
                r.open,
                r.high,
                r.low,
                r.close
            )?;
        }
        f.flush()?;
    } else {
        // write fresh file to temp, then rename
        let mut tmp = NamedTempFile::new_in(out_path.parent().unwrap_or(Path::new(".")))?;
        {
            let mut wtr = WriterBuilder::new().from_writer(tmp.as_file_mut());
            wtr.write_record(["date", "open", "high", "low", "close"])?;
            for r in rows {
                wtr.write_record(&[
                    r.date.format("%Y-%m-%d").to_string(),
                    format!("{:.8}", r.open),
                    format!("{:.8}", r.high),
                    format!("{:.8}", r.low),
                    format!("{:.8}", r.close),
                ])?;
            }
            wtr.flush()?;
        }
        tmp.persist(out_path)?;
    }

    info!("wrote {}", out_path.display());
    Ok(())
}

/// Return vector of normalized DailyBar for [`from_ts..=to_ts`], deduped per date (pick last candle/day)
///
/// # Errors
/// Returns an error if the API request fails or if the response cannot be parsed.
///
/// # Panics
/// Panics if `partial_cmp` returns `None` when sorting timestamps.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub async fn fetch_ohlc_rows(
    client: &Client,
    vs: &str,
    coin_id: &str,
    from_ts: i64,
    to_ts: i64,
    delay_ms: u64,
) -> Result<Vec<DailyBar>> {
    let mut cur_from = from_ts;
    let one_day = 86_400i64;
    let max_days = 180i64;
    let mut raws: Vec<OhlcRaw> = vec![];

    while cur_from < to_ts {
        let cur_to = (cur_from + max_days * one_day).min(to_ts);
        let url = ohlc_range_url(coin_id, vs, cur_from, cur_to);
        let val = do_get_json::<serde_json::Value>(client, url).await?;
        if let Some(arr) = val.as_array() {
            for r in arr {
                if let Some(a) = r.as_array()
                    && a.len() >= 5
                {
                    let ts_ms = a[0]
                        .as_f64()
                        .or_else(|| a[0].as_i64().map(|x| x as f64))
                        .unwrap_or(0.0);
                    let o = a[1].as_f64().unwrap_or(0.0);
                    let h = a[2].as_f64().unwrap_or(0.0);
                    let l = a[3].as_f64().unwrap_or(0.0);
                    let c = a[4].as_f64().unwrap_or(0.0);
                    raws.push(OhlcRaw(ts_ms, o, h, l, c));
                }
            }
        }
        sleep(Duration::from_millis(delay_ms)).await;
        cur_from = cur_to + 1;
    }

    // Normalize to daily bars keyed by date, pick last per date
    raws.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut out = vec![];
    for (_date, group) in &raws.into_iter().chunk_by(|r| {
        let ts = (r.0 / 1000.0) as i64; // ms -> s
        Utc.timestamp_opt(ts, 0).unwrap().date_naive()
    }) {
        if let Some(last) = group.last() {
            let ts = (last.0 / 1000.0) as i64;
            let d = Utc.timestamp_opt(ts, 0).unwrap().date_naive();
            out.push(DailyBar {
                date: d,
                open: last.1,
                high: last.2,
                low: last.3,
                close: last.4,
            });
        }
    }
    Ok(out)
}

/// Read the last date from a CSV file.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn read_last_csv_date(path: &Path) -> Result<Option<NaiveDate>> {
    if !path.exists() {
        return Ok(None);
    }
    // Fast path: read backwards; for simplicity we read all and take last (files are small daily)
    let mut rdr = ReaderBuilder::new().from_path(path)?;
    let mut last: Option<NaiveDate> = None;
    for rec in rdr.records() {
        let r = rec?;
        if r.is_empty() {
            continue;
        }
        let d = NaiveDate::parse_from_str(&r[0], "%Y-%m-%d").ok();
        if d.is_some() {
            last = d;
        }
    }
    Ok(last)
}
