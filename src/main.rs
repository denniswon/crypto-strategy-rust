use std::path::PathBuf;

use anyhow::Result;
use crypto_strategy::{OhlcArgs, StrategyArgs, analyzer, daemon, ohlc, strategy, trade};

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
    Analyze {
        /// Signals directory to analyze
        #[arg(long, default_value = "./out/signals")]
        signals_dir: String,
        /// Asset to show detailed analysis for
        #[arg(long)]
        detailed: Option<String>,
    },
    Trade {
        /// Signals directory to generate playbooks from
        #[arg(long, default_value = "./out/signals")]
        signals_dir: String,
        /// Output JSON file for playbooks
        #[arg(long)]
        output_json: Option<String>,
    },
    Daemon {
        /// Run continuously and generate signals daily
        #[arg(long, default_value = "false")]
        continuous: bool,
        /// Portfolio value for position sizing
        #[arg(long, default_value = "100000")]
        portfolio_value: f64,
        /// Risk cap per position (% of portfolio)
        #[arg(long, default_value = "1.0")]
        risk_cap_percent: f64,
        /// Check interval in minutes (default: 60)
        #[arg(long, default_value = "60")]
        check_interval: u64,
    },
    DeploySystemd {
        /// Portfolio value for position sizing
        #[arg(long, default_value = "100000")]
        portfolio_value: f64,
        /// Risk cap per position (% of portfolio)
        #[arg(long, default_value = "1.0")]
        risk_cap_percent: f64,
        /// Check interval in minutes (default: 60)
        #[arg(long, default_value = "60")]
        check_interval: u64,
    },
    DeployCron {
        /// Check interval in minutes (default: 60)
        #[arg(long, default_value = "60")]
        check_interval: u64,
    },
    DeployDocker {
        /// Portfolio value for position sizing
        #[arg(long, default_value = "100000")]
        portfolio_value: f64,
        /// Risk cap per position (% of portfolio)
        #[arg(long, default_value = "1.0")]
        risk_cap_percent: f64,
        /// Check interval in minutes (default: 60)
        #[arg(long, default_value = "60")]
        check_interval: u64,
    },
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

fn apply_strategy_defaults(args: &mut StrategyArgs) {
    if args.btc.is_none() {
        args.btc = Some(PathBuf::from("./out/BTC.csv"));
    }
    if args.assets.is_none() {
        args.assets = Some(get_files_in_directory(&PathBuf::from("./out")).unwrap());
    }
    if args.out.is_none() {
        args.out = Some(PathBuf::from("./out/signals"));
    }
    if args.ma_short.is_none() {
        args.ma_short = Some(3);
    }
    if args.ma_long.is_none() {
        args.ma_long = Some(7);
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
        Some(Command::Analyze {
            signals_dir,
            detailed,
        }) => {
            analyzer::execute(&signals_dir, detailed.as_deref())?;
        }
        Some(Command::Trade {
            signals_dir,
            output_json,
        }) => {
            trade::execute(&signals_dir, output_json.as_deref()).await?;
        }
        Some(Command::Daemon {
            continuous,
            portfolio_value,
            risk_cap_percent,
            check_interval,
        }) => {
            daemon::execute(
                continuous,
                portfolio_value,
                risk_cap_percent,
                check_interval,
            )
            .await?;
        }
        Some(Command::DeploySystemd {
            portfolio_value,
            risk_cap_percent,
            check_interval,
        }) => {
            daemon::generate_systemd_service(portfolio_value, risk_cap_percent, check_interval)?;
        }
        Some(Command::DeployCron { check_interval }) => {
            daemon::generate_cron_job(check_interval)?;
        }
        Some(Command::DeployDocker {
            portfolio_value,
            risk_cap_percent,
            check_interval,
        }) => {
            daemon::generate_docker_compose(portfolio_value, risk_cap_percent, check_interval)?;
        }
        None => {
            // Default behavior: run OHLC, strategy, and analyze with defaults
            println!("Running with default arguments...");
            println!("1. Fetching OHLC data...");
            let mut ohlc_args = OhlcArgs::default();
            apply_ohlc_defaults(&mut ohlc_args);
            ohlc::execute(&ohlc_args).await?;

            println!("2. Running strategy backtest...");
            let mut strategy_args = StrategyArgs::default();
            apply_strategy_defaults(&mut strategy_args);
            strategy::execute(&strategy_args)?;

            println!("3. Analyzing profitable strategies...");
            analyzer::execute("./out/signals", None)?;

            println!("4. Generating top-10 trading playbooks...");
            trade::execute("./out/signals", Some("./out/playbooks.json")).await?;
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
            // Filter out strategy output files
            let filename = path.file_name().unwrap().to_string_lossy();
            if !filename.starts_with("signals_")
                && filename != "equity_curve.csv"
                && filename != "metrics.txt"
            {
                files.push(path);
            }
        }
    }

    Ok(files)
}
