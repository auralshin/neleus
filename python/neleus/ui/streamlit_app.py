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
            ["Overview", "Risk Analysis", "Portfolio", "Backtest", "Live Trading", "Deployment"],
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
    
    col1, col2 = st.columns([1, 3])
    with col1:
        assets = fetch_market_meta()
        
        # Multi-asset selection for correlation
        st.markdown("**Single Asset Analysis**")
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
        tab1, tab2, tab3, tab4 = st.tabs(["📊 Distribution", "📉 Drawdown", "🔗 Correlation", "⚖️ Risk Decomposition"])
        
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
                        st.error("⚠️ Significant downside skew detected")
                    elif ratio > 1.1:
                        st.warning("⚡ Moderate downside bias")
                    else:
                        st.success("✅ Balanced risk profile")
        
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
