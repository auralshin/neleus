"""
Neleus Shared Constants
=======================

Named constants used across modules. Import from here instead of
writing magic numbers inline.
"""

# ---------------------------------------------------------------------------
# Data / timeframe defaults
# ---------------------------------------------------------------------------
DEFAULT_CANDLE_INTERVAL = "1h"
DEFAULT_LOOKBACK_DAYS = 30

# ---------------------------------------------------------------------------
# Fee / cost defaults (in basis points unless stated otherwise)
# ---------------------------------------------------------------------------
DEFAULT_MAKER_FEE_BPS = 2
DEFAULT_TAKER_FEE_BPS = 5
DEFAULT_SLIPPAGE_BPS = 5

# ---------------------------------------------------------------------------
# Capital / account defaults
# ---------------------------------------------------------------------------
DEFAULT_INITIAL_CAPITAL = 10_000.0

# ---------------------------------------------------------------------------
# Instrument defaults
# ---------------------------------------------------------------------------
DEFAULT_SYMBOL = "BTC-PERP"

# ---------------------------------------------------------------------------
# Networking defaults
# ---------------------------------------------------------------------------
DEFAULT_DASHBOARD_PORT = 8080
DEFAULT_SIGNAL_PORT = 8090
DEFAULT_POLL_INTERVAL_SECS = 1.0

# ---------------------------------------------------------------------------
# Signal defaults
# ---------------------------------------------------------------------------
DEFAULT_SIGNAL_TTL_HOURS = 1

# ---------------------------------------------------------------------------
# Technical-indicator constants
# ---------------------------------------------------------------------------
RSI_PERIOD = 14
SMA_PERIOD = 20
EMA_FAST = 12
EMA_SLOW = 26
BOLLINGER_STD = 2.0
RSI_OVERSOLD = 30
RSI_OVERBOUGHT = 70

# ---------------------------------------------------------------------------
# Statistics / annualisation
# ---------------------------------------------------------------------------
ANNUALIZATION_FACTOR = 252

# ---------------------------------------------------------------------------
# Time conversion helpers
# ---------------------------------------------------------------------------
MS_PER_HOUR = 3_600_000

__all__ = [
    "DEFAULT_CANDLE_INTERVAL",
    "DEFAULT_LOOKBACK_DAYS",
    "DEFAULT_MAKER_FEE_BPS",
    "DEFAULT_TAKER_FEE_BPS",
    "DEFAULT_SLIPPAGE_BPS",
    "DEFAULT_INITIAL_CAPITAL",
    "DEFAULT_SYMBOL",
    "DEFAULT_DASHBOARD_PORT",
    "DEFAULT_SIGNAL_PORT",
    "DEFAULT_POLL_INTERVAL_SECS",
    "DEFAULT_SIGNAL_TTL_HOURS",
    "RSI_PERIOD",
    "SMA_PERIOD",
    "EMA_FAST",
    "EMA_SLOW",
    "BOLLINGER_STD",
    "RSI_OVERSOLD",
    "RSI_OVERBOUGHT",
    "ANNUALIZATION_FACTOR",
    "MS_PER_HOUR",
]
