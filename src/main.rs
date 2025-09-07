use std::path::PathBuf;

use anyhow::Result;
use crypto_strategy::{OhlcArgs, StrategyArgs, ohlc, strategy};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ohlc(OhlcArgs),
    Strategy(StrategyArgs),
}

fn get_default_ohlc_args() -> OhlcArgs {
    OhlcArgs {
        out: Some(PathBuf::from("./out")),
        api_key: None,
        top_n: Some(100),
        vs: Some("usd".to_string()),
        start: None,
        end: None,
        concurrency: Some(6),
        request_delay_ms: Some(250),
        write_manifest: Some(true),
        resume: Some(false),
        daily_at: None,
        lock_file: None,
        skip_btc: Some(false),
    }
}

fn apply_ohlc_defaults(args: &mut OhlcArgs) {
    if args.out.is_none() {
        args.out = Some(PathBuf::from("./out"));
    }
    if args.top_n.is_none() {
        args.top_n = Some(100);
    }
    if args.vs.is_none() {
        args.vs = Some("usd".to_string());
    }
    if args.concurrency.is_none() {
        args.concurrency = Some(6);
    }
    if args.request_delay_ms.is_none() {
        args.request_delay_ms = Some(250);
    }
    if args.write_manifest.is_none() {
        args.write_manifest = Some(true);
    }
    if args.resume.is_none() {
        args.resume = Some(false);
    }
    if args.skip_btc.is_none() {
        args.skip_btc = Some(false);
    }
}

fn get_default_strategy_args() -> StrategyArgs {
    StrategyArgs {
        btc: Some(PathBuf::from("./out/BTC.csv")),
        assets: Some(vec![
            PathBuf::from("./out/LINK_chainlink.csv"),
            PathBuf::from("./out/MNT_mantle.csv"),
            PathBuf::from("./out/ADA_cardano.csv"),
        ]),
        out: Some(PathBuf::from("./out")),
        ma_short: Some(5),
        ma_long: Some(15),
        min_signals: Some(2),
        short_alts: Some(false),
        btc_hedge: Some(0.3),
        stop_lookback: Some(14),
        atr_mult: Some(3.0),
        vol_mult: Some(2.5),
    }
}

fn apply_strategy_defaults(args: &mut StrategyArgs) {
    if args.btc.is_none() {
        args.btc = Some(PathBuf::from("./out/BTC.csv"));
    }
    if args.assets.is_none() {
        if args.out.is_some() {
            args.assets = Some(get_files_in_directory(args.out.as_ref().unwrap()).unwrap());
        } else {
            args.assets = Some(vec![
                PathBuf::from("./out/LINK_chainlink.csv"),
                PathBuf::from("./out/MNT_mantle.csv"),
                PathBuf::from("./out/ADA_cardano.csv"),
            ]);
        }
    }
    if args.out.is_none() {
        args.out = Some(PathBuf::from("./out"));
    }
    if args.ma_short.is_none() {
        args.ma_short = Some(5);
    }
    if args.ma_long.is_none() {
        args.ma_long = Some(15);
    }
    if args.min_signals.is_none() {
        args.min_signals = Some(2);
    }
    if args.short_alts.is_none() {
        args.short_alts = Some(false);
    }
    if args.btc_hedge.is_none() {
        args.btc_hedge = Some(0.3);
    }
    if args.stop_lookback.is_none() {
        args.stop_lookback = Some(14);
    }
    if args.atr_mult.is_none() {
        args.atr_mult = Some(3.0);
    }
    if args.vol_mult.is_none() {
        args.vol_mult = Some(2.5);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();
    match args.command {
        Some(Command::Ohlc(mut ohlc_args)) => {
            apply_ohlc_defaults(&mut ohlc_args);
            ohlc::execute(&ohlc_args).await?;
        }
        Some(Command::Strategy(mut strategy_args)) => {
            apply_strategy_defaults(&mut strategy_args);
            if strategy_args.assets.as_ref().unwrap().is_empty() {
                let out_dir = strategy_args.out.as_ref().unwrap();
                let files = get_files_in_directory(out_dir)?;
                strategy_args.assets = Some(files);
            }
            strategy::execute(&strategy_args)?;
        }
        None => {
            // Default behavior: run both OHLC and strategy with defaults
            println!("Running with default arguments...");
            println!("1. Fetching OHLC data...");
            let ohlc_args = get_default_ohlc_args();
            ohlc::execute(&ohlc_args).await?;

            println!("2. Running strategy backtest...");
            let strategy_args = get_default_strategy_args();
            strategy::execute(&strategy_args)?;
        }
    }
    Ok(())
}

fn get_files_in_directory(dir_path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().unwrap_or_default() == "csv" {
            files.push(path);
        }
    }

    Ok(files)
}
