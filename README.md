# Crypto Momentum AI

A sophisticated momentum-based cryptocurrency trading system that combines quantitative analysis, AI-powered insights, and automated execution to maximize profit through advanced signal generation, risk management, and portfolio optimization.

## 🚀 Overview

This system leverages advanced technical analysis, relative strength metrics, and AI-powered machine learning to generate high-quality trading signals. It features:

- **Quantitative Risk Assessment**: Multi-factor analysis for dynamic position sizing
- **AI-Powered Insights**: OpenAI integration for contextual market analysis
- **Automated Execution**: Daemon system for continuous daily signal generation
- **Production Ready**: Complete deployment tools (systemd, cron, Docker)

## 🎯 Strategy Logic

### Core Signal Definition

#### Buy Entry Conditions

- **Trend**: Price > 30-day MA (medium-term bullish trend)
- **Momentum**: 7-day MA > 30-day MA (short-term momentum)
- **Relative Strength**: RS_MA7 > RS_MA30 (outperforming Bitcoin)

#### Signal Weighting

- **Full Weight (1.0)**: All 3 conditions met (trend + momentum + RS)
- **Half Weight (0.5)**: 2/3 conditions + RS bullish
- **Zero Weight**: Less than 2/3 conditions or RS bearish

### Advanced Risk Management

#### Quantitative Risk Assessment

The system uses a 10-factor risk model to determine optimal position sizing:

1. **Sharpe Ratio**: Risk-adjusted return quality
2. **Maximum Drawdown**: Historical risk exposure
3. **Win Rate**: Signal reliability consistency
4. **Volatility**: Price stability assessment
5. **Return Magnitude**: Performance scale factor
6. **Trading Days**: Data confidence level
7. **Profit Factor**: Risk-reward efficiency
8. **Relative Strength**: Market outperformance
9. **Price Extension**: Overbought/oversold levels
10. **ATR-based Risk**: Volatility-adjusted sizing

#### Dynamic Execution Modes

- **Signal-at-Close**: Standard execution for reliable assets
- **Pullback-to-MA30**: Advanced execution for high-confidence signals
- **Extended Thresholds**: Adaptive limits based on performance metrics

## 🤖 AI-Powered Analysis

### OpenAI Integration

The system includes sophisticated AI analysis powered by GPT-4o-mini with robust error handling:

- **Asset-Specific Insights**: Contextual trading notes and risk assessments
- **Portfolio Analysis**: Market condition analysis and positioning advice
- **Execution Recommendations**: AI-generated entry/exit strategies
- **Intelligent Fallback**: Seamless degradation to quantitative analysis when AI unavailable
- **Response Parsing**: Advanced JSON extraction with markdown code block support
- **Error Recovery**: Graceful handling of API failures and malformed responses

### Intelligent Risk Caps

- **Data-Driven**: Risk caps determined by performance metrics, not hardcoded rules
- **Adaptive**: Automatically adjusts based on market conditions and asset characteristics
- **Conservative**: Bounded between 0.2% and 2.5% with geometric mean weighting

## 📊 Performance Analysis

### Strategy Analyzer

Comprehensive analysis of trading performance:

- **Total Return**: Absolute performance metrics
- **Sharpe Ratio**: Risk-adjusted returns
- **Maximum Drawdown**: Worst-case scenario analysis
- **Win Rate**: Signal success percentage
- **Profit Factor**: Risk-reward efficiency
- **Trading Days**: Data quality assessment

### Top 10 Playbooks

Automated generation of executable trading plans:

- **Entry Rules**: Precise signal conditions and execution modes
- **Exit Rules**: Profit-taking and stop-loss strategies
- **Position Sizing**: Risk-based allocation with real-time calculations
- **Conviction Levels**: Confidence scoring based on historical performance
- **Real-Time Values**: Current prices, MAs, stops, and position sizes

## 🛠️ Usage

### Quick Start

The simplest way to run the complete workflow:

```bash
# Run complete workflow: OHLC + Strategy + Analysis + AI Insights + Trade Playbooks
cargo run --release
# or
make run
```

This automatically:

1. **Fetches OHLC data** for top 100 cryptocurrencies (30 days)
2. **Runs strategy backtest** with optimized parameters
3. **Analyzes profitable strategies** with detailed metrics
4. **Generates AI-powered insights** (with intelligent fallback if API unavailable)
5. **Creates top-10 trading playbooks** with executable rules and AI analysis

### Advanced Usage

#### Individual Commands

```bash
# Data collection only
cargo run -- ohlc --top-n 100 --vs usd

# Strategy backtest only
cargo run -- strategy --btc ./out/BTC.csv --assets ./out/*.csv

# Analysis only
cargo run -- analyze --signals-dir ./out/signals

# AI-powered trade generation
cargo run -- trade --signals-dir ./out/signals --output-json ./playbooks.json

# Daemon mode (continuous execution)
cargo run -- daemon --continuous --portfolio-value 100000
```

#### Environment Setup

```bash
# Required: CoinGecko Pro API key
export CG_PRO_API_KEY="your-api-key"

# Optional: OpenAI API key for AI insights (with intelligent fallback)
export OPENAI_API_KEY="your-openai-key"

# Note: If OPENAI_API_KEY is not set, the system will automatically
# use quantitative analysis fallback for all AI-powered features
```

### Makefile Commands

```bash
# Development
make build          # Build the project
make test           # Run tests
make lint           # Run clippy lints
make clean          # Clean build artifacts

# Execution
make run            # Complete workflow
make ohlc           # Data collection only
make strategy       # Strategy backtest only
make analyze        # Analysis only
make trade          # Trade playbooks only

# AI Features
make trade-ai       # AI-powered trade generation

# Daemon
make daemon         # Start daemon mode
make daemon-once    # Run daemon once

# Deployment
make deploy-systemd # Generate systemd service
make deploy-cron    # Generate cron job
make deploy-docker  # Generate Docker setup
```

## 🔄 Automated Execution

### Daemon System

The daemon provides continuous, automated execution:

```bash
# Start daemon (runs continuously)
cargo run -- daemon --continuous --portfolio-value 100000 --risk-cap-percent 1.0

# Run once (cron-friendly)
cargo run -- daemon --portfolio-value 100000 --risk-cap-percent 1.0
```

### Production Deployment

#### Systemd Service

```bash
# Generate and install systemd service
make deploy-systemd PORTFOLIO_VALUE=100000 RISK_CAP=1.0
sudo systemctl enable crypto-strategy
sudo systemctl start crypto-strategy
```

#### Cron Job

```bash
# Generate cron job
make deploy-cron PORTFOLIO_VALUE=100000 RISK_CAP=1.0

# Add to crontab
crontab -e
# Add: 0 6 * * * /path/to/crypto-strategy/run_daily.sh
```

#### Docker Deployment

```bash
# Generate Docker setup
make deploy-docker

# Run with Docker Compose
docker-compose up -d
```

## 📁 Output Structure

### Directory Layout

```
./out/                    # OHLC data directory
├── BTC.csv              # Bitcoin baseline
├── ETH_ethereum.csv     # Individual coin data
├── LINK_chainlink.csv   # (etc...)
└── manifest.json        # Metadata

./out/signals/           # Strategy output directory
├── signals_ETH.csv      # Daily signals per asset
├── signals_LINK.csv
├── equity_curve.csv     # Portfolio equity curve
└── metrics.txt          # Performance summary
```

### Trade Playbooks

JSON output with executable trading plans and AI insights:

```json
{
  "asset": "ETH_ethereum",
  "entry_rules": {
    "primary": "Go long EOD when 3/3 signals...",
    "alternative": "Staggered entry: 50% at signal close...",
    "signal_conditions": {
      "trend": "close > MA30",
      "momentum": "MA7 > MA30",
      "rs": "RS_MA7 > RS_MA30"
    }
  },
  "computed_values": {
    "current_price": 2456.78,
    "ma30": 2380.45,
    "ma7": 2420.12,
    "stop_price": 2310.25,
    "recommended_shares": 40.5,
    "position_value": 99500.0
  },
  "ai_insights": {
    "trading_notes": [
      "Monitor price action closely as the current price aligns with both MA30 and MA7",
      "Given the high win rate, consider scaling up position sizes gradually"
    ],
    "risk_assessment": "The strategy has a relatively low risk level with a max drawdown of 1.17%",
    "execution_recommendations": [
      "Enter positions at the current price with a stop-loss set just below the recent support level",
      "Consider taking profits at price targets based on historical resistance levels"
    ],
    "market_context": "Current market conditions show a stable price with minor fluctuations"
  }
}
```

## 🔧 Configuration

### Strategy Parameters

| Parameter       | Default | Description                      |
| --------------- | ------- | -------------------------------- |
| `ma_short`      | 3       | Short-term moving average period |
| `ma_long`       | 7       | Long-term moving average period  |
| `stop_lookback` | 14      | ATR calculation period           |
| `min_signals`   | 2       | Minimum signals for trade        |
| `atr_mult`      | 3.0     | ATR multiplier for stops         |
| `vol_mult`      | 2.5     | Volatility multiplier for stops  |
| `btc_hedge`     | 0.0     | BTC hedge ratio (0.0-1.0)        |

### Risk Management

| Parameter          | Range    | Description                      |
| ------------------ | -------- | -------------------------------- |
| `risk_cap_percent` | 0.2-2.5% | Maximum risk per position        |
| `portfolio_value`  | Any      | Total portfolio value for sizing |
| `concurrency`      | 1-10     | Parallel request limit           |
| `request_delay_ms` | 100-1000 | API rate limiting                |

## 🧪 Testing & Validation

### Backtesting

- **Historical Data**: 30-day rolling window analysis
- **Signal Quality**: Win rate and profit factor metrics
- **Risk Assessment**: Drawdown and volatility analysis
- **Performance**: Sharpe ratio and CAGR calculations

### Validation

- **Cross-Validation**: Multiple time periods and market conditions
- **Stress Testing**: Extreme market scenarios
- **Monte Carlo**: Statistical significance testing
- **Live Testing**: Paper trading validation

## 📈 Performance Metrics

### Key Indicators

- **Total Return**: Absolute performance
- **Sharpe Ratio**: Risk-adjusted returns
- **Maximum Drawdown**: Worst-case loss
- **Win Rate**: Signal success percentage
- **Profit Factor**: Risk-reward efficiency
- **CAGR**: Compound annual growth rate

### Risk Metrics

- **VaR**: Value at Risk calculations
- **CVaR**: Conditional Value at Risk
- **Volatility**: Price stability measures
- **Correlation**: Market dependency analysis

## 🚀 Advanced Features

### Recent Improvements

#### AI Integration Enhancements (Latest)

- **Robust JSON Parsing**: Advanced response parsing with markdown code block support
- **Error Recovery**: Comprehensive error handling for API failures and malformed responses
- **Intelligent Fallback**: Seamless degradation to quantitative analysis when AI unavailable
- **Timeout Management**: 30-second request timeouts to prevent hanging
- **Response Validation**: Structured validation of OpenAI API responses
- **Cost Optimization**: Efficient API usage with proper error handling

### AI Integration

- **OpenAI GPT-4o-mini**: Contextual market analysis with structured JSON responses
- **Intelligent Fallback**: Seamless degradation to quantitative analysis when AI unavailable
- **Response Parsing**: Advanced JSON extraction supporting markdown code blocks
- **Error Recovery**: Comprehensive error handling for API failures and malformed responses
- **Timeout Management**: 30-second request timeouts to prevent hanging
- **Cost Optimization**: Efficient API usage with proper error handling

### Quantitative Analysis

- **Multi-Factor Models**: 10-factor risk assessment
- **Geometric Mean**: Balanced risk weighting
- **Confidence Scoring**: Dynamic execution mode selection
- **Adaptive Thresholds**: Market-condition responsive limits

### Production Features

- **File Locking**: Concurrent execution prevention
- **Atomic Writes**: Data integrity protection
- **Resume Mode**: Incremental data collection
- **Monitoring**: Comprehensive logging and metrics

## 🔒 Security & Reliability

### Data Protection

- **API Key Security**: Environment variable storage
- **File Locking**: Process synchronization
- **Atomic Operations**: Transactional file updates
- **Error Recovery**: Graceful failure handling

### Monitoring

- **Comprehensive Logging**: Detailed execution logs
- **Performance Metrics**: Real-time monitoring
- **Error Tracking**: Failure analysis and recovery
- **Health Checks**: System status validation

## 📚 Technical Architecture

### Core Components

- **OHLC Exporter**: CoinGecko Pro API integration with rate limiting
- **Strategy Engine**: Signal generation and backtesting with advanced risk models
- **Analyzer**: Performance metrics and analysis with comprehensive reporting
- **Trade Generator**: Executable playbook creation with AI insights
- **AI Insights**: OpenAI integration with robust error handling and fallback
- **Daemon**: Automated execution system with continuous monitoring

### Technology Stack

- **Rust**: High-performance systems programming
- **Tokio**: Async runtime for concurrent operations
- **Serde**: Serialization for data handling with JSON parsing
- **Clap**: Command-line argument parsing
- **Reqwest**: HTTP client for API calls with timeout management
- **Chrono**: Date and time handling
- **Anyhow**: Error handling and propagation
- **OpenAI API**: GPT-4o-mini integration for market analysis

## 🤝 Contributing

### Development Setup

```bash
git clone <repository>
cd crypto-strategy
cargo build
cargo test
cargo clippy -- -D warnings
```

### Code Quality

- **Clippy**: All warnings must be resolved
- **Tests**: Comprehensive test coverage
- **Documentation**: Clear code comments
- **Performance**: Optimized for production use

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## ⚠️ Disclaimer

This software is for educational and research purposes only. Cryptocurrency trading involves substantial risk of loss and is not suitable for all investors. Past performance does not guarantee future results. Always conduct your own research and consider consulting with a financial advisor before making investment decisions.

## 🔧 Troubleshooting

### Common Issues

#### OpenAI API Issues

```bash
# If you see "Failed to parse OpenAI response" warnings:
# 1. Check your API key is set correctly
echo $OPENAI_API_KEY

# 2. Verify API key format (should start with sk-)
# 3. Check API quota and billing status
# 4. The system will automatically fall back to quantitative analysis
```

#### Data Collection Issues

```bash
# If OHLC data collection fails:
# 1. Verify CoinGecko API key
export CG_PRO_API_KEY="your-key"

# 2. Check rate limits (max 100 requests/minute)
# 3. Verify internet connectivity
# 4. Check API quota status
```

#### Build Issues

```bash
# Clean and rebuild
make clean
cargo build --release

# Check Rust version (requires 1.70+)
rustc --version
```

### Performance Optimization

- **API Rate Limiting**: Built-in delays prevent rate limit violations
- **Concurrent Requests**: Configurable concurrency for optimal performance
- **Caching**: Intelligent caching reduces redundant API calls
- **Error Recovery**: Automatic retry logic for transient failures

## 🆘 Support

For questions, issues, or contributions:

- **Issues**: GitHub Issues for bug reports
- **Discussions**: GitHub Discussions for questions
- **Documentation**: Comprehensive inline documentation
- **Examples**: Extensive usage examples in README

---

**Crypto Momentum AI - Built with ❤️ in Rust for the crypto trading community**
