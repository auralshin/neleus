"""
Neleus Dashboard - Professional Trading Framework Interface

A clean, data-driven dashboard for backtesting and risk analysis.
Uses real market data from Hyperliquid API via Rust core.
"""

import streamlit as st
import pandas as pd
import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from datetime import datetime, timedelta
from pathlib import Path
import sys
import asyncio

# =============================================================================
# Page Configuration
# =============================================================================

def get_logo_base64():
    """Load logo as base64 for embedding."""
    import base64
    logo_path = Path(__file__).parent / "assets" / "logo.svg"
    if logo_path.exists():
        with open(logo_path, "rb") as f:
            data = f.read()
            return base64.b64encode(data).decode()

st.set_page_config(
    page_title="Neleus",
    page_icon="N",
    layout="wide",
    initial_sidebar_state="expanded"
)

# Professional CSS - dark theme, minimal, clean
st.markdown("""
<style>
    /* Main background */
    .stApp {
        background-color: #0a0a0f;
    }
    
    /* Sidebar */
    [data-testid="stSidebar"] {
        background-color: #0f0f14;
        border-right: 1px solid #1a1a24;
    }
    
    /* Headers */
    h1, h2, h3 {
        color: #e5e5e5 !important;
        font-weight: 500 !important;
    }
    
    /* Metric cards */
    [data-testid="stMetricValue"] {
        font-size: 1.8rem !important;
        color: #ffffff !important;
    }
    
    [data-testid="stMetricDelta"] {
        font-size: 0.85rem !important;
    }
    
    /* Remove excess padding */
    .block-container {
        padding-top: 2rem !important;
        padding-bottom: 1rem !important;
    }
    
    /* Tables */
    .stDataFrame {
        border: 1px solid #1a1a24;
        border-radius: 8px;
    }
    
    /* Buttons */
    .stButton > button {
        background-color: #2f9171;
        color: white;
        border: none;
        border-radius: 6px;
        font-weight: 500;
    }
    
    .stButton > button:hover {
        background-color: #3db186;
    }
    
    /* Select boxes */
    .stSelectbox > div > div {
        background-color: #1a1a24;
        border: 1px solid #262630;
    }
    
    /* Dividers */
    hr {
        border-color: #1a1a24 !important;
    }
    
    /* Logo container */
    .logo-container {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: 24px;
    }
    
    .logo-container svg {
        width: 40px;
        height: auto;
    }
    
    .logo-text {
        font-size: 1.5rem;
        font-weight: 600;
        color: #ffffff;
        letter-spacing: -0.02em;
    }
    
    /* Status indicator */
    .status-ok { color: #2f9171; }
    .status-warn { color: #f59e0b; }
    .status-error { color: #ef4444; }
    
    /* Coming soon banner */
    .coming-soon-banner {
        background: linear-gradient(135deg, #1a1a24 0%, #0f0f14 100%);
        border: 1px dashed #2a2a34;
        border-radius: 12px;
        padding: 48px;
        text-align: center;
        margin: 24px 0;
    }
    
    .coming-soon-banner h2 {
        color: #6b7280 !important;
        margin-bottom: 8px;
    }
    
    .coming-soon-banner p {
        color: #4b5563;
    }
</style>
""", unsafe_allow_html=True)

# =============================================================================
# Data Layer - Hyperliquid Integration
# =============================================================================

def get_hyperliquid_client():
    """Get Hyperliquid client from Rust core."""
    try:
        sys.path.insert(0, str(Path(__file__).parent.parent.parent))
        from neleus.types import HyperliquidClient
        return HyperliquidClient(testnet=False)
    except Exception as e:
        return None

@st.cache_data(ttl=60)
def fetch_market_data(coin: str = "BTC", days: int = 30):
    """Fetch real market data from Hyperliquid via Rust core."""
    try:
        sys.path.insert(0, str(Path(__file__).parent.parent.parent))
        from neleus.types import HyperliquidClient
        
        client = HyperliquidClient(testnet=False)
        
        end_time_ms = int(datetime.now().timestamp() * 1000)
        start_time_ms = end_time_ms - (days * 24 * 60 * 60 * 1000)
        
        candles = client.fetch_candles(
            coin,
            "1h",
            start_time_ms,
            end_time_ms
        )
        
        if candles:
            df = pd.DataFrame([{
                "timestamp": datetime.fromtimestamp(c.timestamp / 1000),
                "open": c.open,
                "high": c.high,
                "low": c.low,
                "close": c.close,
                "volume": c.volume
            } for c in candles])
            df = df.sort_values("timestamp").reset_index(drop=True)
            df["returns"] = df["close"].pct_change()
            return df
            
    except Exception as e:
        st.session_state["api_error"] = str(e)
    
    return None

@st.cache_data(ttl=300)
def fetch_market_meta():
    """Fetch market metadata from Hyperliquid with leverage info."""
    try:
        sys.path.insert(0, str(Path(__file__).parent.parent.parent))
        from neleus.types import HyperliquidClient
        
        client = HyperliquidClient(testnet=False)
        meta = client.fetch_meta()
        
        if meta and hasattr(meta, 'symbols'):
            return [s.name for s in meta.symbols]
    except:
        pass
    
    return ["BTC", "ETH", "SOL", "ARB", "DOGE", "AVAX", "MATIC", "OP", "LINK"]


@st.cache_data(ttl=300)
def fetch_all_markets_with_info():
    """Fetch all markets with metadata (leverage, decimals)."""
    try:
        sys.path.insert(0, str(Path(__file__).parent.parent.parent))
        from neleus.types import HyperliquidClient
        
        client = HyperliquidClient(testnet=False)
        meta = client.fetch_meta()
        
        if meta and hasattr(meta, 'symbols'):
            return [{
                "name": s.name,
                "max_leverage": s.max_leverage,
                "sz_decimals": s.sz_decimals
            } for s in meta.symbols]
    except:
        pass
    
    return []

def get_project_root() -> Path:
    """Get project root directory."""
    current = Path(__file__).parent
    while current != current.parent:
        # Prefer Cargo.toml as it's at the true project root
        if (current / "Cargo.toml").exists():
            return current
        current = current.parent
    # Fallback to searching for pyproject.toml
    current = Path(__file__).parent
    while current != current.parent:
        if (current / "pyproject.toml").exists():
            # If pyproject.toml is in a subdirectory, go up one more level
            if (current.parent / "Cargo.toml").exists():
                return current.parent
            return current
        current = current.parent
    return Path.cwd()

def find_strategies(project_path: Path | None = None) -> list:
    """Find available strategy files."""
    if project_path is None:
        project_path = get_project_root()
    
    strategies = []
    
    examples_dir = project_path / "examples"
    if examples_dir.exists():
        for f in examples_dir.glob("*.py"):
            if f.name.startswith("_") or f.name.startswith("test_"):
                continue
            # Include files with common strategy naming patterns
            patterns = [
                "backtest", "strategy", "momentum", "reversion", "statistical",
                "grid", "market_maker", "funding", "breakout", "rsi", "scalper",
                "volatility", "mean"
            ]
            if any(p in f.name for p in patterns):
                name = f.stem.replace("_backtest", "").replace("_strategy", "")
                strategies.append({"name": name.replace("_", " ").title(), "path": str(f), "source": "examples"})
    
    strategies_dir = project_path / "strategies"
    if strategies_dir.exists():
        for f in strategies_dir.glob("*.py"):
            if not f.name.startswith("_"):
                strategies.append({"name": f.stem.replace("_", " ").title(), "path": str(f), "source": "project"})
    
    return strategies

# =============================================================================
# Risk Analytics
# =============================================================================

def calculate_risk_metrics(returns) -> dict:
    """Calculate comprehensive risk metrics."""
    returns = np.array(returns)
    returns = returns[~np.isnan(returns)]
    
    if len(returns) < 2:
        return {}
    
    ann_factor = 24 * 365
    
    total_return = float(np.prod(1 + returns) - 1)
    mean_return = float(np.mean(returns) * ann_factor)
    volatility = float(np.std(returns) * np.sqrt(ann_factor))
    
    sharpe = float((mean_return - 0.02) / volatility) if volatility > 0 else 0
    
    downside = returns[returns < 0]
    downside_std = float(np.std(downside) * np.sqrt(ann_factor)) if len(downside) > 0 else 0.001
    sortino = float((mean_return - 0.02) / downside_std) if downside_std > 0 else 0
    
    cumulative = pd.Series(np.cumprod(1 + returns))
    rolling_max = cumulative.expanding().max()
    drawdowns = (cumulative - rolling_max) / rolling_max
    max_dd = float(drawdowns.min())
    
    calmar = float(mean_return / abs(max_dd)) if max_dd != 0 else 0
    
    var_95 = float(np.percentile(returns, 5))
    var_99 = float(np.percentile(returns, 1))
    
    below_var = returns[returns <= var_95]
    cvar_95 = float(np.mean(below_var)) if len(below_var) > 0 else var_95
    
    win_rate = float(np.sum(returns > 0) / len(returns))
    
    # Additional metrics
    skewness = float(pd.Series(returns).skew()) if len(returns) > 2 else 0
    kurtosis = float(pd.Series(returns).kurtosis()) if len(returns) > 3 else 0
    
    return {
        "total_return": total_return,
        "annualized_return": mean_return,
        "volatility": volatility,
        "sharpe_ratio": sharpe,
        "sortino_ratio": sortino,
        "max_drawdown": max_dd,
        "calmar_ratio": calmar,
        "var_95": var_95,
        "var_99": var_99,
        "cvar_95": cvar_95,
        "win_rate": win_rate,
        "skewness": skewness,
        "kurtosis": kurtosis,
    }


def calculate_rolling_metrics(returns, window: int = 24) -> dict:
    """Calculate rolling risk metrics."""
    returns = pd.Series(returns)
    
    rolling_vol = returns.rolling(window).std() * np.sqrt(24 * 365)
    rolling_sharpe = (returns.rolling(window).mean() * 24 * 365 - 0.02) / rolling_vol
    
    # Rolling max drawdown
    cumulative = (1 + returns).cumprod()
    rolling_max = cumulative.rolling(window).max()
    rolling_dd = (cumulative - rolling_max) / rolling_max
    
    return {
        "rolling_volatility": rolling_vol,
        "rolling_sharpe": rolling_sharpe,
        "rolling_drawdown": rolling_dd,
    }


def calculate_risk_decomposition(returns, prices=None) -> dict:
    """Decompose risk into components."""
    returns = np.array(returns)
    returns = returns[~np.isnan(returns)]
    
    if len(returns) < 10:
        return {}
    
    total_var = float(np.var(returns))
    
    # Separate upside and downside variance
    upside_returns = returns[returns > 0]
    downside_returns = returns[returns < 0]
    
    upside_var = float(np.var(upside_returns)) if len(upside_returns) > 1 else 0
    downside_var = float(np.var(downside_returns)) if len(downside_returns) > 1 else 0
    
    # Tail risk (beyond 2 std)
    std = np.std(returns)
    mean = np.mean(returns)
    tail_returns = returns[np.abs(returns - mean) > 2 * std]
    tail_risk = float(np.var(tail_returns)) if len(tail_returns) > 1 else 0
    
    # Concentration metrics
    sorted_abs_returns = np.sort(np.abs(returns))[::-1]
    top_10_pct = sorted_abs_returns[:max(1, len(returns) // 10)]
    concentration_risk = float(np.sum(top_10_pct ** 2) / np.sum(returns ** 2)) if np.sum(returns ** 2) > 0 else 0
    
    return {
        "total_variance": total_var,
        "upside_variance": upside_var,
        "downside_variance": downside_var,
        "tail_risk": tail_risk,
        "concentration_risk": concentration_risk,
        "variance_ratio": float(downside_var / upside_var) if upside_var > 0 else 0,
    }


def calculate_correlation_analysis(returns_dict: dict) -> pd.DataFrame:
    """Calculate correlation matrix between multiple return series."""
    if not returns_dict or len(returns_dict) < 2:
        return pd.DataFrame()
    
    df = pd.DataFrame(returns_dict)
    return df.corr()


def calculate_nav_distribution(equity_curve: list, initial_capital: float) -> dict:
    """Analyze NAV distribution and performance attribution."""
    if not equity_curve:
        return {}
    
    values = [e[1] for e in equity_curve]
    returns = np.diff(values) / values[:-1]
    
    # NAV statistics
    nav_mean = float(np.mean(values))
    nav_std = float(np.std(values))
    nav_min = float(np.min(values))
    nav_max = float(np.max(values))
    
    # Distribution percentiles
    percentiles = [5, 10, 25, 50, 75, 90, 95]
    nav_percentiles = {f"p{p}": float(np.percentile(values, p)) for p in percentiles}
    
    # Performance attribution
    total_gain = max(0, values[-1] - initial_capital)
    total_loss = max(0, initial_capital - values[-1])
    
    # Recovery analysis
    underwater = [initial_capital - v for v in values if v < initial_capital]
    avg_underwater = float(np.mean(underwater)) if underwater else 0
    time_underwater = len(underwater) / len(values) if values else 0
    
    return {
        "nav_mean": nav_mean,
        "nav_std": nav_std,
        "nav_min": nav_min,
        "nav_max": nav_max,
        "nav_range": nav_max - nav_min,
        "percentiles": nav_percentiles,
        "total_gain": total_gain,
        "total_loss": total_loss,
        "avg_underwater": avg_underwater,
        "time_underwater_pct": time_underwater * 100,
    }

# Pre-built strategy configurations for Monte Carlo simulation
MONTE_CARLO_STRATEGIES = {
    "Momentum": {
        "description": "Trend-following strategy that buys on upward momentum and sells on downward momentum",
        "default_params": {
            "lookback_period": 20,
            "entry_threshold": 0.005,
            "exit_threshold": 0.002,
            "position_size": 0.10,
            "stop_loss": 0.03,
            "take_profit": 0.06,
        },
        "param_ranges": {
            "lookback_period": (5, 50),
            "entry_threshold": (0.001, 0.02),
            "exit_threshold": (0.001, 0.01),
            "position_size": (0.05, 0.25),
            "stop_loss": (0.01, 0.10),
            "take_profit": (0.02, 0.15),
        }
    },
    "Mean Reversion": {
        "description": "Counter-trend strategy that buys oversold conditions and sells overbought conditions",
        "default_params": {
            "lookback_period": 20,
            "std_threshold": 2.0,
            "position_size": 0.15,
            "stop_loss": 0.04,
            "take_profit": 0.05,
            "max_hold_periods": 24,
        },
        "param_ranges": {
            "lookback_period": (10, 60),
            "std_threshold": (1.0, 3.5),
            "position_size": (0.05, 0.30),
            "stop_loss": (0.02, 0.08),
            "take_profit": (0.02, 0.10),
            "max_hold_periods": (6, 72),
        }
    },
    "RSI Reversal": {
        "description": "Mean reversion using RSI indicator for overbought/oversold detection",
        "default_params": {
            "rsi_period": 14,
            "oversold_level": 30,
            "overbought_level": 70,
            "position_size": 0.12,
            "stop_loss": 0.035,
            "take_profit": 0.045,
        },
        "param_ranges": {
            "rsi_period": (7, 28),
            "oversold_level": (15, 40),
            "overbought_level": (60, 85),
            "position_size": (0.05, 0.25),
            "stop_loss": (0.02, 0.08),
            "take_profit": (0.02, 0.10),
        }
    },
    "Breakout": {
        "description": "Trades breakouts from consolidation ranges with momentum confirmation",
        "default_params": {
            "lookback_period": 24,
            "breakout_threshold": 0.008,
            "volume_multiplier": 1.5,
            "position_size": 0.10,
            "stop_loss": 0.025,
            "take_profit": 0.08,
        },
        "param_ranges": {
            "lookback_period": (12, 48),
            "breakout_threshold": (0.003, 0.02),
            "volume_multiplier": (1.0, 3.0),
            "position_size": (0.05, 0.20),
            "stop_loss": (0.015, 0.06),
            "take_profit": (0.03, 0.15),
        }
    },
    "Volatility Scalper": {
        "description": "High-frequency scalping strategy targeting volatility expansion",
        "default_params": {
            "atr_period": 14,
            "volatility_threshold": 1.5,
            "position_size": 0.08,
            "stop_loss": 0.015,
            "take_profit": 0.025,
            "max_trades_per_day": 10,
        },
        "param_ranges": {
            "atr_period": (7, 21),
            "volatility_threshold": (1.0, 3.0),
            "position_size": (0.03, 0.15),
            "stop_loss": (0.008, 0.03),
            "take_profit": (0.01, 0.05),
            "max_trades_per_day": (3, 20),
        }
    },
    "Grid Trading": {
        "description": "Market-making strategy with orders placed at regular price intervals",
        "default_params": {
            "grid_levels": 10,
            "grid_spacing": 0.01,
            "position_per_level": 0.05,
            "stop_loss": 0.08,
            "take_profit": 0.10,
            "rebalance_threshold": 0.05,
        },
        "param_ranges": {
            "grid_levels": (5, 20),
            "grid_spacing": (0.005, 0.025),
            "position_per_level": (0.02, 0.10),
            "stop_loss": (0.05, 0.15),
            "take_profit": (0.05, 0.20),
            "rebalance_threshold": (0.02, 0.10),
        }
    },
}

# Scenario presets for Monte Carlo simulation
MONTE_CARLO_SCENARIOS = {
    "Normal Market": {
        "description": "Typical market conditions with moderate volatility",
        "vol_multiplier": 1.0,
        "drift_adjustment": 0.0,
        "jump_probability": 0.0,
        "jump_magnitude": 0.0,
    },
    "High Volatility": {
        "description": "Elevated volatility regime (2x normal)",
        "vol_multiplier": 2.0,
        "drift_adjustment": 0.0,
        "jump_probability": 0.0,
        "jump_magnitude": 0.0,
    },
    "Bull Market": {
        "description": "Upward trending market with positive drift",
        "vol_multiplier": 0.9,
        "drift_adjustment": 0.0003,
        "jump_probability": 0.01,
        "jump_magnitude": 0.02,
    },
    "Bear Market": {
        "description": "Downward trending market with negative drift",
        "vol_multiplier": 1.3,
        "drift_adjustment": -0.0004,
        "jump_probability": 0.02,
        "jump_magnitude": -0.03,
    },
    "Crash Scenario": {
        "description": "Extreme market stress with large downward jumps",
        "vol_multiplier": 3.0,
        "drift_adjustment": -0.001,
        "jump_probability": 0.05,
        "jump_magnitude": -0.08,
    },
    "Low Volatility": {
        "description": "Compressed volatility regime (0.5x normal)",
        "vol_multiplier": 0.5,
        "drift_adjustment": 0.0,
        "jump_probability": 0.0,
        "jump_magnitude": 0.0,
    },
    "Mean Reverting": {
        "description": "Range-bound market with strong mean reversion",
        "vol_multiplier": 0.8,
        "drift_adjustment": 0.0,
        "jump_probability": 0.0,
        "jump_magnitude": 0.0,
        "mean_reversion_strength": 0.1,
    },
    "Trending": {
        "description": "Strong trending behavior with momentum",
        "vol_multiplier": 1.1,
        "drift_adjustment": 0.0002,
        "jump_probability": 0.005,
        "jump_magnitude": 0.015,
        "autocorrelation": 0.15,
    },
}


def run_monte_carlo_simulation(
    returns: np.ndarray,
    strategy_type: str,
    params: dict,
    scenario: dict,
    n_simulations: int = 1000,
    n_periods: int = 252,
    initial_capital: float = 100000.0,
    confidence_levels: list = [0.95, 0.99],
) -> dict:
    """
    Run Monte Carlo simulation for a given strategy and market scenario.
    
    Uses Geometric Brownian Motion (GBM) with optional jumps and regime changes.
    """
    # Calculate base statistics from historical returns
    mu = float(np.mean(returns))
    sigma = float(np.std(returns))
    
    # Apply scenario adjustments
    vol_mult = scenario.get("vol_multiplier", 1.0)
    drift_adj = scenario.get("drift_adjustment", 0.0)
    jump_prob = scenario.get("jump_probability", 0.0)
    jump_mag = scenario.get("jump_magnitude", 0.0)
    mean_rev = scenario.get("mean_reversion_strength", 0.0)
    autocorr = scenario.get("autocorrelation", 0.0)
    
    adjusted_sigma = sigma * vol_mult
    adjusted_mu = mu + drift_adj
    
    # Strategy-specific parameters
    stop_loss = params.get("stop_loss", 0.03)
    take_profit = params.get("take_profit", 0.06)
    position_size = params.get("position_size", 0.10)
    
    # Initialize results storage
    all_equity_curves = np.zeros((n_simulations, n_periods + 1))
    all_equity_curves[:, 0] = initial_capital
    
    final_values = np.zeros(n_simulations)
    max_drawdowns = np.zeros(n_simulations)
    sharpe_ratios = np.zeros(n_simulations)
    total_returns = np.zeros(n_simulations)
    win_rates = np.zeros(n_simulations)
    
    np.random.seed(42)  # For reproducibility
    
    # Progress tracking
    progress_interval = max(1, n_simulations // 20)  # Update every 5%
    
    for sim in range(n_simulations):
        cash = initial_capital
        position_value = 0.0  # Current value of position
        position_shares = 0.0  # Number of shares/units held
        position_side = 0  # 1 for long, -1 for short, 0 for flat
        entry_price = 0.0
        price = 100.0  # Starting price
        peak_equity = initial_capital
        max_dd = 0.0
        
        trade_returns = []
        wins = 0
        losses = 0
        prev_return = 0.0
        holding_periods = 0
        
        for t in range(n_periods):
            # Generate return with scenario characteristics
            random_return = np.random.normal(adjusted_mu, adjusted_sigma)
            
            # Add autocorrelation if specified
            if autocorr > 0:
                random_return = autocorr * prev_return + (1 - autocorr) * random_return
            
            # Add mean reversion if specified
            if mean_rev > 0 and t > 0:
                price_deviation = (price - 100.0) / 100.0
                random_return -= mean_rev * price_deviation
            
            # Add jump component
            if jump_prob > 0 and np.random.random() < jump_prob:
                random_return += jump_mag * (0.5 + np.random.random())
            
            # Update price
            price = price * (1 + random_return)
            prev_return = random_return
            
            # Update position value based on price movement
            if position_shares != 0:
                position_value = position_shares * price
                holding_periods += 1
            
            # Calculate current equity
            equity = cash + position_value
            
            # Strategy logic
            if position_side == 0:  # No position
                # Entry logic based on strategy type
                should_enter = False
                direction = 0
                
                if strategy_type == "Momentum":
                    # Use adaptive threshold based on volatility (0.5 to 1 sigma moves)
                    threshold = params.get("entry_threshold", 0.02)
                    # Scale to actual data characteristics
                    adaptive_threshold = max(threshold, 0.5 * adjusted_sigma)
                    if random_return > adaptive_threshold:
                        should_enter = True
                        direction = 1
                    elif random_return < -adaptive_threshold:
                        should_enter = True
                        direction = -1
                        
                elif strategy_type == "Mean Reversion":
                    # Use standard deviation threshold (more realistic)
                    threshold = params.get("std_threshold", 2.0) * adjusted_sigma
                    if random_return < -threshold:
                        should_enter = True
                        direction = 1
                    elif random_return > threshold:
                        should_enter = True
                        direction = -1
                        
                elif strategy_type == "RSI Reversal":
                    # More moderate RSI-like behavior (1.5-2 sigma)
                    threshold = 1.8 * adjusted_sigma
                    if random_return < -threshold:
                        should_enter = True
                        direction = 1
                    elif random_return > threshold:
                        should_enter = True
                        direction = -1
                        
                elif strategy_type == "Breakout":
                    # Adaptive breakout based on volatility
                    threshold = params.get("breakout_threshold", 0.015)
                    adaptive_threshold = max(threshold, 1.2 * adjusted_sigma)
                    if abs(random_return) > adaptive_threshold:
                        should_enter = True
                        direction = 1 if random_return > 0 else -1
                        
                elif strategy_type == "Volatility Scalper":
                    # Lower threshold for more frequent entries
                    vol_threshold = params.get("volatility_threshold", 1.5) * adjusted_sigma
                    if abs(random_return) > vol_threshold:
                        should_enter = True
                        direction = 1 if random_return > 0 else -1
                        
                elif strategy_type == "Grid Trading":
                    # Grid trading enters positions more frequently
                    if np.random.random() < 0.25:  # 25% chance each period
                        should_enter = True
                        direction = 1 if np.random.random() < 0.5 else -1
                
                if should_enter and direction != 0:
                    # Enter position
                    position_size_dollars = equity * position_size
                    
                    if direction == 1:  # Long
                        position_shares = position_size_dollars / price
                        cash -= position_size_dollars
                    else:  # Short
                        position_shares = -position_size_dollars / price
                        cash += position_size_dollars
                    
                    position_side = direction
                    entry_price = price
                    position_value = position_shares * price
                    holding_periods = 0
                    
            else:  # Have a position
                # Calculate P&L
                if position_side == 1:  # Long position
                    pnl_pct = (price - entry_price) / entry_price
                else:  # Short position
                    pnl_pct = (entry_price - price) / entry_price
                
                should_exit = False
                exit_reason = None
                
                # Stop loss
                if pnl_pct <= -stop_loss:
                    should_exit = True
                    exit_reason = "stop_loss"
                    losses += 1
                
                # Take profit
                elif pnl_pct >= take_profit:
                    should_exit = True
                    exit_reason = "take_profit"
                    wins += 1
                
                # Strategy-specific exits
                elif strategy_type == "Mean Reversion":
                    # Exit when price reverts or small profit
                    if (abs(random_return) < adjusted_sigma * 0.5 and holding_periods > 3) or (pnl_pct > 0.01 and holding_periods > 5):
                        should_exit = True
                        exit_reason = "reversion"
                        if pnl_pct > 0:
                            wins += 1
                        else:
                            losses += 1
                
                elif strategy_type == "Grid Trading":
                    # Exit after fixed periods or profit target
                    max_hold = params.get("max_hold_periods", 10)
                    if holding_periods >= max_hold or pnl_pct >= take_profit * 0.5:
                        should_exit = True
                        exit_reason = "grid_rebalance"
                        if pnl_pct > 0:
                            wins += 1
                        else:
                            losses += 1
                
                elif holding_periods > params.get("max_hold_periods", 50):
                    # Max holding period exit
                    should_exit = True
                    exit_reason = "max_hold"
                    if pnl_pct > 0:
                        wins += 1
                    else:
                        losses += 1
                
                if should_exit:
                    # Close position
                    if position_side == 1:  # Close long
                        cash += position_shares * price
                    else:  # Close short
                        cash -= abs(position_shares) * price
                    
                    # Record trade return
                    trade_return = pnl_pct
                    trade_returns.append(trade_return)
                    
                    # Reset position
                    position_shares = 0.0
                    position_value = 0.0
                    position_side = 0
                    entry_price = 0.0
                    holding_periods = 0
            
            # Update equity (cash + position value)
            equity = cash + (position_shares * price if position_shares != 0 else 0)
            
            # Update equity curve
            all_equity_curves[sim, t + 1] = equity
            
            # Update drawdown
            if equity > peak_equity:
                peak_equity = equity
            dd = (peak_equity - equity) / peak_equity if peak_equity > 0 else 0
            if dd > max_dd:
                max_dd = dd
        
        # Close any remaining position at end
        if position_shares != 0:
            if position_side == 1:
                cash += position_shares * price
            else:
                cash -= abs(position_shares) * price
            
            if position_side == 1:
                pnl_pct = (price - entry_price) / entry_price
            else:
                pnl_pct = (entry_price - price) / entry_price
            
            trade_returns.append(pnl_pct)
            if pnl_pct > 0:
                wins += 1
            else:
                losses += 1
        
        final_equity = cash
        all_equity_curves[sim, -1] = final_equity
        final_values[sim] = final_equity
        max_drawdowns[sim] = max_dd
        total_returns[sim] = (final_equity - initial_capital) / initial_capital
        
        # Calculate Sharpe ratio from trade returns
        if len(trade_returns) > 1 and np.std(trade_returns) > 0:
            sharpe_ratios[sim] = np.mean(trade_returns) / np.std(trade_returns) * np.sqrt(252 / max(1, len(trade_returns)))
        else:
            sharpe_ratios[sim] = 0.0
            
        total_trades = wins + losses
        win_rates[sim] = wins / total_trades if total_trades > 0 else 0.0
    
    # Calculate statistics
    results = {
        "equity_curves": all_equity_curves,
        "final_values": final_values,
        "max_drawdowns": max_drawdowns,
        "sharpe_ratios": sharpe_ratios,
        "total_returns": total_returns,
        "win_rates": win_rates,
        
        # Summary statistics
        "mean_final_value": float(np.mean(final_values)),
        "median_final_value": float(np.median(final_values)),
        "std_final_value": float(np.std(final_values)),
        "mean_return": float(np.mean(total_returns)),
        "median_return": float(np.median(total_returns)),
        "std_return": float(np.std(total_returns)),
        "mean_max_drawdown": float(np.mean(max_drawdowns)),
        "median_max_drawdown": float(np.median(max_drawdowns)),
        "worst_drawdown": float(np.max(max_drawdowns)),
        "mean_sharpe": float(np.mean(sharpe_ratios)),
        "median_sharpe": float(np.median(sharpe_ratios)),
        "mean_win_rate": float(np.mean(win_rates)),
        
        # Percentiles
        "return_percentiles": {
            f"p{int((1-cl)*100)}": float(np.percentile(total_returns, (1-cl)*100))
            for cl in confidence_levels
        },
        "drawdown_percentiles": {
            f"p{int(cl*100)}": float(np.percentile(max_drawdowns, cl*100))
            for cl in confidence_levels
        },
        
        # Risk metrics
        "probability_of_loss": float(np.mean(total_returns < 0)),
        "probability_of_ruin": float(np.mean(final_values < initial_capital * 0.5)),
        "expected_shortfall_5": float(np.mean(total_returns[total_returns <= np.percentile(total_returns, 5)])),
        
        # Value at Risk
        "var_95": float(np.percentile(total_returns, 5)),
        "var_99": float(np.percentile(total_returns, 1)),
        "cvar_95": float(np.mean(total_returns[total_returns <= np.percentile(total_returns, 5)])),
    }
    
    return results

# =============================================================================
# Backtest Runner
# =============================================================================

def run_backtest(strategy_path: str, coin: str, interval: str, 
                 lookback_days: int, initial_capital: float) -> dict:
    """Run backtest using the Rust core engine and return comprehensive results."""
    try:
        project_root = get_project_root()
        sys.path.insert(0, str(project_root / "python"))
        
        import importlib.util
        spec = importlib.util.spec_from_file_location("strategy_module", strategy_path)
        if spec is None or spec.loader is None:
            return {"error": "Could not load strategy file", "success": False}
        
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        
        from neleus import Strategy, HyperliquidBacktestConfig, HyperliquidBacktestNode, CandleInterval
        from neleus.types import HyperliquidClient
        from decimal import Decimal
        
        strategy_class = None
        for attr_name in dir(module):
            attr = getattr(module, attr_name)
            if isinstance(attr, type) and issubclass(attr, Strategy) and attr is not Strategy:
                strategy_class = attr
                break
        
        if strategy_class is None:
            return {"error": "No Strategy class found in file", "success": False}
        
        interval_map = {
            "1m": CandleInterval.MIN_1,
            "5m": CandleInterval.MIN_5,
            "15m": CandleInterval.MIN_15,
            "1h": CandleInterval.HOUR_1,
            "4h": CandleInterval.HOUR_4,
            "1d": CandleInterval.DAY_1,
        }
        
        config = HyperliquidBacktestConfig(
            coin=coin,
            interval=interval_map.get(interval, CandleInterval.HOUR_1),
            lookback_days=lookback_days,
            testnet=False,
            initial_capital=Decimal(str(initial_capital)),
            maker_fee_bps=2.0,
            taker_fee_bps=5.0,
            slippage_bps=5.0,
        )
        
        node = HyperliquidBacktestNode(config)
        strategy = strategy_class()
        node.add_strategy(strategy)
        
        result = asyncio.run(node.run_async())
        
        # Extract metrics from result (BacktestResults has .metrics attribute)
        metrics = result.metrics if hasattr(result, 'metrics') else None
        
        # Also fetch the candle data for charting
        client = HyperliquidClient(testnet=False)
        end_time_ms = int(datetime.now().timestamp() * 1000)
        start_time_ms = end_time_ms - (lookback_days * 24 * 60 * 60 * 1000)
        
        candles = client.fetch_candles(coin, interval, start_time_ms, end_time_ms)
        
        candle_data = []
        if candles:
            for c in candles:
                candle_data.append({
                    "timestamp": datetime.fromtimestamp(c.timestamp / 1000),
                    "open": c.open,
                    "high": c.high,
                    "low": c.low,
                    "close": c.close,
                    "volume": c.volume
                })
        
        # Get equity curve from result
        equity_curve = getattr(result, 'equity_curve', [])
        final_balance = equity_curve[-1][1] if equity_curve else initial_capital
        total_pnl = final_balance - initial_capital
        
        if metrics:
            return {
                "success": True,
                "initial_balance": initial_capital,
                "final_balance": final_balance,
                "total_pnl": total_pnl,
                "return_pct": metrics.total_return * 100,  # Convert to percentage
                "total_trades": metrics.total_trades,
                "winning_trades": metrics.winning_trades,
                "losing_trades": metrics.losing_trades,
                "win_rate": metrics.win_rate * 100,  # Convert to percentage
                "max_drawdown_pct": metrics.max_drawdown * 100,  # Convert to percentage
                "sharpe_ratio": metrics.sharpe_ratio,
                "sortino_ratio": metrics.sortino_ratio,
                "calmar_ratio": metrics.calmar_ratio,
                "total_volume": 0,
                "total_commission": metrics.total_commission,
                "avg_trade_pnl": metrics.avg_trade_pnl,
                "max_consecutive_wins": 0,
                "max_consecutive_losses": 0,
                "profit_factor": metrics.profit_factor,
                "expectancy": metrics.avg_trade_pnl,
                "candles": candle_data,
                "equity_curve": equity_curve,
                "fills": getattr(result, 'fills', []),
                "coin": coin,
                "interval": interval,
                "lookback_days": lookback_days,
            }
        else:
            return {
                "success": True,
                "initial_balance": initial_capital,
                "final_balance": final_balance,
                "total_pnl": total_pnl,
                "return_pct": (total_pnl / initial_capital) * 100 if initial_capital > 0 else 0,
                "total_trades": len(getattr(result, 'fills', [])),
                "winning_trades": 0,
                "losing_trades": 0,
                "win_rate": 0,
                "max_drawdown_pct": 0,
                "sharpe_ratio": 0,
                "sortino_ratio": 0,
                "calmar_ratio": 0,
                "total_volume": 0,
                "total_commission": 0,
                "avg_trade_pnl": 0,
                "max_consecutive_wins": 0,
                "max_consecutive_losses": 0,
                "profit_factor": 0,
                "expectancy": 0,
                "candles": candle_data,
                "equity_curve": equity_curve,
                "fills": getattr(result, 'fills', []),
                "coin": coin,
                "interval": interval,
                "lookback_days": lookback_days,
            }
        
    except Exception as e:
        import traceback
        return {"error": str(e), "traceback": traceback.format_exc(), "success": False}

# =============================================================================
# Page Components
# =============================================================================

def render_sidebar():
    """Render the sidebar navigation."""
    with st.sidebar:
        logo_base64 = get_logo_base64()
        if logo_base64:
            st.markdown(f"""
            <div class="logo-container">
                <img src="data:image/svg+xml;base64,{logo_base64}" alt="Neleus" style="width: 40px; height: auto;">
                <span class="logo-text">Neleus</span>
            </div>
            """, unsafe_allow_html=True)
        else:
            st.markdown("### Neleus")
        
        st.caption("Trading Framework v0.1.0")
        
        st.markdown("---")
        
        page = st.radio(
            "Navigation",
            ["Overview", "Risk Analysis", "Monte Carlo", "Portfolio", "Backtest", "Live Trading", "Deployment"],
            label_visibility="collapsed"
        )
        
        st.markdown("---")
        
        st.markdown("**System**")
        try:
            sys.path.insert(0, str(Path(__file__).parent.parent.parent))
            from neleus.types import using_rust_types
            st.markdown('<span class="status-ok">● Rust Core Active</span>', unsafe_allow_html=True)
        except:
            st.markdown('<span class="status-error">● Rust Core Unavailable</span>', unsafe_allow_html=True)
        
        return page


def render_overview():
    """Render the Overview page with real market data."""
    st.title("Market Overview")
    
    col1, col2, col3 = st.columns([1, 1, 2])
    with col1:
        assets = fetch_market_meta()
        selected_asset = st.selectbox("Asset", assets, index=0)
    with col2:
        timeframe = st.selectbox("Period", ["7 days", "30 days", "90 days"], index=1)
        days = {"7 days": 7, "30 days": 30, "90 days": 90}[timeframe]
    
    df = fetch_market_data(selected_asset, days)
    
    if df is not None and len(df) > 0:
        metrics = calculate_risk_metrics(df["returns"].dropna())
        
        st.markdown("---")
        col1, col2, col3, col4, col5 = st.columns(5)
        
        with col1:
            current_price = df["close"].iloc[-1]
            prev_price = df["close"].iloc[-24] if len(df) > 24 else df["close"].iloc[0]
            change_24h = (current_price - prev_price) / prev_price * 100
            st.metric("Price", f"${current_price:,.2f}", f"{change_24h:+.2f}%")
        
        with col2:
            vol_24h = df["volume"].tail(24).sum()
            st.metric("Volume (24h)", f"${vol_24h:,.0f}")
        
        with col3:
            st.metric("Volatility (Ann.)", f"{metrics.get('volatility', 0)*100:.1f}%")
        
        with col4:
            st.metric("Max Drawdown", f"{metrics.get('max_drawdown', 0)*100:.1f}%")
        
        with col5:
            st.metric("Sharpe Ratio", f"{metrics.get('sharpe_ratio', 0):.2f}")
        
        st.markdown("---")
        
        col1, col2 = st.columns([2, 1])
        
        with col1:
            st.subheader(f"{selected_asset}-PERP")
            
            fig = make_subplots(rows=2, cols=1, shared_xaxes=True, 
                               vertical_spacing=0.03, row_heights=[0.7, 0.3])
            
            fig.add_trace(go.Candlestick(
                x=df["timestamp"],
                open=df["open"],
                high=df["high"],
                low=df["low"],
                close=df["close"],
                name="OHLC",
                increasing_line_color='#2f9171',
                decreasing_line_color='#ef4444'
            ), row=1, col=1)
            
            colors = ['#2f9171' if c >= o else '#ef4444' 
                     for c, o in zip(df["close"], df["open"])]
            fig.add_trace(go.Bar(
                x=df["timestamp"],
                y=df["volume"],
                marker_color=colors,
                opacity=0.5,
                name="Volume"
            ), row=2, col=1)
            
            fig.update_layout(
                height=500,
                margin=dict(l=0, r=0, t=20, b=0),
                template="plotly_dark",
                paper_bgcolor='rgba(0,0,0,0)',
                plot_bgcolor='rgba(0,0,0,0)',
                showlegend=False,
                xaxis_rangeslider_visible=False
            )
            fig.update_xaxes(gridcolor='#1a1a24')
            fig.update_yaxes(gridcolor='#1a1a24')
            
            st.plotly_chart(fig, use_container_width=True, key="overview_candles")
        
        with col2:
            st.subheader("Statistics")
            
            stats = [
                ("Return (Period)", f"{metrics.get('total_return', 0)*100:+.2f}%"),
                ("Ann. Return", f"{metrics.get('annualized_return', 0)*100:+.2f}%"),
                ("Volatility", f"{metrics.get('volatility', 0)*100:.1f}%"),
                ("Sharpe Ratio", f"{metrics.get('sharpe_ratio', 0):.2f}"),
                ("Sortino Ratio", f"{metrics.get('sortino_ratio', 0):.2f}"),
                ("Max Drawdown", f"{metrics.get('max_drawdown', 0)*100:.1f}%"),
                ("Calmar Ratio", f"{metrics.get('calmar_ratio', 0):.2f}"),
                ("VaR (95%)", f"{metrics.get('var_95', 0)*100:.2f}%"),
                ("CVaR (95%)", f"{metrics.get('cvar_95', 0)*100:.2f}%"),
            ]
            
            for label, value in stats:
                col_a, col_b = st.columns([1, 1])
                with col_a:
                    st.caption(label)
                with col_b:
                    st.write(value)
    else:
        st.warning("Unable to fetch market data. Check your connection.")
        if "api_error" in st.session_state:
            st.caption(f"Error: {st.session_state['api_error']}")


def render_risk_analysis():
    """Render Risk Analysis page with comprehensive analytics."""
    st.title("Risk Analysis")
    st.caption("Comprehensive market risk analytics with distribution analysis, drawdowns, correlations, and risk decomposition")
    
    # Main content area
    col1, col2 = st.columns([1, 3])
    with col1:
        assets = fetch_market_meta()
        
        st.markdown("**Asset Selection**")
        selected_asset = st.selectbox("Primary Asset", assets, index=0, key="risk_asset")
        days = st.slider("Lookback (days)", 7, 180, 30)
        
        st.markdown("---")
        st.markdown("**Correlation Analysis**")
        compare_assets = st.multiselect(
            "Compare with", 
            [a for a in assets if a != selected_asset],
            default=[assets[1] if len(assets) > 1 else None][:3] if len(assets) > 1 else []
        )
    
    df = fetch_market_data(selected_asset, days)
    
    if df is not None and len(df) > 0:
        returns = df["returns"].dropna()
        metrics = calculate_risk_metrics(returns)
        risk_decomp = calculate_risk_decomposition(returns)
        
        st.markdown("---")
        
        # Key Metrics Row
        col1, col2, col3, col4, col5 = st.columns(5)
        
        with col1:
            sharpe = metrics.get('sharpe_ratio', 0)
            delta = "Good" if sharpe > 1 else ("Moderate" if sharpe > 0.5 else "Low")
            st.metric("Sharpe Ratio", f"{sharpe:.2f}", delta)
        
        with col2:
            st.metric("Sortino Ratio", f"{metrics.get('sortino_ratio', 0):.2f}")
        
        with col3:
            max_dd = metrics.get('max_drawdown', 0) * 100
            st.metric("Max Drawdown", f"{max_dd:.1f}%")
        
        with col4:
            st.metric("Calmar Ratio", f"{metrics.get('calmar_ratio', 0):.2f}")
        
        with col5:
            st.metric("Win Rate", f"{metrics.get('win_rate', 0) * 100:.1f}%")
        
        st.markdown("---")
        
        # Tabs for different analysis views
        tab1, tab2, tab3, tab4 = st.tabs(["Distribution", "Drawdown", "Correlation", "Risk Decomposition"])
        
        with tab1:
            col1, col2 = st.columns(2)
            
            with col1:
                st.subheader("Return Distribution")
                
                fig = go.Figure()
                fig.add_trace(go.Histogram(
                    x=returns * 100,
                    nbinsx=50,
                    marker_color='#2f9171',
                    opacity=0.7
                ))
                
                var_95 = metrics.get('var_95', 0) * 100
                var_99 = metrics.get('var_99', 0) * 100
                
                fig.add_vline(x=var_95, line_dash="dash", line_color="#f59e0b",
                             annotation_text=f"VaR 95%: {var_95:.1f}%",
                             annotation_position="top left")
                fig.add_vline(x=var_99, line_dash="dash", line_color="#ef4444",
                             annotation_text=f"VaR 99%: {var_99:.1f}%",
                             annotation_position="top left")
                
                fig.update_layout(
                    height=300,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Return (%)",
                    yaxis_title="Frequency",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="risk_hist")
            
            with col2:
                st.subheader("Distribution Statistics")
                
                dist_stats = {
                    "Metric": [
                        "Mean Return",
                        "Std Dev",
                        "Skewness",
                        "Kurtosis",
                        "VaR (95%)",
                        "VaR (99%)",
                        "CVaR (ES)",
                        "Win Rate"
                    ],
                    "Value": [
                        f"{float(np.mean(returns)) * 100:.3f}%",
                        f"{float(np.std(returns)) * 100:.3f}%",
                        f"{metrics.get('skewness', 0):.3f}",
                        f"{metrics.get('kurtosis', 0):.3f}",
                        f"{metrics.get('var_95', 0) * 100:.2f}%",
                        f"{metrics.get('var_99', 0) * 100:.2f}%",
                        f"{metrics.get('cvar_95', 0) * 100:.2f}%",
                        f"{metrics.get('win_rate', 0) * 100:.1f}%"
                    ],
                    "Interpretation": [
                        "Avg hourly return",
                        "Return volatility",
                        "< 0 = left tail" if metrics.get('skewness', 0) < 0 else "> 0 = right tail",
                        "High = fat tails" if metrics.get('kurtosis', 0) > 3 else "Normal tails",
                        "5% worst case",
                        "1% worst case",
                        "Avg beyond VaR",
                        "% positive returns"
                    ]
                }
                st.dataframe(pd.DataFrame(dist_stats), hide_index=True, height=320)
        
        with tab2:
            col1, col2 = st.columns(2)
            
            with col1:
                st.subheader("Drawdown Analysis")
                
                cumulative = (1 + returns).cumprod()
                rolling_max = cumulative.expanding().max()
                drawdown = (cumulative - rolling_max) / rolling_max * 100
                
                fig = go.Figure()
                fig.add_trace(go.Scatter(
                    x=df["timestamp"].iloc[1:],
                    y=drawdown,
                    fill='tozeroy',
                    fillcolor='rgba(239, 68, 68, 0.2)',
                    line=dict(color='#ef4444', width=1),
                    name='Drawdown'
                ))
                
                fig.update_layout(
                    height=300,
                    margin=dict(l=0, r=0, t=20, b=0),
                    yaxis_title="Drawdown (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="risk_drawdown")
            
            with col2:
                st.subheader("Drawdown Statistics")
                
                # Calculate drawdown periods
                dd_values = drawdown.values
                in_dd = dd_values < -0.01  # In drawdown if > 1%
                
                # Count consecutive drawdown periods
                dd_periods = []
                current_dd_len = 0
                current_dd_depth = 0
                for i, (is_dd, dd_val) in enumerate(zip(in_dd, dd_values)):
                    if is_dd:
                        current_dd_len += 1
                        current_dd_depth = min(current_dd_depth, dd_val)
                    elif current_dd_len > 0:
                        dd_periods.append({'length': current_dd_len, 'depth': current_dd_depth})
                        current_dd_len = 0
                        current_dd_depth = 0
                
                max_dd_len = max([p['length'] for p in dd_periods]) if dd_periods else 0
                avg_dd_len = np.mean([p['length'] for p in dd_periods]) if dd_periods else 0
                
                dd_stats = {
                    "Metric": [
                        "Max Drawdown",
                        "Current Drawdown",
                        "Avg Drawdown",
                        "# Drawdown Periods",
                        "Max DD Duration",
                        "Avg DD Duration",
                        "Time in Drawdown",
                        "Calmar Ratio"
                    ],
                    "Value": [
                        f"{float(drawdown.min()):.2f}%",
                        f"{float(drawdown.iloc[-1]):.2f}%",
                        f"{float(drawdown[drawdown < 0].mean()):.2f}%" if len(drawdown[drawdown < 0]) > 0 else "0%",
                        f"{len(dd_periods)}",
                        f"{max_dd_len} hours",
                        f"{avg_dd_len:.1f} hours",
                        f"{float(np.sum(in_dd) / len(in_dd) * 100):.1f}%",
                        f"{metrics.get('calmar_ratio', 0):.3f}"
                    ]
                }
                st.dataframe(pd.DataFrame(dd_stats), hide_index=True, height=320)
        
        with tab3:
            st.subheader("Correlation Analysis")
            
            if compare_assets:
                # Fetch data for comparison assets
                returns_dict = {selected_asset: returns.values}
                
                for asset in compare_assets:
                    asset_df = fetch_market_data(asset, days)
                    if asset_df is not None and len(asset_df) > 0:
                        asset_returns = asset_df["returns"].dropna().values
                        # Align lengths
                        min_len = min(len(returns_dict[selected_asset]), len(asset_returns))
                        returns_dict[selected_asset] = returns_dict[selected_asset][:min_len]
                        returns_dict[asset] = asset_returns[:min_len]
                
                if len(returns_dict) > 1:
                    corr_matrix = calculate_correlation_analysis(returns_dict)
                    
                    col1, col2 = st.columns(2)
                    
                    with col1:
                        st.markdown("**Correlation Matrix**")
                        
                        fig = go.Figure(data=go.Heatmap(
                            z=corr_matrix.values,
                            x=corr_matrix.columns,
                            y=corr_matrix.index,
                            colorscale='RdBu',
                            zmid=0,
                            text=np.round(corr_matrix.values, 3),
                            texttemplate='%{text}',
                            textfont={"size": 12},
                            hoverongaps=False
                        ))
                        
                        fig.update_layout(
                            height=350,
                            margin=dict(l=0, r=0, t=20, b=0),
                            template="plotly_dark",
                            paper_bgcolor='rgba(0,0,0,0)',
                            plot_bgcolor='rgba(0,0,0,0)',
                        )
                        
                        st.plotly_chart(fig, use_container_width=True, key="corr_heatmap")
                    
                    with col2:
                        st.markdown("**Rolling Correlation**")
                        
                        # Calculate rolling correlation with first comparison asset
                        if len(compare_assets) > 0:
                            primary = pd.Series(returns_dict[selected_asset])
                            secondary = pd.Series(returns_dict[compare_assets[0]])
                            rolling_corr = primary.rolling(24).corr(secondary)
                            
                            fig = go.Figure()
                            fig.add_trace(go.Scatter(
                                y=rolling_corr,
                                mode='lines',
                                line=dict(color='#6366f1', width=1.5),
                                name=f'{selected_asset} vs {compare_assets[0]}'
                            ))
                            fig.add_hline(y=0, line_dash="dash", line_color="#666")
                            fig.add_hline(y=0.7, line_dash="dot", line_color="#2f9171",
                                         annotation_text="High Corr", annotation_position="right")
                            fig.add_hline(y=-0.7, line_dash="dot", line_color="#ef4444",
                                         annotation_text="High Neg", annotation_position="right")
                            
                            fig.update_layout(
                                height=350,
                                margin=dict(l=0, r=0, t=20, b=0),
                                yaxis_title="Correlation",
                                template="plotly_dark",
                                paper_bgcolor='rgba(0,0,0,0)',
                                plot_bgcolor='rgba(0,0,0,0)',
                                showlegend=True
                            )
                            fig.update_xaxes(gridcolor='#1a1a24')
                            fig.update_yaxes(gridcolor='#1a1a24', range=[-1, 1])
                            
                            st.plotly_chart(fig, use_container_width=True, key="rolling_corr")
                    
                    # Scatter plot matrix
                    st.markdown("**Return Scatter Plots**")
                    
                    n_assets = len(compare_assets)
                    cols = st.columns(min(n_assets, 3))
                    
                    for i, asset in enumerate(compare_assets[:3]):
                        with cols[i]:
                            fig = go.Figure()
                            fig.add_trace(go.Scatter(
                                x=returns_dict[selected_asset] * 100,
                                y=returns_dict[asset] * 100,
                                mode='markers',
                                marker=dict(
                                    color='#2f9171',
                                    size=4,
                                    opacity=0.5
                                ),
                                name=f'{selected_asset} vs {asset}'
                            ))
                            
                            # Add regression line
                            z = np.polyfit(returns_dict[selected_asset], returns_dict[asset], 1)
                            p = np.poly1d(z)
                            x_line = np.linspace(min(returns_dict[selected_asset]), max(returns_dict[selected_asset]), 100)
                            fig.add_trace(go.Scatter(
                                x=x_line * 100,
                                y=p(x_line) * 100,
                                mode='lines',
                                line=dict(color='#f59e0b', width=2),
                                name='Regression'
                            ))
                            
                            corr_val = corr_matrix.loc[selected_asset, asset]
                            fig.update_layout(
                                height=250,
                                margin=dict(l=0, r=0, t=30, b=0),
                                title=f"{selected_asset} vs {asset} (ρ={corr_val:.3f})",
                                xaxis_title=f"{selected_asset} (%)",
                                yaxis_title=f"{asset} (%)",
                                template="plotly_dark",
                                paper_bgcolor='rgba(0,0,0,0)',
                                plot_bgcolor='rgba(0,0,0,0)',
                                showlegend=False
                            )
                            fig.update_xaxes(gridcolor='#1a1a24')
                            fig.update_yaxes(gridcolor='#1a1a24')
                            
                            st.plotly_chart(fig, use_container_width=True, key=f"scatter_{asset}")
            else:
                st.info("Select comparison assets in the sidebar to see correlation analysis")
        
        with tab4:
            st.subheader("Risk Decomposition")
            
            if risk_decomp:
                col1, col2 = st.columns(2)
                
                with col1:
                    st.markdown("**Variance Components**")
                    
                    # Bar chart of variance components
                    components = ['Upside\nVariance', 'Downside\nVariance', 'Tail\nRisk']
                    values = [
                        risk_decomp.get('upside_variance', 0) * 10000,
                        risk_decomp.get('downside_variance', 0) * 10000,
                        risk_decomp.get('tail_risk', 0) * 10000
                    ]
                    colors = ['#2f9171', '#ef4444', '#f59e0b']
                    
                    fig = go.Figure(data=[
                        go.Bar(
                            x=components,
                            y=values,
                            marker_color=colors,
                            text=[f'{v:.2f}' for v in values],
                            textposition='auto'
                        )
                    ])
                    
                    fig.update_layout(
                        height=300,
                        margin=dict(l=0, r=0, t=20, b=0),
                        yaxis_title="Variance (bps²)",
                        template="plotly_dark",
                        paper_bgcolor='rgba(0,0,0,0)',
                        plot_bgcolor='rgba(0,0,0,0)',
                        showlegend=False
                    )
                    fig.update_xaxes(gridcolor='#1a1a24')
                    fig.update_yaxes(gridcolor='#1a1a24')
                    
                    st.plotly_chart(fig, use_container_width=True, key="var_components")
                
                with col2:
                    st.markdown("**Risk Metrics**")
                    
                    risk_data = {
                        "Metric": [
                            "Total Variance",
                            "Upside Variance",
                            "Downside Variance",
                            "Tail Risk",
                            "Downside/Upside Ratio",
                            "Concentration Risk"
                        ],
                        "Value": [
                            f"{risk_decomp.get('total_variance', 0) * 10000:.4f} bps²",
                            f"{risk_decomp.get('upside_variance', 0) * 10000:.4f} bps²",
                            f"{risk_decomp.get('downside_variance', 0) * 10000:.4f} bps²",
                            f"{risk_decomp.get('tail_risk', 0) * 10000:.4f} bps²",
                            f"{risk_decomp.get('variance_ratio', 0):.3f}",
                            f"{risk_decomp.get('concentration_risk', 0) * 100:.1f}%"
                        ],
                        "Description": [
                            "Overall return variance",
                            "Variance of positive returns",
                            "Variance of negative returns",
                            "Variance beyond 2σ",
                            "> 1 = more downside risk",
                            "Risk from top 10% moves"
                        ]
                    }
                    st.dataframe(pd.DataFrame(risk_data), hide_index=True, height=260)
                    
                    # Risk assessment
                    ratio = risk_decomp.get('variance_ratio', 0)
                    if ratio > 1.5:
                        st.error("Significant downside skew detected")
                    elif ratio > 1.1:
                        st.warning("Moderate downside bias")
                    else:
                        st.success("Balanced risk profile")
        
        # Summary section
        st.markdown("---")
        st.subheader("Detailed Metrics")
        
        col1, col2, col3 = st.columns(3)
        
        with col1:
            st.markdown("**Value at Risk**")
            st.write(f"VaR (95%): {metrics.get('var_95', 0)*100:.2f}%")
            st.write(f"VaR (99%): {metrics.get('var_99', 0)*100:.2f}%")
            st.write(f"CVaR (ES): {metrics.get('cvar_95', 0)*100:.2f}%")
        
        with col2:
            st.markdown("**Returns**")
            st.write(f"Total: {metrics.get('total_return', 0)*100:.2f}%")
            st.write(f"Annualized: {metrics.get('annualized_return', 0)*100:.2f}%")
            st.write(f"Volatility: {metrics.get('volatility', 0)*100:.2f}%")
        
        with col3:
            st.markdown("**Ratios**")
            st.write(f"Sharpe: {metrics.get('sharpe_ratio', 0):.3f}")
            st.write(f"Sortino: {metrics.get('sortino_ratio', 0):.3f}")
            st.write(f"Calmar: {metrics.get('calmar_ratio', 0):.3f}")
    else:
        st.warning("Unable to fetch market data.")


def render_monte_carlo():
    """Render Monte Carlo Simulation page with comprehensive scenario analysis."""
    st.title("Monte Carlo Simulation")
    st.caption("Advanced Monte Carlo simulation for strategy backtesting across multiple market scenarios with configurable parameters")
    
    # Sidebar for Monte Carlo parameters
    with st.sidebar:
        st.markdown("---")
        st.markdown("**Simulation Configuration**")
        
        # Strategy selection
        mc_strategy = st.selectbox(
            "Strategy",
            list(MONTE_CARLO_STRATEGIES.keys()),
            index=0,
            key="mc_strategy"
        )
        
        strategy_config = MONTE_CARLO_STRATEGIES[mc_strategy]
        st.caption(strategy_config["description"])
        
        # Scenario selection
        st.markdown("**Market Scenario**")
        mc_scenario = st.selectbox(
            "Scenario",
            list(MONTE_CARLO_SCENARIOS.keys()),
            index=0,
            key="mc_scenario"
        )
        scenario_config = MONTE_CARLO_SCENARIOS[mc_scenario]
        st.caption(scenario_config["description"])
        
        # Simulation parameters
        st.markdown("**Simulation Settings**")
        n_simulations = st.slider("Simulations", 100, 5000, 1000, step=100, key="mc_n_sims")
        n_periods = st.slider("Periods (days)", 30, 504, 252, key="mc_n_periods")
        initial_capital = st.number_input("Initial Capital ($)", 10000, 10000000, 100000, step=10000, key="mc_capital")
        
        # Initialize strategy_params with defaults
        strategy_params = strategy_config["default_params"].copy()
        
        # Strategy parameters (expandable)
        with st.expander("Strategy Parameters", expanded=False):
            for param_name, default_val in strategy_config["default_params"].items():
                param_range = strategy_config["param_ranges"].get(param_name, (0, 1))
                
                # Format label
                label = param_name.replace("_", " ").title()
                
                if isinstance(default_val, int):
                    strategy_params[param_name] = st.slider(
                        label, 
                        int(param_range[0]), 
                        int(param_range[1]), 
                        int(default_val),
                        key=f"mc_param_{param_name}"
                    )
                else:
                    strategy_params[param_name] = st.slider(
                        label,
                        float(param_range[0]),
                        float(param_range[1]),
                        float(default_val),
                        format="%.3f",
                        key=f"mc_param_{param_name}"
                    )
        
        # Scenario adjustments (expandable)
        with st.expander("Scenario Adjustments", expanded=False):
            custom_vol_mult = st.slider(
                "Volatility Multiplier",
                0.1, 5.0,
                float(scenario_config.get("vol_multiplier", 1.0)),
                format="%.2f",
                key="mc_vol_mult"
            )
            custom_drift = st.slider(
                "Drift Adjustment",
                -0.002, 0.002,
                float(scenario_config.get("drift_adjustment", 0.0)),
                format="%.4f",
                key="mc_drift"
            )
            custom_jump_prob = st.slider(
                "Jump Probability",
                0.0, 0.1,
                float(scenario_config.get("jump_probability", 0.0)),
                format="%.3f",
                key="mc_jump_prob"
            )
            custom_jump_mag = st.slider(
                "Jump Magnitude",
                -0.15, 0.15,
                float(scenario_config.get("jump_magnitude", 0.0)),
                format="%.3f",
                key="mc_jump_mag"
            )
            
            # Update scenario with custom values
            scenario_config = scenario_config.copy()
            scenario_config["vol_multiplier"] = custom_vol_mult
            scenario_config["drift_adjustment"] = custom_drift
            scenario_config["jump_probability"] = custom_jump_prob
            scenario_config["jump_magnitude"] = custom_jump_mag
    
    # Main content
    assets = fetch_market_meta()
    
    col1, col2 = st.columns([1, 2])
    with col1:
        selected_asset = st.selectbox("Asset", assets, index=0, key="mc_asset")
    with col2:
        days = st.slider("Historical Lookback (days)", 30, 180, 90, key="mc_days")
    
    st.markdown("---")
    
    # Run simulation button - prominent and clearly visible
    run_mc = st.button("▶ Run Monte Carlo Simulation", type="primary", use_container_width=True, key="run_mc_btn")
    
    # Fetch historical data for calibration
    df = fetch_market_data(selected_asset, days)
    
    if df is not None and len(df) > 0:
        returns = df["returns"].dropna()
        
        # Show calibration info
        col_a, col_b, col_c = st.columns(3)
        with col_a:
            st.metric("Data Points", len(returns))
        with col_b:
            st.metric("Mean Return", f"{returns.mean()*100:.3f}%")
        with col_c:
            st.metric("Volatility", f"{returns.std()*100:.2f}%")
        
        if run_mc or st.session_state.get("mc_results") is not None:
            if run_mc:
                with st.spinner(f"Running {n_simulations:,} Monte Carlo simulations..."):
                    try:
                        mc_results = run_monte_carlo_simulation(
                            returns=np.array(returns.values),
                            strategy_type=mc_strategy,
                            params=strategy_params,
                            scenario=scenario_config,
                            n_simulations=n_simulations,
                            n_periods=n_periods,
                            initial_capital=initial_capital,
                        )
                        st.session_state["mc_results"] = mc_results
                        
                        # Show completion with statistics
                        non_zero = np.sum(np.abs(mc_results['total_returns']) > 0.001)
                        st.success(f"Simulation complete! Analyzed {n_simulations:,} scenarios over {n_periods} days. {non_zero}/{n_simulations} simulations had trading activity.")
                        
                    except Exception as e:
                        st.error(f"Simulation error: {str(e)}")
                        import traceback
                        st.code(traceback.format_exc())
                        return
            else:
                mc_results = st.session_state["mc_results"]
            
            st.markdown("---")
            
            # Key Metrics Row
            st.subheader("Simulation Results")
            st.caption(f"Strategy: {mc_strategy} | Scenario: {mc_scenario} | Asset: {selected_asset}")
            
            st.markdown("---")
            
            mc_col1, mc_col2, mc_col3, mc_col4, mc_col5, mc_col6 = st.columns(6)
            
            with mc_col1:
                mean_ret = mc_results["mean_return"] * 100
                delta_color = "normal" if mean_ret >= 0 else "inverse"
                st.metric("Mean Return", f"{mean_ret:.1f}%", delta_color=delta_color)
            
            with mc_col2:
                median_ret = mc_results["median_return"] * 100
                st.metric("Median Return", f"{median_ret:.1f}%")
            
            with mc_col3:
                mean_dd = mc_results["mean_max_drawdown"] * 100
                st.metric("Mean Max DD", f"{mean_dd:.1f}%")
            
            with mc_col4:
                prob_loss = mc_results["probability_of_loss"] * 100
                st.metric("P(Loss)", f"{prob_loss:.1f}%")
            
            with mc_col5:
                mean_sharpe = mc_results["mean_sharpe"]
                st.metric("Mean Sharpe", f"{mean_sharpe:.2f}")
            
            with mc_col6:
                win_rate = mc_results["mean_win_rate"] * 100
                st.metric("Win Rate", f"{win_rate:.1f}%")
            
            st.markdown("---")
            
            # Charts row
            st.subheader("Distribution Analysis")
            mc_chart_col1, mc_chart_col2 = st.columns(2)
            
            with mc_chart_col1:
                st.markdown("**Equity Curve Distribution**")
                
                equity_curves = mc_results["equity_curves"]
                
                fig = go.Figure()
                
                # Plot percentile bands
                periods = np.arange(equity_curves.shape[1])
                
                p5 = np.percentile(equity_curves, 5, axis=0)
                p25 = np.percentile(equity_curves, 25, axis=0)
                p50 = np.percentile(equity_curves, 50, axis=0)
                p75 = np.percentile(equity_curves, 75, axis=0)
                p95 = np.percentile(equity_curves, 95, axis=0)
                
                # 5-95% range
                fig.add_trace(go.Scatter(
                    x=periods, y=p95,
                    mode='lines',
                    line=dict(width=0),
                    showlegend=False,
                    hoverinfo='skip'
                ))
                fig.add_trace(go.Scatter(
                    x=periods, y=p5,
                    mode='lines',
                    line=dict(width=0),
                    fill='tonexty',
                    fillcolor='rgba(47, 145, 113, 0.1)',
                    name='5th-95th Percentile'
                ))
                
                # 25-75% range
                fig.add_trace(go.Scatter(
                    x=periods, y=p75,
                    mode='lines',
                    line=dict(width=0),
                    showlegend=False,
                    hoverinfo='skip'
                ))
                fig.add_trace(go.Scatter(
                    x=periods, y=p25,
                    mode='lines',
                    line=dict(width=0),
                    fill='tonexty',
                    fillcolor='rgba(47, 145, 113, 0.25)',
                    name='25th-75th Percentile'
                ))
                
                # Median line
                fig.add_trace(go.Scatter(
                    x=periods, y=p50,
                    mode='lines',
                    line=dict(color='#2f9171', width=2),
                    name='Median'
                ))
                
                # Initial capital reference
                fig.add_hline(
                    y=initial_capital, 
                    line_dash="dash", 
                    line_color="#666",
                    annotation_text="Initial Capital",
                    annotation_position="right"
                )
                
                fig.update_layout(
                    height=400,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Period (days)",
                    yaxis_title="Portfolio Value ($)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="left", x=0)
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_equity_curves")
            
            with mc_chart_col2:
                st.markdown("**Final Value Distribution**")
                
                final_values = mc_results["final_values"]
                
                fig = go.Figure()
                fig.add_trace(go.Histogram(
                    x=final_values,
                    nbinsx=50,
                    marker_color='#2f9171',
                    opacity=0.7
                ))
                
                # Add VaR lines
                var_5 = np.percentile(final_values, 5)
                var_1 = np.percentile(final_values, 1)
                
                fig.add_vline(x=var_5, line_dash="dash", line_color="#f59e0b",
                             annotation_text=f"VaR 95%: ${var_5:,.0f}",
                             annotation_position="top left")
                fig.add_vline(x=var_1, line_dash="dash", line_color="#ef4444",
                             annotation_text=f"VaR 99%: ${var_1:,.0f}",
                             annotation_position="top left")
                fig.add_vline(x=initial_capital, line_dash="solid", line_color="#666",
                             annotation_text="Initial",
                             annotation_position="top right")
                
                fig.update_layout(
                    height=400,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Final Portfolio Value ($)",
                    yaxis_title="Frequency",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_final_dist")
            
            # Second row of charts
            mc_chart_col3, mc_chart_col4 = st.columns(2)
            
            with mc_chart_col3:
                st.markdown("**Return Distribution**")
                
                total_returns = mc_results["total_returns"] * 100
                
                fig = go.Figure()
                fig.add_trace(go.Histogram(
                    x=total_returns,
                    nbinsx=50,
                    marker_color='#6366f1',
                    opacity=0.7
                ))
                
                # Add reference lines
                fig.add_vline(x=0, line_dash="solid", line_color="#666")
                fig.add_vline(x=np.mean(total_returns), line_dash="dash", line_color="#2f9171",
                             annotation_text=f"Mean: {np.mean(total_returns):.1f}%",
                             annotation_position="top right")
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Total Return (%)",
                    yaxis_title="Frequency",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_return_dist")
            
            with mc_chart_col4:
                st.markdown("**Max Drawdown Distribution**")
                
                max_dds = mc_results["max_drawdowns"] * 100
                
                fig = go.Figure()
                fig.add_trace(go.Histogram(
                    x=max_dds,
                    nbinsx=50,
                    marker_color='#ef4444',
                    opacity=0.7
                ))
                
                # Add percentile lines
                p95_dd = np.percentile(max_dds, 95)
                fig.add_vline(x=p95_dd, line_dash="dash", line_color="#f59e0b",
                             annotation_text=f"95th Pctl: {p95_dd:.1f}%",
                             annotation_position="top left")
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Max Drawdown (%)",
                    yaxis_title="Frequency",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_dd_dist")
            
            # Third row - Advanced analytics
            st.markdown("---")
            st.subheader("Advanced Analytics")
            
            mc_chart_col5, mc_chart_col6 = st.columns(2)
            
            with mc_chart_col5:
                st.markdown("**Risk-Reward Scatter**")
                
                total_rets = mc_results["total_returns"] * 100
                max_dds_scatter = mc_results["max_drawdowns"] * 100
                
                fig = go.Figure()
                
                # Color by Sharpe ratio
                sharpes = mc_results["sharpe_ratios"]
                
                fig.add_trace(go.Scatter(
                    x=max_dds_scatter,
                    y=total_rets,
                    mode='markers',
                    marker=dict(
                        size=6,
                        color=sharpes,
                        colorscale='RdYlGn',
                        showscale=True,
                        colorbar=dict(title="Sharpe"),
                        line=dict(width=0.5, color='#1a1a24')
                    ),
                    text=[f"Return: {r:.1f}%<br>DD: {d:.1f}%<br>Sharpe: {s:.2f}" 
                          for r, d, s in zip(total_rets, max_dds_scatter, sharpes)],
                    hovertemplate='%{text}<extra></extra>'
                ))
                
                # Add quadrant lines
                fig.add_hline(y=0, line_dash="solid", line_color="#666", line_width=1)
                fig.add_vline(x=np.median(max_dds_scatter), line_dash="dash", line_color="#666", line_width=1)
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Max Drawdown (%)",
                    yaxis_title="Total Return (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_risk_reward")
            
            with mc_chart_col6:
                st.markdown("**Sharpe Ratio Distribution**")
                
                sharpe_values = mc_results["sharpe_ratios"]
                
                fig = go.Figure()
                fig.add_trace(go.Histogram(
                    x=sharpe_values,
                    nbinsx=40,
                    marker_color='#10b981',
                    opacity=0.7
                ))
                
                # Add reference lines
                mean_sharpe_val = np.mean(sharpe_values)
                median_sharpe_val = np.median(sharpe_values)
                
                fig.add_vline(x=mean_sharpe_val, line_dash="dash", line_color="#2f9171",
                             annotation_text=f"Mean: {mean_sharpe_val:.2f}",
                             annotation_position="top right")
                fig.add_vline(x=1.0, line_dash="dot", line_color="#f59e0b",
                             annotation_text="Threshold: 1.0",
                             annotation_position="top left")
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Sharpe Ratio",
                    yaxis_title="Frequency",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_sharpe_dist")
            
            # Fourth row - Win rate and underwater
            mc_chart_col7, mc_chart_col8 = st.columns(2)
            
            with mc_chart_col7:
                st.markdown("**Win Rate vs Return**")
                
                win_rates_pct = mc_results["win_rates"] * 100
                
                fig = go.Figure()
                
                fig.add_trace(go.Scatter(
                    x=win_rates_pct,
                    y=total_rets,
                    mode='markers',
                    marker=dict(
                        size=6,
                        color=total_rets,
                        colorscale='RdYlGn',
                        showscale=True,
                        colorbar=dict(title="Return %"),
                        line=dict(width=0.5, color='#1a1a24')
                    ),
                    text=[f"Win Rate: {w:.1f}%<br>Return: {r:.1f}%" 
                          for w, r in zip(win_rates_pct, total_rets)],
                    hovertemplate='%{text}<extra></extra>'
                ))
                
                # Add reference line at 50% win rate
                fig.add_vline(x=50, line_dash="dot", line_color="#666", line_width=1)
                fig.add_hline(y=0, line_dash="solid", line_color="#666", line_width=1)
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Win Rate (%)",
                    yaxis_title="Total Return (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_winrate_scatter")
            
            with mc_chart_col8:
                st.markdown("**Underwater Plot (Median Simulation)**")
                
                # Calculate drawdown over time for median simulation
                median_idx = np.argsort(mc_results["total_returns"])[len(mc_results["total_returns"])//2]
                equity_curve = mc_results["equity_curves"][median_idx]
                
                running_max = np.maximum.accumulate(equity_curve)
                drawdown_series = (equity_curve - running_max) / running_max * 100
                
                fig = go.Figure()
                
                fig.add_trace(go.Scatter(
                    x=np.arange(len(drawdown_series)),
                    y=drawdown_series,
                    fill='tozeroy',
                    fillcolor='rgba(239, 68, 68, 0.3)',
                    line=dict(color='#ef4444', width=2),
                    name='Drawdown'
                ))
                
                fig.add_hline(y=0, line_dash="solid", line_color="#666", line_width=1)
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Period (days)",
                    yaxis_title="Drawdown (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_underwater")
            
            # Fifth row - Percentile analysis and rolling metrics
            st.markdown("---")
            st.subheader("Percentile Analysis")
            
            mc_chart_col9, mc_chart_col10 = st.columns(2)
            
            with mc_chart_col9:
                st.markdown("**Return Percentiles**")
                
                percentiles = [1, 5, 10, 25, 50, 75, 90, 95, 99]
                percentile_values = [np.percentile(total_rets, p) for p in percentiles]
                
                colors = ['#ef4444' if v < 0 else '#10b981' for v in percentile_values]
                
                fig = go.Figure()
                fig.add_trace(go.Bar(
                    x=[f"P{p}" for p in percentiles],
                    y=percentile_values,
                    marker_color=colors,
                    text=[f"{v:.1f}%" for v in percentile_values],
                    textposition='outside',
                    hovertemplate='%{x}: %{y:.2f}%<extra></extra>'
                ))
                
                fig.add_hline(y=0, line_dash="solid", line_color="#666", line_width=1)
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=40, b=0),
                    xaxis_title="Percentile",
                    yaxis_title="Return (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_percentiles")
            
            with mc_chart_col10:
                st.markdown("**Rolling 30-Day Returns (Best/Median/Worst)**")
                
                equity_curves = mc_results["equity_curves"]
                
                # Calculate rolling returns for best, median, worst scenarios
                best_idx = np.argmax(mc_results["total_returns"])
                worst_idx = np.argmin(mc_results["total_returns"])
                median_idx = np.argsort(mc_results["total_returns"])[len(mc_results["total_returns"])//2]
                
                window = min(30, n_periods // 4)
                
                def rolling_return(equity_curve, window):
                    returns = []
                    for i in range(window, len(equity_curve)):
                        ret = (equity_curve[i] / equity_curve[i-window] - 1) * 100
                        returns.append(ret)
                    return returns
                
                fig = go.Figure()
                
                x_vals = np.arange(window, equity_curves.shape[1])
                
                fig.add_trace(go.Scatter(
                    x=x_vals,
                    y=rolling_return(equity_curves[best_idx], window),
                    mode='lines',
                    line=dict(color='#10b981', width=2),
                    name='Best Case'
                ))
                
                fig.add_trace(go.Scatter(
                    x=x_vals,
                    y=rolling_return(equity_curves[median_idx], window),
                    mode='lines',
                    line=dict(color='#6366f1', width=2),
                    name='Median Case'
                ))
                
                fig.add_trace(go.Scatter(
                    x=x_vals,
                    y=rolling_return(equity_curves[worst_idx], window),
                    mode='lines',
                    line=dict(color='#ef4444', width=2),
                    name='Worst Case'
                ))
                
                fig.add_hline(y=0, line_dash="solid", line_color="#666", line_width=1)
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    xaxis_title="Period (days)",
                    yaxis_title=f"{window}-Day Return (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="left", x=0)
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, use_container_width=True, key="mc_rolling_returns")
            
            # Detailed statistics table
            st.markdown("---")
            st.subheader("Detailed Statistics")
            
            stats_col1, stats_col2, stats_col3 = st.columns(3)
            
            with stats_col1:
                st.markdown("**Performance Metrics**")
                perf_data = {
                    "Metric": [
                        "Mean Final Value",
                        "Median Final Value",
                        "Std Dev Final Value",
                        "Mean Return",
                        "Median Return",
                        "Std Dev Return",
                        "Mean Sharpe Ratio",
                        "Mean Win Rate",
                    ],
                    "Value": [
                        f"${mc_results['mean_final_value']:,.0f}",
                        f"${mc_results['median_final_value']:,.0f}",
                        f"${mc_results['std_final_value']:,.0f}",
                        f"{mc_results['mean_return']*100:.2f}%",
                        f"{mc_results['median_return']*100:.2f}%",
                        f"{mc_results['std_return']*100:.2f}%",
                        f"{mc_results['mean_sharpe']:.3f}",
                        f"{mc_results['mean_win_rate']*100:.1f}%",
                    ]
                }
                st.dataframe(pd.DataFrame(perf_data), hide_index=True, height=320)
            
            with stats_col2:
                st.markdown("**Risk Metrics**")
                risk_mc_data = {
                    "Metric": [
                        "Mean Max Drawdown",
                        "Median Max Drawdown",
                        "Worst Drawdown",
                        "VaR (95%)",
                        "VaR (99%)",
                        "CVaR (95%)",
                        "Probability of Loss",
                        "Probability of Ruin",
                    ],
                    "Value": [
                        f"{mc_results['mean_max_drawdown']*100:.2f}%",
                        f"{mc_results['median_max_drawdown']*100:.2f}%",
                        f"{mc_results['worst_drawdown']*100:.2f}%",
                        f"{mc_results['var_95']*100:.2f}%",
                        f"{mc_results['var_99']*100:.2f}%",
                        f"{mc_results['cvar_95']*100:.2f}%",
                        f"{mc_results['probability_of_loss']*100:.1f}%",
                        f"{mc_results['probability_of_ruin']*100:.2f}%",
                    ]
                }
                st.dataframe(pd.DataFrame(risk_mc_data), hide_index=True, height=320)
            
            with stats_col3:
                st.markdown("**Scenario Configuration**")
                scenario_data = {
                    "Parameter": [
                        "Strategy Type",
                        "Market Scenario",
                        "Simulations",
                        "Periods",
                        "Initial Capital",
                        "Volatility Multiplier",
                        "Drift Adjustment",
                        "Jump Probability",
                    ],
                    "Value": [
                        mc_strategy,
                        mc_scenario,
                        f"{n_simulations:,}",
                        f"{n_periods} days",
                        f"${initial_capital:,.0f}",
                        f"{scenario_config.get('vol_multiplier', 1.0):.2f}x",
                        f"{scenario_config.get('drift_adjustment', 0.0)*100:.3f}%",
                        f"{scenario_config.get('jump_probability', 0.0)*100:.1f}%",
                    ]
                }
                st.dataframe(pd.DataFrame(scenario_data), hide_index=True, height=320)
            
            # Risk assessment summary
            st.markdown("---")
            st.subheader("Risk Assessment")
            
            prob_loss = mc_results["probability_of_loss"]
            prob_ruin = mc_results["probability_of_ruin"]
            mean_dd = mc_results["mean_max_drawdown"]
            mean_sharpe = mc_results["mean_sharpe"]
            
            assess_col1, assess_col2, assess_col3 = st.columns(3)
            
            with assess_col1:
                if prob_loss < 0.3:
                    st.success(f"Low loss probability ({prob_loss*100:.1f}%)")
                elif prob_loss < 0.5:
                    st.warning(f"Moderate loss probability ({prob_loss*100:.1f}%)")
                else:
                    st.error(f"High loss probability ({prob_loss*100:.1f}%)")
            
            with assess_col2:
                if mean_dd < 0.15:
                    st.success(f"Acceptable drawdown profile ({mean_dd*100:.1f}%)")
                elif mean_dd < 0.25:
                    st.warning(f"Elevated drawdown risk ({mean_dd*100:.1f}%)")
                else:
                    st.error(f"Severe drawdown risk ({mean_dd*100:.1f}%)")
            
            with assess_col3:
                if mean_sharpe > 1.0:
                    st.success(f"Favorable risk-adjusted returns (Sharpe: {mean_sharpe:.2f})")
                elif mean_sharpe > 0.5:
                    st.warning(f"Moderate risk-adjusted returns (Sharpe: {mean_sharpe:.2f})")
                else:
                    st.error(f"Poor risk-adjusted returns (Sharpe: {mean_sharpe:.2f})")
        
        else:
            # Initial state - show instructions and available options
            st.info("Configure simulation parameters in the sidebar and click the 'Run Monte Carlo Simulation' button above to generate projections.")
            
            # Show strategy description
            st.markdown("---")
            st.subheader("Available Strategies")
            
            strat_cols = st.columns(3)
            for i, (strat_name, strat_config) in enumerate(MONTE_CARLO_STRATEGIES.items()):
                with strat_cols[i % 3]:
                    st.markdown(f"**{strat_name}**")
                    st.caption(strat_config["description"])
                    
                    # Show default parameters
                    with st.expander("Default Parameters"):
                        for param, value in strat_config["default_params"].items():
                            st.text(f"{param.replace('_', ' ').title()}: {value}")
            
            st.markdown("---")
            st.subheader("Available Scenarios")
            
            scen_cols = st.columns(4)
            for i, (scen_name, scen_config) in enumerate(MONTE_CARLO_SCENARIOS.items()):
                with scen_cols[i % 4]:
                    st.markdown(f"**{scen_name}**")
                    st.caption(scen_config["description"])
                    
                    # Show scenario parameters
                    with st.expander("Scenario Details"):
                        st.text(f"Vol Mult: {scen_config.get('vol_multiplier', 1.0):.2f}x")
                        st.text(f"Drift: {scen_config.get('drift_adjustment', 0.0)*100:.3f}%")
                        st.text(f"Jump Prob: {scen_config.get('jump_probability', 0.0)*100:.1f}%")
    else:
        st.error("Unable to fetch market data. Please check your connection and try again.")


def render_portfolio():
    """Render Portfolio page with all Hyperliquid markets."""
    st.title("Markets & Portfolio")
    
    st.subheader("Hyperliquid Perpetual Markets")
    
    # Fetch all markets with info
    markets = fetch_all_markets_with_info()
    
    if markets:
        # Filter and search
        col1, col2 = st.columns([2, 1])
        with col1:
            search = st.text_input("Search markets", placeholder="BTC, ETH, SOL...")
        with col2:
            leverage_filter = st.selectbox("Min Leverage", [0, 3, 5, 10, 20, 40], index=0)
        
        # Filter markets
        filtered_markets = markets
        if search:
            filtered_markets = [m for m in filtered_markets if search.upper() in m["name"].upper()]
        if leverage_filter > 0:
            filtered_markets = [m for m in filtered_markets if (m["max_leverage"] or 0) >= leverage_filter]
        
        st.caption(f"Showing {len(filtered_markets)} of {len(markets)} markets")
        
        # Display market grid
        st.markdown("---")
        
        # Show top markets with live prices
        st.markdown("**Top Markets by Leverage**")
        top_markets = sorted(filtered_markets, key=lambda x: x["max_leverage"] or 0, reverse=True)[:20]
        
        # Display in 5 columns
        cols = st.columns(5)
        for i, market in enumerate(top_markets):
            with cols[i % 5]:
                lev = market["max_leverage"] or 0
                lev_color = "🟢" if lev >= 20 else ("🟡" if lev >= 10 else "🔴")
                st.markdown(f"""
                **{market['name']}-PERP**  
                {lev_color} {lev}x Leverage  
                Decimals: {market['sz_decimals']}
                """)
        
        st.markdown("---")
        
        # All markets table
        st.markdown("**All Markets**")
        df = pd.DataFrame(filtered_markets)
        df = df.rename(columns={
            "name": "Symbol",
            "max_leverage": "Max Leverage",
            "sz_decimals": "Size Decimals"
        })
        df = df.sort_values("Max Leverage", ascending=False)
        st.dataframe(df, hide_index=True, height=400)
    else:
        # Fallback to basic display
        st.info("Unable to fetch complete market data. Showing popular markets.")
        assets = fetch_market_meta()
        
        if assets:
            cols = st.columns(4)
            for i, asset in enumerate(assets[:12]):
                with cols[i % 4]:
                    df = fetch_market_data(asset, 1)
                    if df is not None and len(df) > 0:
                        price = df["close"].iloc[-1]
                        change = ((df["close"].iloc[-1] / df["close"].iloc[0]) - 1) * 100
                        delta_color = "normal" if change >= 0 else "inverse"
                        st.metric(f"{asset}-PERP", f"${price:,.2f}", f"{change:+.2f}%", delta_color=delta_color)
                    else:
                        st.metric(f"{asset}-PERP", "...")


def render_backtest():
    """Render Backtest page with comprehensive analysis."""
    st.title("Backtest")
    
    strategies = find_strategies()
    
    col1, col2 = st.columns([2, 1])
    
    with col1:
        st.subheader("Strategy Selection")
        
        if strategies:
            strategy_options = [f"{s['name']} ({s['source']})" for s in strategies]
            selected_idx = st.selectbox("Strategy", range(len(strategy_options)), 
                                        format_func=lambda x: strategy_options[x])
            selected_strategy = strategies[selected_idx]
            
            with st.expander("View Strategy Code"):
                try:
                    with open(selected_strategy['path'], 'r') as f:
                        code = f.read()[:3000]
                    st.code(code, language='python')
                except:
                    st.write("Unable to load strategy file")
        else:
            st.warning("No strategies found. Add strategy files to examples/ or strategies/")
            selected_strategy = None
    
    with col2:
        st.subheader("Configuration")
        
        assets = fetch_market_meta()
        coin = st.selectbox("Instrument", assets, index=0)
        interval = st.selectbox("Timeframe", ["1h", "4h", "1d"], index=0)
        lookback_days = st.slider("Lookback (days)", 7, 90, 30)
        initial_capital = st.number_input("Initial Capital ($)", 1000, 1000000, 10000, step=1000)
    
    st.markdown("---")
    
    if st.button("Run Backtest", type="primary", disabled=selected_strategy is None):
        with st.spinner("Running backtest..."):
            result = run_backtest(
                selected_strategy['path'],
                coin,
                interval,
                lookback_days,
                initial_capital
            )
        
        if result.get('success'):
            st.success("Backtest completed successfully")
            
            # Key Metrics Row 1
            st.subheader("Performance Summary")
            col1, col2, col3, col4, col5 = st.columns(5)
            
            with col1:
                pnl = result.get('total_pnl', 0)
                st.metric(
                    "Total P&L", 
                    f"${pnl:,.2f}", 
                    f"{result.get('return_pct', 0):.2f}%",
                    delta_color="normal" if pnl >= 0 else "inverse"
                )
            
            with col2:
                st.metric(
                    "Final Balance", 
                    f"${result.get('final_balance', 0):,.2f}",
                    f"from ${result.get('initial_balance', 0):,.2f}"
                )
            
            with col3:
                st.metric("Max Drawdown", f"{result.get('max_drawdown_pct', 0):.2f}%")
            
            with col4:
                st.metric("Sharpe Ratio", f"{result.get('sharpe_ratio', 0):.2f}")
            
            with col5:
                st.metric("Win Rate", f"{result.get('win_rate', 0):.1f}%")
            
            # Key Metrics Row 2
            col1, col2, col3, col4, col5 = st.columns(5)
            
            with col1:
                st.metric("Total Trades", result.get('total_trades', 0))
            
            with col2:
                st.metric("Winning Trades", result.get('winning_trades', 0))
            
            with col3:
                st.metric("Losing Trades", result.get('losing_trades', 0))
            
            with col4:
                st.metric("Sortino Ratio", f"{result.get('sortino_ratio', 0):.2f}")
            
            with col5:
                st.metric("Calmar Ratio", f"{result.get('calmar_ratio', 0):.2f}")
            
            st.markdown("---")
            
            # Price Chart with Candlesticks
            candles = result.get('candles', [])
            if candles:
                st.subheader(f"{coin}-PERP Price Chart ({interval})")
                
                df = pd.DataFrame(candles)
                
                fig = make_subplots(
                    rows=2, cols=1, 
                    shared_xaxes=True, 
                    vertical_spacing=0.03, 
                    row_heights=[0.7, 0.3]
                )
                
                # Candlestick chart
                fig.add_trace(go.Candlestick(
                    x=df["timestamp"],
                    open=df["open"],
                    high=df["high"],
                    low=df["low"],
                    close=df["close"],
                    name="OHLC",
                    increasing_line_color='#2f9171',
                    decreasing_line_color='#ef4444'
                ), row=1, col=1)
                
                # Volume bars
                colors = ['#2f9171' if c >= o else '#ef4444' 
                         for c, o in zip(df["close"], df["open"])]
                fig.add_trace(go.Bar(
                    x=df["timestamp"],
                    y=df["volume"],
                    marker_color=colors,
                    opacity=0.5,
                    name="Volume"
                ), row=2, col=1)
                
                fig.update_layout(
                    height=500,
                    margin=dict(l=0, r=0, t=20, b=0),
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False,
                    xaxis_rangeslider_visible=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, key="backtest_candles")
            
            # Equity Curve Chart
            equity_curve = result.get('equity_curve', [])
            if equity_curve:
                st.markdown("---")
                st.subheader("Equity Curve")
                
                eq_times = [e[0] for e in equity_curve]
                eq_values = [e[1] for e in equity_curve]
                
                fig = go.Figure()
                fig.add_trace(go.Scatter(
                    x=eq_times,
                    y=eq_values,
                    mode='lines',
                    line=dict(color='#2f9171', width=2),
                    fill='tozeroy',
                    fillcolor='rgba(47, 145, 113, 0.2)',
                    name='Equity'
                ))
                
                # Add initial capital reference line
                initial = result.get('initial_balance', 10000)
                fig.add_hline(y=initial, line_dash="dash", line_color="#6366f1",
                             annotation_text=f"Initial: ${initial:,.0f}",
                             annotation_position="top left")
                
                fig.update_layout(
                    height=350,
                    margin=dict(l=0, r=0, t=20, b=0),
                    yaxis_title="Portfolio Value ($)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                
                st.plotly_chart(fig, key="equity_curve")
            
            st.markdown("---")
            
            # Risk Analysis Section
            st.subheader("Risk Analysis")
            
            col1, col2 = st.columns(2)
            
            with col1:
                # Detailed Risk Metrics
                st.markdown("**Risk Metrics**")
                risk_data = {
                    "Metric": [
                        "Max Drawdown",
                        "Sharpe Ratio",
                        "Sortino Ratio",
                        "Calmar Ratio",
                        "Profit Factor",
                        "Expectancy",
                        "Avg Trade P&L",
                        "Total Commission"
                    ],
                    "Value": [
                        f"{result.get('max_drawdown_pct', 0):.2f}%",
                        f"{result.get('sharpe_ratio', 0):.3f}",
                        f"{result.get('sortino_ratio', 0):.3f}",
                        f"{result.get('calmar_ratio', 0):.3f}",
                        f"{result.get('profit_factor', 0):.2f}",
                        f"${result.get('expectancy', 0):.2f}",
                        f"${result.get('avg_trade_pnl', 0):.2f}",
                        f"${result.get('total_commission', 0):.2f}"
                    ]
                }
                st.dataframe(pd.DataFrame(risk_data), hide_index=True)
            
            with col2:
                # Trade Statistics
                st.markdown("**Trade Statistics**")
                trade_data = {
                    "Metric": [
                        "Total Trades",
                        "Winning Trades",
                        "Losing Trades",
                        "Win Rate",
                        "Max Consecutive Wins",
                        "Max Consecutive Losses",
                        "Total Volume",
                        "Return %"
                    ],
                    "Value": [
                        f"{result.get('total_trades', 0)}",
                        f"{result.get('winning_trades', 0)}",
                        f"{result.get('losing_trades', 0)}",
                        f"{result.get('win_rate', 0):.1f}%",
                        f"{result.get('max_consecutive_wins', 0)}",
                        f"{result.get('max_consecutive_losses', 0)}",
                        f"${result.get('total_volume', 0):,.2f}",
                        f"{result.get('return_pct', 0):.2f}%"
                    ]
                }
                st.dataframe(pd.DataFrame(trade_data), hide_index=True)
            
            # Return Distribution if we have candle data
            if candles:
                st.markdown("---")
                st.subheader("Return Distribution & NAV Analysis")
                
                df = pd.DataFrame(candles)
                df["returns"] = df["close"].pct_change() * 100
                returns = df["returns"].dropna()
                
                col1, col2 = st.columns(2)
                
                with col1:
                    fig = go.Figure()
                    fig.add_trace(go.Histogram(
                        x=returns,
                        nbinsx=50,
                        marker_color='#2f9171',
                        opacity=0.7,
                        name='Returns'
                    ))
                    
                    # Add VaR lines
                    var_95 = np.percentile(returns, 5)
                    var_99 = np.percentile(returns, 1)
                    
                    fig.add_vline(x=var_95, line_dash="dash", line_color="#f59e0b",
                                 annotation_text=f"VaR 95%: {var_95:.2f}%",
                                 annotation_position="top left")
                    fig.add_vline(x=var_99, line_dash="dash", line_color="#ef4444",
                                 annotation_text=f"VaR 99%: {var_99:.2f}%",
                                 annotation_position="top left")
                    
                    fig.update_layout(
                        height=300,
                        margin=dict(l=0, r=0, t=20, b=0),
                        xaxis_title="Return (%)",
                        yaxis_title="Frequency",
                        template="plotly_dark",
                        paper_bgcolor='rgba(0,0,0,0)',
                        plot_bgcolor='rgba(0,0,0,0)',
                        showlegend=False
                    )
                    fig.update_xaxes(gridcolor='#1a1a24')
                    fig.update_yaxes(gridcolor='#1a1a24')
                    
                    st.plotly_chart(fig, key="return_dist")
                
                with col2:
                    # NAV Distribution
                    equity_curve = result.get('equity_curve', [])
                    if equity_curve:
                        nav_values = [e[1] for e in equity_curve]
                        
                        fig = go.Figure()
                        fig.add_trace(go.Histogram(
                            x=nav_values,
                            nbinsx=30,
                            marker_color='#6366f1',
                            opacity=0.7,
                            name='NAV'
                        ))
                        
                        # Add mean and percentile lines
                        nav_mean = np.mean(nav_values)
                        nav_p25 = np.percentile(nav_values, 25)
                        nav_p75 = np.percentile(nav_values, 75)
                        
                        fig.add_vline(x=nav_mean, line_dash="solid", line_color="#2f9171",
                                     annotation_text=f"Mean: ${nav_mean:,.0f}",
                                     annotation_position="top right")
                        
                        fig.update_layout(
                            height=300,
                            margin=dict(l=0, r=0, t=20, b=0),
                            xaxis_title="NAV ($)",
                            yaxis_title="Frequency",
                            template="plotly_dark",
                            paper_bgcolor='rgba(0,0,0,0)',
                            plot_bgcolor='rgba(0,0,0,0)',
                            showlegend=False
                        )
                        fig.update_xaxes(gridcolor='#1a1a24')
                        fig.update_yaxes(gridcolor='#1a1a24')
                        
                        st.plotly_chart(fig, key="nav_dist")
                    else:
                        # Fallback to cumulative returns
                        cumulative = (1 + returns / 100).cumprod() * 100 - 100
                        
                        fig = go.Figure()
                        fig.add_trace(go.Scatter(
                            x=df["timestamp"].iloc[1:],
                            y=cumulative,
                            mode='lines',
                            line=dict(color='#2f9171', width=2),
                            fill='tozeroy',
                            fillcolor='rgba(47, 145, 113, 0.1)',
                            name='Cumulative Return'
                        ))
                        
                        fig.update_layout(
                            height=300,
                            margin=dict(l=0, r=0, t=20, b=0),
                            yaxis_title="Cumulative Return (%)",
                            template="plotly_dark",
                            paper_bgcolor='rgba(0,0,0,0)',
                            plot_bgcolor='rgba(0,0,0,0)',
                            showlegend=False
                        )
                        fig.update_xaxes(gridcolor='#1a1a24')
                        fig.update_yaxes(gridcolor='#1a1a24')
                        
                        st.plotly_chart(fig, key="cumulative_returns")
            
            # Risk Decomposition Section
            st.markdown("---")
            st.subheader("Risk Decomposition")
            
            equity_curve = result.get('equity_curve', [])
            if equity_curve and len(equity_curve) > 10:
                eq_values = [e[1] for e in equity_curve]
                eq_returns = np.diff(eq_values) / eq_values[:-1]
                
                risk_decomp = calculate_risk_decomposition(eq_returns)
                nav_analysis = calculate_nav_distribution(equity_curve, result.get('initial_balance', 10000))
                
                col1, col2, col3 = st.columns(3)
                
                with col1:
                    st.markdown("**Variance Decomposition**")
                    if risk_decomp:
                        # Pie chart of risk components
                        labels = ['Upside Variance', 'Downside Variance', 'Tail Risk']
                        values = [
                            risk_decomp.get('upside_variance', 0) * 10000,
                            risk_decomp.get('downside_variance', 0) * 10000,
                            risk_decomp.get('tail_risk', 0) * 10000
                        ]
                        
                        fig = go.Figure(data=[go.Pie(
                            labels=labels,
                            values=values,
                            hole=0.4,
                            marker_colors=['#2f9171', '#ef4444', '#f59e0b']
                        )])
                        fig.update_layout(
                            height=250,
                            margin=dict(l=0, r=0, t=20, b=0),
                            template="plotly_dark",
                            paper_bgcolor='rgba(0,0,0,0)',
                            showlegend=True,
                            legend=dict(orientation="h", yanchor="bottom", y=-0.2)
                        )
                        st.plotly_chart(fig, key="risk_pie")
                        
                        st.caption(f"Downside/Upside Ratio: {risk_decomp.get('variance_ratio', 0):.2f}")
                
                with col2:
                    st.markdown("**NAV Statistics**")
                    if nav_analysis:
                        nav_stats = {
                            "Metric": ["Mean NAV", "Std Dev", "Min NAV", "Max NAV", "Range", "Time Underwater"],
                            "Value": [
                                f"${nav_analysis.get('nav_mean', 0):,.2f}",
                                f"${nav_analysis.get('nav_std', 0):,.2f}",
                                f"${nav_analysis.get('nav_min', 0):,.2f}",
                                f"${nav_analysis.get('nav_max', 0):,.2f}",
                                f"${nav_analysis.get('nav_range', 0):,.2f}",
                                f"{nav_analysis.get('time_underwater_pct', 0):.1f}%"
                            ]
                        }
                        st.dataframe(pd.DataFrame(nav_stats), hide_index=True, height=250)
                
                with col3:
                    st.markdown("**NAV Percentiles**")
                    if nav_analysis and nav_analysis.get('percentiles'):
                        percentiles = nav_analysis['percentiles']
                        pct_data = {
                            "Percentile": [k.replace('p', '') + '%' for k in percentiles.keys()],
                            "NAV": [f"${v:,.2f}" for v in percentiles.values()]
                        }
                        st.dataframe(pd.DataFrame(pct_data), hide_index=True, height=250)
            
            # Rolling Metrics Section
            if equity_curve and len(equity_curve) > 24:
                st.markdown("---")
                st.subheader("Rolling Performance Analysis")
                
                eq_times = [e[0] for e in equity_curve]
                eq_values = [e[1] for e in equity_curve]
                eq_returns = pd.Series(np.diff(eq_values) / eq_values[:-1])
                
                rolling = calculate_rolling_metrics(eq_returns, window=24)
                
                col1, col2 = st.columns(2)
                
                with col1:
                    fig = go.Figure()
                    fig.add_trace(go.Scatter(
                        x=eq_times[1:],
                        y=rolling['rolling_volatility'] * 100,
                        mode='lines',
                        line=dict(color='#f59e0b', width=1.5),
                        name='Rolling Volatility'
                    ))
                    fig.update_layout(
                        height=250,
                        margin=dict(l=0, r=0, t=30, b=0),
                        title="Rolling Volatility (24-period)",
                        yaxis_title="Volatility (%)",
                        template="plotly_dark",
                        paper_bgcolor='rgba(0,0,0,0)',
                        plot_bgcolor='rgba(0,0,0,0)',
                        showlegend=False
                    )
                    fig.update_xaxes(gridcolor='#1a1a24')
                    fig.update_yaxes(gridcolor='#1a1a24')
                    st.plotly_chart(fig, key="rolling_vol")
                
                with col2:
                    fig = go.Figure()
                    sharpe_vals = rolling['rolling_sharpe'].fillna(0)
                    colors = ['#2f9171' if v >= 0 else '#ef4444' for v in sharpe_vals]
                    fig.add_trace(go.Scatter(
                        x=eq_times[1:],
                        y=sharpe_vals,
                        mode='lines',
                        line=dict(color='#6366f1', width=1.5),
                        name='Rolling Sharpe'
                    ))
                    fig.add_hline(y=0, line_dash="dash", line_color="#666")
                    fig.add_hline(y=1, line_dash="dot", line_color="#2f9171", 
                                 annotation_text="Good (1.0)", annotation_position="right")
                    fig.update_layout(
                        height=250,
                        margin=dict(l=0, r=0, t=30, b=0),
                        title="Rolling Sharpe Ratio (24-period)",
                        yaxis_title="Sharpe Ratio",
                        template="plotly_dark",
                        paper_bgcolor='rgba(0,0,0,0)',
                        plot_bgcolor='rgba(0,0,0,0)',
                        showlegend=False
                    )
                    fig.update_xaxes(gridcolor='#1a1a24')
                    fig.update_yaxes(gridcolor='#1a1a24')
                    st.plotly_chart(fig, key="rolling_sharpe")
                
                # Drawdown chart
                st.markdown("**Underwater Analysis**")
                fig = go.Figure()
                fig.add_trace(go.Scatter(
                    x=eq_times[1:],
                    y=rolling['rolling_drawdown'] * 100,
                    fill='tozeroy',
                    fillcolor='rgba(239, 68, 68, 0.3)',
                    line=dict(color='#ef4444', width=1),
                    name='Drawdown'
                ))
                fig.update_layout(
                    height=200,
                    margin=dict(l=0, r=0, t=20, b=0),
                    yaxis_title="Drawdown (%)",
                    template="plotly_dark",
                    paper_bgcolor='rgba(0,0,0,0)',
                    plot_bgcolor='rgba(0,0,0,0)',
                    showlegend=False
                )
                fig.update_xaxes(gridcolor='#1a1a24')
                fig.update_yaxes(gridcolor='#1a1a24')
                st.plotly_chart(fig, key="underwater")
            
            # Trade List
            fills = result.get('fills', [])
            if fills:
                st.markdown("---")
                st.subheader("Trade History")
                
                trade_list = []
                for fill in fills[:50]:  # Show last 50 trades
                    # Handle different field naming conventions
                    timestamp = fill.get('timestamp', fill.get('time', ''))
                    if isinstance(timestamp, (int, float)):
                        try:
                            timestamp = datetime.fromtimestamp(timestamp / 1000 if timestamp > 1e10 else timestamp)
                        except:
                            pass
                    
                    side = str(fill.get('side', 'unknown')).upper()
                    if 'Buy' in side or 'LONG' in side:
                        side = '🟢 LONG'
                    elif 'Sell' in side or 'SHORT' in side:
                        side = '🔴 SHORT'
                    
                    size = fill.get('quantity', fill.get('size', fill.get('qty', 0)))
                    price = fill.get('price', 0)
                    pnl = fill.get('realized_pnl', fill.get('pnl', 0))
                    fee = fill.get('commission', fill.get('fee', 0))
                    
                    trade_list.append({
                        "Time": str(timestamp)[:19] if timestamp else "-",
                        "Side": side,
                        "Size": f"{float(size):.4f}" if size else "-",
                        "Price": f"${float(price):,.2f}" if price else "-",
                        "P&L": f"${float(pnl):+.2f}" if pnl else "-",
                        "Fee": f"${float(fee):.4f}" if fee else "-"
                    })
                
                if trade_list:
                    st.dataframe(pd.DataFrame(trade_list), hide_index=True, height=300)
                    if len(fills) > 50:
                        st.caption(f"Showing 50 of {len(fills)} trades")
        
        else:
            st.error(f"Backtest failed: {result.get('error', 'Unknown error')}")
            if result.get('traceback'):
                with st.expander("Error Details"):
                    st.code(result.get('traceback'), language='python')


def render_coming_soon(title: str, description: str, features: list):
    """Render a Coming Soon page."""
    st.title(title)
    
    st.markdown(f"""
    <div class="coming-soon-banner">
        <h2>Coming Soon</h2>
        <p>{description}</p>
    </div>
    """, unsafe_allow_html=True)
    
    st.subheader("Planned Features")
    for feature in features:
        st.write(f"• {feature}")


def render_live_trading():
    """Render Live Trading page."""
    render_coming_soon(
        "Live Trading",
        "Real-time execution on supported venues with advanced risk controls.",
        [
            "Multi-venue connectivity (Hyperliquid, Lighter, Polymarket)",
            "Real-time order execution with sub-millisecond latency",
            "Advanced risk management and circuit breakers",
            "Live P&L tracking and position monitoring",
            "Automated alerts and notifications"
        ]
    )


def render_deployment():
    """Render Deployment page."""
    render_coming_soon(
        "Agent Deployment",
        "CI/CD pipeline for trading strategies with automated validation.",
        [
            "Automated strategy deployment pipeline",
            "Pre-deployment backtesting and validation",
            "Secure credential management",
            "Version control and rollback support",
            "Deployment health monitoring"
        ]
    )


# =============================================================================
# Main Application
# =============================================================================

page = render_sidebar()

if page == "Overview":
    render_overview()
elif page == "Risk Analysis":
    render_risk_analysis()
elif page == "Monte Carlo":
    render_monte_carlo()
elif page == "Portfolio":
    render_portfolio()
elif page == "Backtest":
    render_backtest()
elif page == "Live Trading":
    render_live_trading()
elif page == "Deployment":
    render_deployment()

st.markdown("---")
st.caption(f"Neleus v0.1.0 • {datetime.now().strftime('%Y-%m-%d %H:%M')}")
