use anyhow::Result;
use chrono::{Duration, Utc};
use std::fs;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

use crate::{OhlcArgs, StrategyArgs, analyzer, ohlc, strategy, trade};

/// Daemon mode for continuous signal generation and portfolio management
pub async fn execute(
    continuous: bool,
    portfolio_value: f64,
    risk_cap_percent: f64,
    check_interval: u64,
) -> Result<()> {
    println!("🚀 Starting Crypto Strategy Daemon");
    println!("Portfolio Value: ${:.0}", portfolio_value);
    println!("Risk Cap per Position: {:.1}%", risk_cap_percent);
    println!("Check Interval: {} minutes", check_interval);
    println!("Continuous Mode: {}", continuous);
    println!();

    let mut iteration = 0;

    loop {
        iteration += 1;
        let start_time = Utc::now();

        println!(
            "⏰ === DAEMON CYCLE #{} - {} ===",
            iteration,
            start_time.format("%Y-%m-%d %H:%M:%S UTC")
        );

        // Step 1: Fetch latest OHLC data
        println!("1. Fetching latest OHLC data...");
        let ohlc_result = fetch_latest_data().await;
        match ohlc_result {
            Ok(_) => println!("   ✅ OHLC data updated successfully"),
            Err(e) => {
                println!("   ❌ OHLC data fetch failed: {}", e);
                if !continuous {
                    return Err(e);
                }
                println!(
                    "   ⏭️  Skipping this cycle, will retry in {} minutes",
                    check_interval
                );
                sleep(StdDuration::from_secs(check_interval * 60)).await;
                continue;
            }
        }

        // Step 2: Generate strategy signals
        println!("2. Generating strategy signals...");
        let strategy_result = generate_signals().await;
        match strategy_result {
            Ok(_) => println!("   ✅ Strategy signals generated successfully"),
            Err(e) => {
                println!("   ❌ Strategy signal generation failed: {}", e);
                if !continuous {
                    return Err(e);
                }
                println!(
                    "   ⏭️  Skipping this cycle, will retry in {} minutes",
                    check_interval
                );
                sleep(StdDuration::from_secs(check_interval * 60)).await;
                continue;
            }
        }

        // Step 3: Analyze profitable strategies
        println!("3. Analyzing profitable strategies...");
        let analysis_result = analyze_strategies().await;
        match analysis_result {
            Ok(_) => println!("   ✅ Strategy analysis completed successfully"),
            Err(e) => {
                println!("   ❌ Strategy analysis failed: {}", e);
                if !continuous {
                    return Err(e);
                }
                println!(
                    "   ⏭️  Skipping this cycle, will retry in {} minutes",
                    check_interval
                );
                sleep(StdDuration::from_secs(check_interval * 60)).await;
                continue;
            }
        }

        // Step 4: Generate trading playbooks with real execution values
        println!("4. Generating trading playbooks...");
        let trade_result = generate_playbooks(portfolio_value, risk_cap_percent).await;
        match trade_result {
            Ok(_) => println!("   ✅ Trading playbooks generated successfully"),
            Err(e) => {
                println!("   ❌ Trading playbook generation failed: {}", e);
                if !continuous {
                    return Err(e);
                }
                println!(
                    "   ⏭️  Skipping this cycle, will retry in {} minutes",
                    check_interval
                );
                sleep(StdDuration::from_secs(check_interval * 60)).await;
                continue;
            }
        }

        // Step 5: Generate portfolio summary
        println!("5. Generating portfolio summary...");
        generate_portfolio_summary(portfolio_value, risk_cap_percent).await?;

        let end_time = Utc::now();
        let duration = end_time - start_time;
        println!(
            "   ✅ Cycle completed in {:.1} seconds",
            duration.num_seconds() as f64
        );

        if !continuous {
            println!("🎯 Single run completed successfully!");
            break;
        }

        // Wait for next cycle
        let next_run = start_time + Duration::minutes(check_interval as i64);
        println!(
            "⏰ Next run scheduled for: {}",
            next_run.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!();

        sleep(StdDuration::from_secs(check_interval * 60)).await;
    }

    Ok(())
}

async fn fetch_latest_data() -> Result<()> {
    // Use default OHLC args but with resume=true to only fetch missing data
    let mut ohlc_args = OhlcArgs {
        out: Some(std::path::PathBuf::from("./out")),
        top_n: Some(100),
        vs: Some("usd".to_string()),
        concurrency: Some(6),
        request_delay_ms: Some(250),
        write_manifest: Some(true),
        resume: Some(true), // Only fetch missing data
        ..Default::default()
    };

    // Set date range to last 30 days
    let end_date = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    let start_date = end_date - chrono::Duration::days(30);

    ohlc_args.start = Some(start_date.format("%Y-%m-%d").to_string());
    ohlc_args.end = Some(end_date.format("%Y-%m-%d").to_string());

    ohlc::execute(&ohlc_args).await
}

async fn generate_signals() -> Result<()> {
    // Use default strategy args
    let mut strategy_args = StrategyArgs {
        out: Some(std::path::PathBuf::from("./out/signals")),
        ma_short: Some(3),
        ma_long: Some(7),
        stop_lookback: Some(14),
        min_signals: Some(2),
        atr_mult: Some(3.0),
        vol_mult: Some(2.5),
        btc_hedge: Some(0.0),
        btc: Some(std::path::PathBuf::from("./out/BTC.csv")),
        ..Default::default()
    };

    // Get all CSV files in the out directory (excluding BTC)
    let out_dir = std::path::Path::new("./out");
    let mut asset_paths = Vec::new();

    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str()
                && file_name.ends_with(".csv") && !file_name.starts_with("BTC_")
            {
                asset_paths.push(entry.path());
            }
        }
    }

    strategy_args.assets = Some(asset_paths);

    strategy::execute(&strategy_args)
}

async fn analyze_strategies() -> Result<()> {
    analyzer::execute("./out/signals", None)
}

async fn generate_playbooks(portfolio_value: f64, risk_cap_percent: f64) -> Result<()> {
    // Generate playbooks with current execution values
    trade::execute("./out/signals", Some("./out/current_playbooks.json")).await?;

    // Also generate a portfolio-specific playbook
    generate_portfolio_playbook(portfolio_value, risk_cap_percent).await?;

    Ok(())
}

async fn generate_portfolio_playbook(portfolio_value: f64, risk_cap_percent: f64) -> Result<()> {
    println!("   📊 Generating portfolio-specific playbook...");

    // Load current playbooks
    let playbooks = trade::generate_top_10_playbooks("./out/signals").await?;

    // Filter for assets with active signals (all_signals = true)
    let active_playbooks: Vec<_> = playbooks
        .iter()
        .filter(|p| p.computed_values.all_signals)
        .collect();

    if active_playbooks.is_empty() {
        println!("   ⚠️  No assets with active signals found");
        return Ok(());
    }

    println!(
        "   🎯 Found {} assets with active signals:",
        active_playbooks.len()
    );

    let mut total_position_value = 0.0;
    let mut total_risk = 0.0;
    let mut portfolio_playbook = Vec::new();

    for (i, playbook) in active_playbooks.iter().enumerate() {
        let cv = &playbook.computed_values;

        // Calculate position size based on portfolio value
        let position_value =
            (portfolio_value * risk_cap_percent / 100.0) / (cv.risk_per_share / cv.current_price);
        let shares = (position_value / cv.current_price).floor() as u64;
        let actual_position_value = shares as f64 * cv.current_price;
        let actual_risk = shares as f64 * cv.risk_per_share;

        total_position_value += actual_position_value;
        total_risk += actual_risk;

        let entry = serde_json::json!({
            "rank": i + 1,
            "asset": playbook.asset,
            "current_price": cv.current_price,
            "ma30": cv.ma30,
            "ma7": cv.ma7,
            "rs_ma7": cv.rs_ma7,
            "rs_ma30": cv.rs_ma30,
            "atr_14": cv.atr_14,
            "signal_strength": cv.signal_strength,
            "position": {
                "shares": shares,
                "value": actual_position_value,
                "percent_of_portfolio": (actual_position_value / portfolio_value) * 100.0
            },
            "risk_management": {
                "stop_price": cv.stop_price,
                "risk_per_share": cv.risk_per_share,
                "total_risk": actual_risk,
                "risk_percent": (actual_risk / portfolio_value) * 100.0
            },
            "profit_taking": {
                "target_price": cv.profit_target,
                "target_percent": cv.profit_target_percent,
                "scale_out_shares": cv.scale_out_shares,
                "remaining_shares": cv.remaining_shares
            },
            "execution": {
                "is_extended": cv.is_extended,
                "ma30_pullback_price": cv.ma30_pullback_price,
                "extended_percent": cv.extended_percent
            }
        });

        portfolio_playbook.push(entry);

        println!(
            "   {}. {} - ${:.2} ({} shares, ${:.0} value, {:.1}% risk)",
            i + 1,
            playbook.asset,
            cv.current_price,
            shares,
            actual_position_value,
            (actual_risk / portfolio_value) * 100.0
        );
    }

    // Create portfolio summary
    let portfolio_summary = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "portfolio_value": portfolio_value,
        "risk_cap_percent": risk_cap_percent,
        "active_positions": active_playbooks.len(),
        "total_position_value": total_position_value,
        "total_risk": total_risk,
        "portfolio_utilization": (total_position_value / portfolio_value) * 100.0,
        "total_risk_percent": (total_risk / portfolio_value) * 100.0,
        "positions": portfolio_playbook
    });

    // Save portfolio playbook
    let json = serde_json::to_string_pretty(&portfolio_summary)?;
    fs::write("./out/portfolio_playbook.json", json)?;

    println!("   📈 Portfolio Summary:");
    println!(
        "      Total Position Value: ${:.0} ({:.1}% of portfolio)",
        total_position_value,
        (total_position_value / portfolio_value) * 100.0
    );
    println!(
        "      Total Risk: ${:.0} ({:.1}% of portfolio)",
        total_risk,
        (total_risk / portfolio_value) * 100.0
    );
    println!("      Active Positions: {}", active_playbooks.len());

    Ok(())
}

async fn generate_portfolio_summary(portfolio_value: f64, risk_cap_percent: f64) -> Result<()> {
    // Create a simple text summary for quick reference
    let summary = format!(
        "=== PORTFOLIO SUMMARY - {} ===\n\
        Portfolio Value: ${:.0}\n\
        Risk Cap per Position: {:.1}%\n\
        Generated: {}\n\
        \n\
        Run 'cargo run -- daemon --help' for options\n\
        Run 'cargo run -- trade' to see detailed playbooks\n\
        Run 'cargo run -- analyze' to see strategy analysis\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        portfolio_value,
        risk_cap_percent,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    fs::write("./out/portfolio_summary.txt", summary)?;
    println!("   📄 Portfolio summary saved to ./out/portfolio_summary.txt");

    Ok(())
}

/// Generate systemd service file for production deployment
pub fn generate_systemd_service(
    portfolio_value: f64,
    risk_cap_percent: f64,
    check_interval: u64,
) -> Result<()> {
    let service_content = format!(
        "[Unit]
Description=Crypto Strategy Daemon
After=network.target

[Service]
Type=simple
User=crypto-strategy
WorkingDirectory=/opt/crypto-strategy
ExecStart=/opt/crypto-strategy/target/release/crypto-strategy daemon --continuous --portfolio-value {:.0} --risk-cap-percent {:.1} --check-interval {}
Restart=always
RestartSec=10
Environment=RUST_LOG=info
Environment=COINGECKO_API_KEY=your_api_key_here

[Install]
WantedBy=multi-user.target",
        portfolio_value, risk_cap_percent, check_interval
    );

    fs::write("./crypto-strategy.service", service_content)?;
    println!("📄 Systemd service file generated: ./crypto-strategy.service");
    println!("To install:");
    println!("  sudo cp crypto-strategy.service /etc/systemd/system/");
    println!("  sudo systemctl daemon-reload");
    println!("  sudo systemctl enable crypto-strategy");
    println!("  sudo systemctl start crypto-strategy");

    Ok(())
}

/// Generate cron job for scheduled execution
pub fn generate_cron_job(check_interval: u64) -> Result<()> {
    let cron_expression = match check_interval {
        60 => "0 * * * *",          // Every hour
        30 => "0,30 * * * *",       // Every 30 minutes
        15 => "0,15,30,45 * * * *", // Every 15 minutes
        5 => "*/5 * * * *",         // Every 5 minutes
        _ => "0 * * * *",           // Default to hourly
    };

    let cron_job = format!(
        "# Crypto Strategy Daemon - Run every {} minutes
{} /opt/crypto-strategy/target/release/crypto-strategy daemon --portfolio-value 100000 --risk-cap-percent 1.0 >> /var/log/crypto-strategy.log 2>&1

# Optional: Clean old logs weekly
0 2 * * 0 find /var/log/crypto-strategy.log -mtime +7 -delete",
        check_interval, cron_expression
    );

    fs::write("./crypto-strategy.cron", cron_job)?;
    println!("📄 Cron job generated: ./crypto-strategy.cron");
    println!("To install:");
    println!("  sudo cp crypto-strategy.cron /etc/cron.d/crypto-strategy");
    println!("  sudo chmod 644 /etc/cron.d/crypto-strategy");

    Ok(())
}

/// Generate Docker Compose file for containerized deployment
pub fn generate_docker_compose(
    portfolio_value: f64,
    risk_cap_percent: f64,
    check_interval: u64,
) -> Result<()> {
    let compose_content = format!(
        "version: '3.8'

services:
  crypto-strategy:
    build: .
    container_name: crypto-strategy-daemon
    restart: unless-stopped
    environment:
      - RUST_LOG=info
      - COINGECKO_API_KEY=your_api_key_here
    volumes:
      - ./out:/app/out
      - ./logs:/app/logs
    command: daemon --continuous --portfolio-value {:.0} --risk-cap-percent {:.1} --check-interval {}
    healthcheck:
      test: [\"CMD\", \"cargo\", \"run\", \"--\", \"daemon\", \"--help\"]
      interval: 5m
      timeout: 10s
      retries: 3
      start_period: 30s

  # Optional: Add monitoring with Prometheus/Grafana
  prometheus:
    image: prom/prometheus:latest
    container_name: crypto-strategy-prometheus
    ports:
      - \"9090:9090\"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    container_name: crypto-strategy-grafana
    ports:
      - \"3000:3000\"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana
    restart: unless-stopped

volumes:
  grafana-storage:",
        portfolio_value, risk_cap_percent, check_interval
    );

    fs::write("./docker-compose.yml", compose_content)?;
    println!("🐳 Docker Compose file generated: ./docker-compose.yml");
    println!("To deploy:");
    println!("  docker-compose up -d");
    println!("  docker-compose logs -f crypto-strategy");

    Ok(())
}
