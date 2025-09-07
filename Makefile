# Crypto Momentum AI Project Makefile
# A Rust-based AI-powered cryptocurrency momentum trading system

.PHONY: help build run test clean fmt clippy check release install uninstall docs deps ohlc strategy default

# Default target
help:
	@echo "Crypto Momentum AI Project - Available Commands:"
	@echo ""
	@echo "Development:"
	@echo "  build      - Build the project in debug mode"
	@echo "  run        - Run the project with default arguments"
	@echo "  test       - Run all tests"
	@echo "  check      - Run cargo check (fast compilation check)"
	@echo "  fmt        - Format code with rustfmt"
	@echo "  clippy     - Run clippy linter"
	@echo ""
	@echo "Release:"
	@echo "  release    - Build optimized release binary"
	@echo "  install    - Install binary to ~/.cargo/bin"
	@echo "  uninstall  - Remove binary from ~/.cargo/bin"
	@echo ""
	@echo "Data & Strategy:"
	@echo "  ohlc       - Fetch OHLC data only"
	@echo "  strategy   - Run strategy backtest only"
	@echo "  analyze    - Analyze profitable trading strategies"
	@echo "  trade      - Generate top-10 trading playbooks"
	@echo "  default    - Run complete workflow (OHLC + Strategy + Analysis + Trade)"
	@echo ""
	@echo "Utilities:"
	@echo "  clean      - Clean build artifacts"
	@echo "  deps       - Show dependency tree"
	@echo "  docs       - Generate and open documentation"
	@echo "  env        - Show environment setup"
	@echo ""

# Development commands
build:
	@echo "Building crypto-momentum-ai in debug mode..."
	cargo build

run: build
	@echo "Running crypto-momentum-ai with default arguments..."
	cargo run

test:
	@echo "Running tests..."
	cargo test

check:
	@echo "Running cargo check..."
	cargo check

fmt:
	@echo "Formatting code..."
	cargo fmt -all

clippy:
	@echo "Running clippy linter..."
	cargo clippy

clippy-strict:
	@echo "Running strict clippy linter..."
	cargo clippy -- -D warnings

clippy-fix:
	@echo "Auto-fixing clippy issues..."
	cargo clippy --fix --allow-dirty --allow-staged

# Release commands
release:
	@echo "Building optimized release binary..."
	cargo build --release
	@echo "Release binary built at: target/release/crypto-momentum-ai"

install: release
	@echo "Installing binary to ~/.cargo/bin..."
	cp target/release/crypto-momentum-ai ~/.cargo/bin/
	@echo "Installed! You can now run 'crypto-momentum-ai' from anywhere."

uninstall:
	@echo "Removing binary from ~/.cargo/bin..."
	rm -f ~/.cargo/bin/crypto-momentum-ai
	@echo "Uninstalled!"

# Data and strategy commands
ohlc:
	@echo "Fetching OHLC data..."
	cargo run --release -- ohlc

strategy:
	@echo "Running strategy backtest..."
	cargo run --release -- strategy

analyze:
	@echo "Analyzing trading strategies..."
	cargo run --release -- analyze

analyze-detailed:
	@echo "Analyzing trading strategies with detailed output..."
	cargo run --release -- analyze --detailed $(ASSET)

trade:
	@echo "Generating top-10 trading playbooks..."
	cargo run --release -- trade

trade-json:
	@echo "Generating trading playbooks and saving to JSON..."
	cargo run --release -- trade --output-json ./out/playbooks.json

daemon:
	@echo "Starting daemon mode for continuous signal generation..."
	cargo run --release -- daemon --continuous --portfolio-value $(PORTFOLIO) --risk-cap-percent $(RISK) --check-interval $(INTERVAL)

daemon-once:
	@echo "Running single daemon cycle..."
	cargo run --release -- daemon --portfolio-value $(PORTFOLIO) --risk-cap-percent $(RISK)

deploy-systemd:
	@echo "Generating systemd service file..."
	cargo run --release -- deploy-systemd --portfolio-value $(PORTFOLIO) --risk-cap-percent $(RISK) --check-interval $(INTERVAL)

deploy-cron:
	@echo "Generating cron job..."
	cargo run --release -- deploy-cron --check-interval $(INTERVAL)

deploy-docker:
	@echo "Generating Docker Compose file..."
	cargo run --release -- deploy-docker --portfolio-value $(PORTFOLIO) --risk-cap-percent $(RISK) --check-interval $(INTERVAL)

default:
	@echo "Running complete workflow (OHLC + Strategy + Analysis + Trade)..."
	cargo run --release

# Utility commands
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Cleaned!"

deps:
	@echo "Dependency tree:"
	cargo tree

docs:
	@echo "Generating documentation..."
	cargo doc --open

env:
	@echo "Environment setup:"
	@echo "Rust version: $(shell rustc --version)"
	@echo "Cargo version: $(shell cargo --version)"
	@echo "Project directory: $(shell pwd)"
	@echo "Target directory: $(shell pwd)/target"
	@echo ""
	@echo "Required environment variables:"
	@echo "  CG_PRO_API_KEY - CoinGecko Pro API key (optional, can use --api-key)"
	@echo ""
	@echo "Default output directory: ./out"
	@echo "Default data period: Last 30 days (up to yesterday)"

# Advanced development commands
dev-setup:
	@echo "Setting up development environment..."
	rustup component add rustfmt clippy
	@echo "Development tools installed!"

bench:
	@echo "Running benchmarks..."
	cargo bench

audit:
	@echo "Running security audit..."
	cargo audit

update:
	@echo "Updating dependencies..."
	cargo update

# Data management commands
data-clean:
	@echo "Cleaning output data..."
	rm -rf ./out/*.csv ./out/*.json ./out/*.txt
	@echo "Data cleaned!"

data-backup:
	@echo "Creating data backup..."
	mkdir -p ./backups
	tar -czf ./backups/data-$(shell date +%Y%m%d-%H%M%S).tar.gz ./out/
	@echo "Backup created in ./backups/"

# Configuration commands
config-example:
	@echo "Creating example .env file..."
	@echo "# CoinGecko Pro API Key" > .env.example
	@echo "CG_PRO_API_KEY=your_api_key_here" >> .env.example
	@echo "Example .env file created!"

# Docker commands (if needed)
docker-build:
	@echo "Building Docker image..."
	docker build -t crypto-momentum-ai .

docker-run:
	@echo "Running in Docker container..."
	docker run --rm -v $(PWD)/out:/app/out crypto-momentum-ai

# CI/CD helpers
ci-check: fmt clippy test
	@echo "CI checks passed!"

ci-build: clean release
	@echo "CI build completed!"

# Help for specific commands
help-ohlc:
	@echo "OHLC Data Collection Commands:"
	@echo "  make ohlc                    - Fetch data with defaults"
	@echo "  cargo run -- ohlc --help     - Show all OHLC options"
	@echo "  cargo run -- ohlc --top-n 50 - Fetch top 50 coins"
	@echo "  cargo run -- ohlc --vs eur   - Use EUR as base currency"

help-strategy:
	@echo "Strategy Backtesting Commands:"
	@echo "  make strategy                    - Run strategy with defaults"
	@echo "  cargo run -- strategy --help    - Show all strategy options"
	@echo "  cargo run -- strategy --ma-short 10 --ma-long 20 - Custom parameters"

help-analyze:
	@echo "Strategy Analysis Commands:"
	@echo "  make analyze                     - Analyze all strategies"
	@echo "  make analyze-detailed ASSET=BTC  - Detailed analysis for specific asset"
	@echo "  cargo run -- analyze --help     - Show all analysis options"

help-trade:
	@echo "Trading Playbook Commands:"
	@echo "  make trade                       - Generate top-10 trading playbooks"
	@echo "  make trade-json                  - Generate playbooks and save to JSON"
	@echo "  cargo run -- trade --help       - Show all trade options"

help-daemon:
	@echo "Daemon Commands:"
	@echo "  make daemon PORTFOLIO=100000 RISK=1.0 INTERVAL=60 - Start continuous daemon"
	@echo "  make daemon-once PORTFOLIO=100000 RISK=1.0        - Run single daemon cycle"
	@echo "  cargo run -- daemon --help                        - Show all daemon options"

# Quick development workflow
dev: fmt clippy test build
	@echo "Development cycle completed!"

dev-strict: fmt clippy-strict test build
	@echo "Strict development cycle completed!"

# Production deployment
deploy: clean release
	@echo "Production deployment ready!"
	@echo "Binary location: target/release/crypto-momentum-ai"
	@echo "Size: $(shell du -h target/release/crypto-momentum-ai | cut -f1)"
