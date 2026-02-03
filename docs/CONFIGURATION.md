# Configuration Guide

Complete guide to configuring Neleus for backtesting, paper trading, and live trading.

## Table of Contents

- [Project Configuration](#project-configuration)
- [Environment Variables](#environment-variables)
- [Backtest Configuration](#backtest-configuration)
- [Risk Configuration](#risk-configuration)
- [Venue Configuration](#venue-configuration)
- [Logging Configuration](#logging-configuration)

---

## Project Configuration

### neleus.toml

The main project configuration file.

```toml
[project]
name = "my_trading_bot"
version = "0.1.0"
description = "A Neleus trading project"
created = "2024-01-15T10:30:00"

[trading]
default_venue = "hyperliquid"     # hyperliquid, lighter, polymarket
network = "testnet"               # testnet or mainnet
default_timeframe = "1h"          # 1m, 5m, 15m, 1h, 4h, 1d
default_symbol = "ETH-PERP"

[backtest]
initial_capital = 100000.0
commission_bps = 5.0              # Basis points (5 = 0.05%)
slippage_bps = 2.0                # Slippage in basis points
fill_model = "immediate"          # immediate, next_tick, probabilistic
latency_ms = 0                    # Simulated latency in milliseconds

[risk]
max_position_pct = 10.0           # Max 10% per position
max_daily_loss_pct = 5.0          # Kill switch at 5% daily loss
max_drawdown_pct = 20.0           # Stop trading at 20% drawdown
max_leverage = 5.0
dynamic_limits = true             # Adjust limits based on volatility
position_limit_per_venue = 50.0   # Max position per venue

[portfolio]
enable_portfolio_mode = false     # Run multiple strategies as portfolio
correlation_tracking = true
rebalance_interval_hours = 24

[ui]
port = 8765
host = "127.0.0.1"
auto_open = true
theme = "dark"                    # dark or light
enable_live_charts = true

[logging]
level = "info"                    # debug, info, warning, error
file = "logs/neleus.log"
max_file_size_mb = 100
backup_count = 5
console_output = true
```

---

## Environment Variables

### .env File

**NEVER commit this file to version control!**

```bash
# =============================================================================
# Hyperliquid Configuration
# =============================================================================
HYPERLIQUID_PRIVATE_KEY=0x1234567890abcdef...
HYPERLIQUID_NETWORK=testnet           # testnet or mainnet
HYPERLIQUID_WALLET=0xYourWalletAddress
HYPERLIQUID_VAULT_ADDRESS=            # Optional for vault trading
HYPERLIQUID_API_URL=                  # Optional custom API endpoint

# =============================================================================
# Lighter (zkLighter) Configuration
# =============================================================================
LIGHTER_API_KEY=your_api_key_here
LIGHTER_PRIVATE_KEY=0x1234567890abcdef...
LIGHTER_NETWORK=testnet               # testnet or mainnet
LIGHTER_ACCOUNT_TIER=standard         # standard, pro, or vip

# =============================================================================
# Polymarket Configuration
# =============================================================================
POLYMARKET_API_KEY=your_api_key_here
POLYMARKET_API_SECRET=your_secret_here
POLYMARKET_NETWORK=mainnet

# =============================================================================
# Database (Optional - for event storage)
# =============================================================================
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_USER=neleus
POSTGRES_PASSWORD=your_secure_password
POSTGRES_DATABASE=neleus_trading

# TimescaleDB
TIMESCALE_HOST=localhost
TIMESCALE_PORT=5432
TIMESCALE_USER=neleus
TIMESCALE_PASSWORD=your_secure_password
TIMESCALE_DATABASE=neleus_timeseries

# =============================================================================
# Monitoring & Alerts (Optional)
# =============================================================================
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_CHAT_ID=your_chat_id
EMAIL_SMTP_SERVER=smtp.gmail.com
EMAIL_SMTP_PORT=587
EMAIL_FROM=alerts@yourdomain.com
EMAIL_TO=you@yourdomain.com
EMAIL_PASSWORD=your_app_password

# =============================================================================
# UI Configuration
# =============================================================================
NELEUS_UI_PORT=8765
NELEUS_UI_HOST=127.0.0.1

# =============================================================================
# General Settings
# =============================================================================
NELEUS_LOG_LEVEL=info
NELEUS_ENV=development                # development, staging, production
```

### Loading Environment Variables

```python
from neleus.config import load_env

# Load .env file
load_env()

# Or load specific file
load_env(".env.production")
```

---

## Backtest Configuration

### BacktestConfig

Configure backtest parameters programmatically.

```python
from neleus import BacktestConfig, FillModel, LatencyModel

config = BacktestConfig(
    # Capital
    initial_capital=100000.0,
    
    # Costs
    commission_bps=5.0,          # 5 basis points (0.05%)
    slippage_bps=2.0,            # 2 basis points (0.02%)
    
    # Date range
    start_date="2024-01-01",     # YYYY-MM-DD
    end_date="2024-06-01",
    
    # Simulation models
    fill_model=FillModel.NextTick,
    latency_model=LatencyModel.Fixed,
    latency_ms=10,               # 10ms fixed latency
    
    # Data
    bar_interval="1h",
    
    # Advanced
    enable_short_selling=True,
    margin_requirement=0.05,     # 5% margin (20x leverage)
)
```

### Fill Models

How orders are filled in backtesting.

```python
from neleus import FillModel

# Immediate fill at current price (optimistic)
config.fill_model = FillModel.Immediate

# Fill at next bar open (more realistic)
config.fill_model = FillModel.NextTick

# Probabilistic fill based on volume
config.fill_model = FillModel.Probabilistic

# Simulate order book for realistic fills
config.fill_model = FillModel.OrderBook
```

### Slippage Models

How slippage is calculated.

```python
from neleus import SlippageModel

# Fixed basis points
config.slippage_model = SlippageModel.FixedBps
config.slippage_bps = 2.0

# Volume-based (larger orders = more slippage)
config.slippage_model = SlippageModel.VolumeBased
config.volume_impact_factor = 0.1

# Spread-based (use actual bid-ask spread)
config.slippage_model = SlippageModel.SpreadBased
```

### Latency Models

Simulate order latency.

```python
from neleus import LatencyModel

# No latency (instant execution)
config.latency_model = LatencyModel.Zero

# Fixed latency with optional jitter
config.latency_model = LatencyModel.Fixed
config.latency_ms = 50
config.latency_jitter_ms = 10

# Realistic network latency distribution
config.latency_model = LatencyModel.LogNormal
config.latency_mean_ms = 50
config.latency_std_ms = 20
```

---

## Risk Configuration

### RiskConfig

Configure risk management rules.

```python
from neleus import RiskConfig

risk_config = RiskConfig(
    # Position limits
    max_position_pct=10.0,              # Max 10% per position
    max_open_positions=10,              # Max 10 open positions
    concentration_limit_pct=25.0,       # Max 25% in single asset
    
    # Leverage
    max_leverage=5.0,
    margin_requirement=0.20,            # 20% margin
    
    # Loss limits
    max_daily_loss_pct=5.0,             # Kill switch
    max_drawdown_pct=20.0,
    max_unrealized_loss_pct=10.0,
    
    # Dynamic risk
    dynamic_limits=True,
    volatility_lookback_days=30,
    scale_down_on_losses=True,
    
    # Per-venue limits
    position_limits={
        "hyperliquid": 100000.0,
        "lighter": 50000.0,
    },
    
    # Order rate limits
    max_orders_per_second=5,
    max_orders_per_minute=100,
)
```

### Stop Loss Configuration

```python
from neleus import StopLossConfig, StopLossType

# Fixed percentage stop loss
stop_config = StopLossConfig(
    type=StopLossType.Fixed,
    stop_loss_pct=0.02,          # 2%
)

# ATR-based trailing stop
stop_config = StopLossConfig(
    type=StopLossType.ATR,
    atr_period=14,
    atr_multiplier=2.0,
)

# Trailing stop
stop_config = StopLossConfig(
    type=StopLossType.Trailing,
    trailing_pct=0.03,           # 3%
)

# Chandelier stop
stop_config = StopLossConfig(
    type=StopLossType.Chandelier,
    period=22,
    multiplier=3.0,
)
```

### Position Sizing

```python
from neleus import PositionSizingConfig, PositionSizingMethod

size_config = PositionSizingConfig(
    method=PositionSizingMethod.Kelly,
    
    # Kelly Criterion parameters
    win_rate=0.6,
    avg_win_loss_ratio=2.0,
    kelly_fraction=0.25,         # Use 25% of Kelly
    
    # Risk-based sizing
    max_risk_per_trade=0.01,     # Risk 1% per trade
    
    # Volatility-based
    target_volatility=0.15,       # 15% annualized
    
    # Fixed notional
    fixed_notional=10000.0,
)
```

---

## Venue Configuration

### Hyperliquid

```python
from neleus import HyperliquidConfig, Network

config = HyperliquidConfig(
    network=Network.Testnet,        # or Network.Mainnet
    private_key="0x...",
    wallet_address="0x...",
    
    # Optional
    vault_address="0x...",          # For vault trading
    
    # Connection
    ws_url=None,                    # Auto-detected from network
    rest_url=None,                  # Auto-detected from network
    
    # Rate limits
    max_orders_per_second=10,
    max_open_orders=1000,
    
    # Reconnection
    reconnect_timeout_secs=30,
    max_reconnect_attempts=5,
)
```

### Lighter (zkLighter)

```python
from neleus import LighterConfig, Network

config = LighterConfig(
    network=Network.Testnet,
    api_key="your_api_key",
    private_key="0x...",
    
    # Account tier affects fees
    account_tier="standard",        # standard, pro, vip
    
    # Connection
    ws_url=None,                    # Auto-detected
    rest_url=None,                  # Auto-detected
    
    # Rate limits (vary by tier)
    max_orders_per_second=5,
)
```

### Polymarket

```python
from neleus import PolymarketConfig

config = PolymarketConfig(
    api_key="your_api_key",
    api_secret="your_secret",
    
    # Auth
    l1_auth=True,                   # Level 1 auth (read-only)
    l2_auth=False,                  # Level 2 auth (trading)
    
    # Connection
    rest_url="https://api.polymarket.com",
    ws_url="wss://ws.polymarket.com",
)
```

---

## Logging Configuration

### Configure Logging

```python
from neleus.config import configure_logging

configure_logging(
    level="info",                   # debug, info, warning, error
    log_file="logs/neleus.log",
    max_bytes=100_000_000,          # 100 MB
    backup_count=5,
    console_output=True,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
```

### Log Levels

```python
import logging

# Debug - verbose output
logging.DEBUG

# Info - general information
logging.INFO

# Warning - warnings
logging.WARNING

# Error - errors
logging.ERROR

# Critical - critical errors
logging.CRITICAL
```

### Logging in Strategies

```python
from neleus import Strategy
import logging

class MyStrategy(Strategy):
    def __init__(self):
        super().__init__()
        self.logger = logging.getLogger(self.__class__.__name__)
    
    def on_bar(self, ctx, bar):
        self.logger.debug(f"Processing bar: {bar.close}")
        self.logger.info(f"Signal generated: BUY")
        self.logger.warning(f"High volatility detected")
        self.logger.error(f"Order failed: {error}")
```

---

## Configuration Best Practices

### 1. Use Environment-Specific Configs

```bash
.env.development
.env.testnet
.env.production
```

```python
import os
from neleus.config import load_env

env = os.getenv("NELEUS_ENV", "development")
load_env(f".env.{env}")
```

### 2. Never Hardcode Secrets

❌ **Bad:**
```python
config = HyperliquidConfig(
    private_key="0x1234567890..."  # Never do this!
)
```

 **Good:**
```python
import os
config = HyperliquidConfig(
    private_key=os.getenv("HYPERLIQUID_PRIVATE_KEY")
)
```

### 3. Validate Configuration

```python
from neleus.config import validate_config

# Raises exception if invalid
validate_config(config)
```

### 4. Use Configuration Profiles

```python
# configs/development.toml
[trading]
default_venue = "hyperliquid"
network = "testnet"

# configs/production.toml
[trading]
default_venue = "hyperliquid"
network = "mainnet"
```

```python
from neleus.config import load_config

config = load_config("configs/development.toml")
```

### 5. Override Configs Programmatically

```python
from neleus import BacktestConfig

# Load from file
config = BacktestConfig.from_file("backtest.toml")

# Override specific values
config.initial_capital = 200000.0
config.commission_bps = 3.0
```

---

## Configuration Validation

### Validate Before Running

```python
from neleus.config import ConfigValidator

validator = ConfigValidator()

# Validate backtest config
errors = validator.validate_backtest(config)
if errors:
    for error in errors:
        print(f"Error: {error}")
    exit(1)

# Validate risk config
errors = validator.validate_risk(risk_config)

# Validate venue config
errors = validator.validate_venue(venue_config)
```

---

## Example: Complete Configuration

```python
from neleus import (
    BacktestConfig,
    RiskConfig,
    HyperliquidConfig,
    Network,
    FillModel,
    LatencyModel,
)
import os

# Backtest settings
backtest_config = BacktestConfig(
    initial_capital=100000.0,
    commission_bps=5.0,
    slippage_bps=2.0,
    start_date="2024-01-01",
    end_date="2024-06-01",
    fill_model=FillModel.NextTick,
    latency_model=LatencyModel.Fixed,
    latency_ms=50,
)

# Risk management
risk_config = RiskConfig(
    max_position_pct=10.0,
    max_leverage=5.0,
    max_daily_loss_pct=5.0,
    dynamic_limits=True,
)

# Venue (for live/paper trading)
venue_config = HyperliquidConfig(
    network=Network.Testnet,
    private_key=os.getenv("HYPERLIQUID_PRIVATE_KEY"),
    wallet_address=os.getenv("HYPERLIQUID_WALLET"),
)

# Use in trading
from neleus import LiveNode

node = LiveNode(
    venue_config=venue_config,
    risk_config=risk_config,
)
```

---

## Troubleshooting

### Configuration Not Found

```bash
Error: neleus.toml not found
```

**Solution:** Run `neleus init` or `neleus new <project_name>`

### Invalid API Keys

```bash
Error: Authentication failed
```

**Solution:** Check `.env` file has correct API keys

### Permission Errors

```bash
Error: Cannot write to logs/
```

**Solution:** Check directory permissions or create logs directory

---

## See Also

- [Getting Started](./GETTING_STARTED.md)
- [Risk Management](./RISK_MANAGEMENT.md)
- [Venue Documentation](./VENUES.md)
- [API Reference](./API_REFERENCE.md)
