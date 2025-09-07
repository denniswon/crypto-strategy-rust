# Crypto Strategy Project Makefile
# A Rust-based cryptocurrency trading strategy backtesting tool

.PHONY: help build run test clean fmt clippy check release install uninstall docs deps ohlc strategy default

# Default target
help:
	@echo "Crypto Strategy Project - Available Commands:"
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
	@echo "  default    - Run complete workflow (OHLC + Strategy)"
	@echo ""
	@echo "Utilities:"
	@echo "  clean      - Clean build artifacts"
	@echo "  deps       - Show dependency tree"
	@echo "  docs       - Generate and open documentation"
	@echo "  env        - Show environment setup"
	@echo ""

# Development commands
build:
	@echo "Building crypto-strategy in debug mode..."
	cargo build

run: build
	@echo "Running crypto-strategy with default arguments..."
	cargo run

test:
	@echo "Running tests..."
	cargo test

check:
	@echo "Running cargo check..."
	cargo check

fmt:
	@echo "Formatting code..."
	cargo fmt

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
	@echo "Release binary built at: target/release/crypto-strategy"

install: release
	@echo "Installing binary to ~/.cargo/bin..."
	cp target/release/crypto-strategy ~/.cargo/bin/
	@echo "Installed! You can now run 'crypto-strategy' from anywhere."

uninstall:
	@echo "Removing binary from ~/.cargo/bin..."
	rm -f ~/.cargo/bin/crypto-strategy
	@echo "Uninstalled!"

# Data and strategy commands
ohlc:
	@echo "Fetching OHLC data..."
	cargo run --release -- ohlc

strategy:
	@echo "Running strategy backtest..."
	cargo run --release -- strategy

default:
	@echo "Running complete workflow (OHLC + Strategy)..."
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
	docker build -t crypto-strategy .

docker-run:
	@echo "Running in Docker container..."
	docker run --rm -v $(PWD)/out:/app/out crypto-strategy

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

# Quick development workflow
dev: fmt clippy test build
	@echo "Development cycle completed!"

dev-strict: fmt clippy-strict test build
	@echo "Strict development cycle completed!"

# Production deployment
deploy: clean release
	@echo "Production deployment ready!"
	@echo "Binary location: target/release/crypto-strategy"
	@echo "Size: $(shell du -h target/release/crypto-strategy | cut -f1)"
