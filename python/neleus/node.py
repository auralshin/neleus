"""
Neleus Trading Node - Execution layer for strategies.

Provides BacktestNode, PaperNode, and LiveNode for different execution modes.
Uses types from the unified types module - no duplicate definitions.
"""
from __future__ import annotations

import json
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from decimal import Decimal
from enum import Enum
from pathlib import Path
from typing import Any, Dict, Generic, List, Optional, TYPE_CHECKING, TypeVar, Union

from .constants import (
    DEFAULT_LOOKBACK_DAYS,
    ANNUALIZATION_FACTOR,
)

logger = logging.getLogger(__name__)

# Import all types from the Rust bridge
from .types import (
    Network,
    FillModel,
    LatencyModel,
    OrderSide,
    Bar,
    StrategyContext,
    InstrumentId,
    Venue,
    InstrumentType,
)

if TYPE_CHECKING:
    from .strategy import Strategy


NodeResultT = TypeVar("NodeResultT", covariant=True)


# =============================================================================
# Node-Specific Configuration (extends base types)
# =============================================================================


@dataclass
class HyperliquidVenueConfig:
    """Hyperliquid venue configuration for live/paper trading."""
    network: Network = field(default_factory=lambda: Network.Mainnet)
    wallet_address: Optional[str] = None
    api_key: Optional[str] = None
    api_secret: Optional[str] = None
    vault_address: Optional[str] = None
    
    # Connection settings
    ws_url: Optional[str] = None
    rest_url: Optional[str] = None
    
    # Rate limits
    max_orders_per_second: int = 10
    max_open_orders: int = 1000
    
    def __post_init__(self):
        if self.ws_url is None:
            self.ws_url = (
                "wss://api.hyperliquid.xyz/ws"
                if self.network == Network.Mainnet
                else "wss://api.hyperliquid-testnet.xyz/ws"
            )
        if self.rest_url is None:
            self.rest_url = (
                "https://api.hyperliquid.xyz"
                if self.network == Network.Mainnet
                else "https://api.hyperliquid-testnet.xyz"
            )


@dataclass
class PolymarketVenueConfig:
    """Polymarket venue configuration for live/paper trading."""
    network: Network = field(default_factory=lambda: Network.Mainnet)
    wallet_address: Optional[str] = None
    private_key: Optional[str] = None
    funder_address: Optional[str] = None
    
    # API keys (L2 auth)
    api_key: Optional[str] = None
    api_secret: Optional[str] = None
    api_passphrase: Optional[str] = None
    
    # Connection settings
    clob_url: Optional[str] = None
    gamma_url: Optional[str] = None
    ws_url: Optional[str] = None
    
    def __post_init__(self):
        if self.clob_url is None:
            self.clob_url = (
                "https://clob.polymarket.com"
                if self.network == Network.Mainnet
                else "https://clob-staging.polymarket.com"
            )
        if self.gamma_url is None:
            self.gamma_url = (
                "https://gamma-api.polymarket.com"
                if self.network == Network.Mainnet
                else "https://gamma-api-staging.polymarket.com"
            )
        if self.ws_url is None:
            self.ws_url = (
                "wss://ws-subscriptions-clob.polymarket.com/ws/"
                if self.network == Network.Mainnet
                else "wss://ws-subscriptions-clob-staging.polymarket.com/ws/"
            )


VenueConfig = Union[HyperliquidVenueConfig, PolymarketVenueConfig]


@dataclass
class SimulationConfig:
    """Configuration for paper trading and backtest simulation."""
    
    # Fill simulation
    fill_model: FillModel = field(default_factory=lambda: FillModel.Immediate)
    fill_probability: float = 1.0
    slippage_bps: float = 0.0
    
    # Latency simulation
    latency_model: LatencyModel = field(default_factory=lambda: LatencyModel.Zero)
    latency_mean_ms: float = 0.0
    latency_std_ms: float = 0.0
    
    # Fee structure
    maker_fee_bps: float = 0.0
    taker_fee_bps: float = 5.0
    
    # Capital settings
    initial_capital: Decimal = Decimal("10000")
    margin_mode: str = "cross"
    default_leverage: int = 1


@dataclass
class BacktestConfig:
    """Configuration for backtest execution."""
    
    # Time range
    start_time: datetime
    end_time: datetime
    
    # Simulation settings
    simulation: SimulationConfig = field(default_factory=SimulationConfig)
    
    # Data source
    data_path: Optional[Path] = None
    data_format: str = "parquet"
    
    # Output settings
    output_path: Optional[Path] = None
    save_orders: bool = True
    save_positions: bool = True
    save_equity_curve: bool = True
    
    # Performance
    cache_data: bool = True
    parallel_feeds: int = 1


# =============================================================================
# Backtest Results
# =============================================================================


@dataclass
class BacktestMetrics:
    """Performance metrics from a backtest run."""
    
    # Returns
    total_return: float
    annualized_return: float
    max_drawdown: float
    calmar_ratio: float
    
    # Risk
    volatility: float
    sharpe_ratio: float
    sortino_ratio: float
    
    # Trading
    total_trades: int
    winning_trades: int
    losing_trades: int
    win_rate: float
    avg_trade_pnl: float
    avg_win: float
    avg_loss: float
    profit_factor: float
    
    # Time
    start_time: datetime
    end_time: datetime
    trading_days: int
    
    # Costs
    total_commission: float


@dataclass
class BacktestResults:
    """Complete results from a backtest run."""
    
    # Metrics
    metrics: BacktestMetrics
    
    # Time series
    equity_curve: List[tuple[datetime, float]]
    drawdown_curve: List[tuple[datetime, float]]
    
    # Trade history
    orders: List[Dict[str, Any]]
    fills: List[Dict[str, Any]]
    
    # State
    positions: List[Dict[str, Any]]
    
    # Reference config
    config: BacktestConfig
    
    def summary(self) -> str:
        """Generate human-readable summary."""
        m = self.metrics
        return f"""
Backtest Results
================
Period: {m.start_time.date()} to {m.end_time.date()} ({m.trading_days} days)

Performance:
  Total Return:      {m.total_return:>10.2%}
  Annualized Return: {m.annualized_return:>10.2%}
  Max Drawdown:      {m.max_drawdown:>10.2%}
  Sharpe Ratio:      {m.sharpe_ratio:>10.2f}
  Sortino Ratio:     {m.sortino_ratio:>10.2f}
  Calmar Ratio:      {m.calmar_ratio:>10.2f}

Trading:
  Total Trades:      {m.total_trades:>10d}
  Win Rate:          {m.win_rate:>10.2%}
  Profit Factor:     {m.profit_factor:>10.2f}
  Avg Trade PnL:     ${m.avg_trade_pnl:>9.2f}

Costs:
  Total Commission:  ${m.total_commission:>9.2f}
"""

    def to_dict(self) -> Dict[str, Any]:
        """Convert results to dictionary for serialization."""
        return {
            "metrics": {
                "total_return": self.metrics.total_return,
                "annualized_return": self.metrics.annualized_return,
                "max_drawdown": self.metrics.max_drawdown,
                "sharpe_ratio": self.metrics.sharpe_ratio,
                "sortino_ratio": self.metrics.sortino_ratio,
                "calmar_ratio": self.metrics.calmar_ratio,
                "total_trades": self.metrics.total_trades,
                "win_rate": self.metrics.win_rate,
                "profit_factor": self.metrics.profit_factor,
                "total_commission": self.metrics.total_commission,
            },
            "equity_curve": [
                {"timestamp": ts.isoformat(), "equity": eq}
                for ts, eq in self.equity_curve
            ],
            "orders_count": len(self.orders),
            "fills_count": len(self.fills),
        }

    def to_json(self, path: Path) -> None:
        """Save results to JSON file."""
        with open(path, "w") as f:
            json.dump(self.to_dict(), f, indent=2)


# =============================================================================
# Base Node
# =============================================================================


class Node(ABC, Generic[NodeResultT]):
    """
    Abstract base class for trading nodes.
    
    Nodes manage strategy execution in different modes:
    - BacktestNode: Historical simulation
    - PaperNode: Real-time simulation with live data
    - LiveNode: Real trading with exchange connectivity
    """

    def __init__(self, node_id: Optional[str] = None):
        self.node_id = node_id or self.__class__.__name__
        self._strategies: List["Strategy"] = []
        self._is_running = False

    @property
    def is_running(self) -> bool:
        return self._is_running

    def add_strategy(self, strategy: "Strategy") -> "Node[NodeResultT]":
        """Add a strategy to this node."""
        self._strategies.append(strategy)
        return self

    def add_strategies(self, *strategies: "Strategy") -> "Node[NodeResultT]":
        """Add multiple strategies to this node."""
        for strategy in strategies:
            self.add_strategy(strategy)
        return self

    @abstractmethod
    def run(self) -> NodeResultT:
        """Run the node synchronously."""
        pass

    @abstractmethod
    async def run_async(self) -> NodeResultT:
        """Run the node asynchronously."""
        pass

    def stop(self) -> None:
        """Stop the node."""
        self._is_running = False


# =============================================================================
# Backtest Node
# =============================================================================


class BacktestNode(Node[BacktestResults]):
    """
    Node for backtesting strategies on historical data.
    
    Uses historical market data to simulate strategy execution
    with configurable fill models, latency, and fees.
    
    Example:
        config = BacktestConfig(
            start_time=datetime(2024, 1, 1),
            end_time=datetime(2024, 6, 1),
        )
        node = BacktestNode(config)
        node.add_strategy(MyStrategy())
        results = node.run()
        print(results.summary())
    """

    def __init__(
        self,
        config: BacktestConfig,
        node_id: Optional[str] = None,
    ):
        super().__init__(node_id)
        self.config = config
        self._results: Optional[BacktestResults] = None

    @property
    def results(self) -> Optional[BacktestResults]:
        return self._results

    def run(self) -> BacktestResults:
        """Run backtest and return results."""
        self._is_running = True
        
        try:
            self._initialize_engine()
            self._run_event_loop()
            self._results = self._calculate_results()
            return self._results
        finally:
            self._is_running = False

    async def run_async(self) -> BacktestResults:
        """Async wrapper for backtest (runs synchronously internally)."""
        return self.run()

    def _initialize_engine(self) -> None:
        """Initialize backtest engine state."""
        pass

    def _run_event_loop(self) -> None:
        """Process historical events."""
        pass

    def _calculate_results(self) -> BacktestResults:
        """Calculate backtest metrics and results."""
        trading_days = (self.config.end_time - self.config.start_time).days
        
        metrics = BacktestMetrics(
            total_return=0.0,
            annualized_return=0.0,
            max_drawdown=0.0,
            calmar_ratio=0.0,
            volatility=0.0,
            sharpe_ratio=0.0,
            sortino_ratio=0.0,
            total_trades=0,
            winning_trades=0,
            losing_trades=0,
            win_rate=0.0,
            avg_trade_pnl=0.0,
            avg_win=0.0,
            avg_loss=0.0,
            profit_factor=0.0,
            start_time=self.config.start_time,
            end_time=self.config.end_time,
            trading_days=trading_days,
            total_commission=0.0,
        )
        
        return BacktestResults(
            metrics=metrics,
            equity_curve=[],
            drawdown_curve=[],
            orders=[],
            fills=[],
            positions=[],
            config=self.config,
        )


# =============================================================================
# Paper Trading Node
# =============================================================================


class PaperNode(Node[None]):
    """
    Node for paper trading with live market data.
    
    Connects to real exchanges for market data but simulates
    order execution locally without actual trading.
    
    Example:
        venues = [HyperliquidVenueConfig(network=Network.Mainnet)]
        node = PaperNode(venues)
        node.add_strategy(MyStrategy())
        node.run()
    """

    def __init__(
        self,
        venues: List[VenueConfig],
        simulation: Optional[SimulationConfig] = None,
        node_id: Optional[str] = None,
    ):
        super().__init__(node_id)
        self.venues = venues
        self.simulation = simulation or SimulationConfig()

    def run(self) -> None:
        """Run paper trading node synchronously."""
        import asyncio
        asyncio.run(self.run_async())

    async def run_async(self) -> None:
        """Run paper trading node asynchronously."""
        self._is_running = True
        
        try:
            await self._connect_venues()
            self._start_strategies()
            await self._run_event_loop()
        finally:
            await self._disconnect_venues()
            self._is_running = False

    async def _connect_venues(self) -> None:
        """Connect to venue WebSocket feeds."""
        pass

    async def _disconnect_venues(self) -> None:
        """Disconnect from venues."""
        pass

    def _start_strategies(self) -> None:
        """Initialize and start strategies."""
        pass

    async def _run_event_loop(self) -> None:
        """Process live market events."""
        pass


# =============================================================================
# Live Trading Node
# =============================================================================


class LiveNode(Node[None]):
    """
    Node for live trading with real order execution.
    
    Connects to exchanges and executes real trades.
    Use with caution - real money is at risk.
    
    Example:
        venues = [
            HyperliquidVenueConfig(
                network=Network.Mainnet,
                wallet_address="0x...",
            )
        ]
        node = LiveNode(venues)
        node.add_strategy(MyStrategy())
        node.run()
    """

    def __init__(
        self,
        venues: List[VenueConfig],
        node_id: Optional[str] = None,
        dry_run: bool = False,
    ):
        super().__init__(node_id)
        self.venues = venues
        self.dry_run = dry_run

    def run(self) -> None:
        """Run live trading node synchronously."""
        import asyncio
        asyncio.run(self.run_async())

    async def run_async(self) -> None:
        """Run live trading node asynchronously."""
        self._validate_config()
        self._is_running = True
        
        try:
            await self._connect_venues()
            await self._sync_positions()
            self._start_strategies()
            await self._run_event_loop()
        finally:
            await self._safe_shutdown()
            self._is_running = False

    def _validate_config(self) -> None:
        """Validate venue configurations for live trading."""
        for venue in self.venues:
            if not venue.wallet_address:
                raise ValueError("Hyperliquid requires wallet_address for live trading")

    async def _connect_venues(self) -> None:
        """Connect to venues."""
        pass

    async def _sync_positions(self) -> None:
        """Sync existing positions from venues."""
        pass

    def _start_strategies(self) -> None:
        """Initialize and start strategies."""
        pass

    async def _run_event_loop(self) -> None:
        """Process live market events."""
        pass

    async def _safe_shutdown(self) -> None:
        """Safely shutdown: cancel open orders, close positions if configured."""
        pass


# =============================================================================
# Convenience Functions
# =============================================================================


def backtest(
    strategy: "Strategy",
    start_time: datetime,
    end_time: datetime,
    **kwargs,
) -> BacktestResults:
    """
    Convenience function to run a simple backtest.
    
    Args:
        strategy: Strategy instance to backtest
        start_time: Backtest start time
        end_time: Backtest end time
        **kwargs: Additional config options (initial_capital, maker_fee_bps, etc.)
    
    Returns:
        BacktestResults with metrics and trade history
    """
    simulation = SimulationConfig(
        initial_capital=Decimal(str(kwargs.get("initial_capital", 10000))),
        maker_fee_bps=kwargs.get("maker_fee_bps", 0.0),
        taker_fee_bps=kwargs.get("taker_fee_bps", 5.0),
    )
    
    config = BacktestConfig(
        start_time=start_time,
        end_time=end_time,
        simulation=simulation,
    )
    
    node = BacktestNode(config)
    node.add_strategy(strategy)
    return node.run()


# =============================================================================
# Hyperliquid Backtest Node (with live data fetching)
# =============================================================================


class CandleInterval(Enum):
    """Candle interval for historical data."""
    MIN_1 = "1m"
    MIN_5 = "5m"
    MIN_15 = "15m"
    HOUR_1 = "1h"
    HOUR_4 = "4h"
    DAY_1 = "1d"


@dataclass
class HyperliquidBacktestConfig:
    """Configuration for backtesting with Hyperliquid historical data."""
    
    # Instrument
    coin: str = "BTC"
    
    # Data settings
    interval: CandleInterval = CandleInterval.HOUR_1
    
    # Time range
    start_time: Optional[datetime] = None
    end_time: Optional[datetime] = None
    
    # Alternative: lookback
    lookback_days: int = DEFAULT_LOOKBACK_DAYS
    
    # Network (testnet for testing)
    testnet: bool = False
    
    # Trading simulation
    initial_capital: Decimal = Decimal("10000")
    maker_fee_bps: float = 0.0
    taker_fee_bps: float = 4.0
    
    # Slippage
    slippage_bps: float = 5.0
    
    def __post_init__(self):
        if self.start_time is None or self.end_time is None:
            self.end_time = datetime.now()
            self.start_time = self.end_time - timedelta(days=self.lookback_days)


class HyperliquidBacktestNode(Node[BacktestResults]):
    """
    Backtest node that fetches historical data from Hyperliquid.
    
    This is a convenience wrapper that fetches data from Hyperliquid's API
    and runs it through the Rust backtest engine.
    
    Example:
        config = HyperliquidBacktestConfig(
            coin="ETH",
            interval=CandleInterval.HOUR_1,
            lookback_days=60,
        )
        node = HyperliquidBacktestNode(config)
        node.add_strategy(MomentumStrategy())
        results = node.run()
    """

    def __init__(
        self,
        config: HyperliquidBacktestConfig,
        node_id: Optional[str] = None,
    ):
        super().__init__(node_id)
        self.config = config
        self._results: Optional[BacktestResults] = None
        
    @property
    def results(self) -> Optional[BacktestResults]:
        return self._results
        
    async def _fetch_hyperliquid_candles(self) -> List[Dict[str, Any]]:
        """Fetch historical candles from Hyperliquid API."""
        import aiohttp

        start_time = self.config.start_time
        end_time = self.config.end_time
        if start_time is None or end_time is None:
            raise RuntimeError("HyperliquidBacktestConfig must have start_time and end_time set")
        
        base_url = (
            "https://api.hyperliquid-testnet.xyz"
            if self.config.testnet
            else "https://api.hyperliquid.xyz"
        )
        
        start_ms = int(start_time.timestamp() * 1000)
        end_ms = int(end_time.timestamp() * 1000)
        
        request = {
            "type": "candleSnapshot",
            "req": {
                "coin": self.config.coin,
                "interval": self.config.interval.value,
                "startTime": start_ms,
                "endTime": end_ms,
            }
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{base_url}/info",
                json=request,
                headers={"Content-Type": "application/json"},
            ) as resp:
                if resp.status != 200:
                    raise RuntimeError(f"Hyperliquid API error: {resp.status}")
                data = await resp.json()
                return data
    
    def run(self) -> BacktestResults:
        """Run backtest synchronously."""
        import asyncio
        return asyncio.run(self.run_async())
    
    async def run_async(self) -> BacktestResults:
        """
        Run backtest with Hyperliquid data.
        
        This is a simple Python implementation for demonstration.
        Production users should use the Rust backtest engine for performance.
        """
        self._is_running = True
        
        try:
            # Fetch historical data
            logger.info("Fetching %s data from Hyperliquid...", self.config.coin)
            raw_candles = await self._fetch_hyperliquid_candles()
            logger.info("Retrieved %d candles", len(raw_candles))
            
            if not raw_candles:
                raise RuntimeError("No candle data received")

            start_time = self.config.start_time
            end_time = self.config.end_time
            if start_time is None or end_time is None:
                raise RuntimeError("HyperliquidBacktestConfig must have start_time and end_time set")
            
            # Initialize tracking
            equity_curve = []
            orders = []
            fills = []
            position = Decimal("0")
            cash = Decimal(str(self.config.initial_capital))
            total_commission = Decimal("0")
            
            # Create instrument ID
            instrument = InstrumentId(
                venue=Venue.Hyperliquid,
                symbol=self.config.coin,
                instrument_type=InstrumentType.Perp,
            )
            
            # Run strategies
            logger.info("Running %d strategies...", len(self._strategies))
            
            for strategy in self._strategies:
                # Create context (simplified - no order routing)
                ctx = StrategyContext(0)
                
                # Call on_start
                strategy.on_start(ctx)
                
                # Process each bar
                for candle in raw_candles:
                    timestamp_ns = candle["t"] * 1_000_000
                    
                    # Create bar (Rust Bar uses float, but we convert to Decimal internally)
                    bar = Bar(
                        instrument_id=instrument,
                        timestamp_ns=timestamp_ns,
                        open=float(candle["o"]),
                        high=float(candle["h"]),
                        low=float(candle["l"]),
                        close=float(candle["c"]),
                        volume=float(candle["v"]),
                    )
                    
                    # Store decimal close price for precision
                    bar_close_decimal = Decimal(str(candle["c"]))
                    
                    # Call strategy
                    strategy.on_bar(ctx, bar)
                    
                    # Also call on_data for strategies that use it
                    if hasattr(strategy, 'on_data'):
                        strategy.on_data(ctx, bar)
                    
                    # Simple order execution (market orders only)
                    # Note: This is very simplified - real engine has much more
                    pending_orders = ctx.drain_order_requests()
                    for order in pending_orders:
                        # Execute at close price with Decimal precision
                        fill_price = Decimal(str(bar.close))
                        qty = Decimal(str(order.quantity))
                        side = order.side
                        
                        # Apply slippage
                        slippage = fill_price * Decimal(str(self.config.slippage_bps)) / Decimal("10000")
                        if side == OrderSide.Buy:
                            fill_price += slippage
                        else:
                            fill_price -= slippage
                        
                        # Calculate commission
                        notional = fill_price * qty
                        commission = notional * Decimal(str(self.config.taker_fee_bps)) / Decimal("10000")
                        total_commission += commission
                        
                        # Update position
                        if side == OrderSide.Buy:
                            position += qty
                            cash -= (notional + commission)
                        else:
                            position -= qty
                            cash += (notional - commission)
                        
                        # Record
                        orders.append({
                            "timestamp": candle["t"],
                            "side": str(side),
                            "quantity": float(qty),
                            "price": float(fill_price),
                        })
                        
                        fills.append({
                            "timestamp": candle["t"],
                            "side": str(side),
                            "quantity": float(qty),
                            "price": float(fill_price),
                            "commission": float(commission),
                        })
                    
                    # Record equity (use Decimal precision internally)
                    equity = cash + (position * bar_close_decimal)
                    equity_curve.append((datetime.fromtimestamp(candle["t"] / 1000), float(equity)))
                
                # Call on_stop
                strategy.on_stop(ctx)
            
            # Calculate metrics
            logger.info("Calculating performance metrics...")
            
            initial_equity = float(self.config.initial_capital)
            final_equity = equity_curve[-1][1] if equity_curve else initial_equity
            
            total_return = (final_equity - initial_equity) / initial_equity
            
            # Drawdown
            peak = initial_equity
            max_dd = 0.0
            drawdown_curve = []
            for ts, eq in equity_curve:
                peak = max(peak, eq)
                dd = (peak - eq) / peak if peak > 0 else 0.0
                max_dd = max(max_dd, dd)
                drawdown_curve.append((ts, dd))
            
            # Daily returns for Sharpe/Sortino
            daily_returns = []
            for i in range(1, len(equity_curve)):
                prev = equity_curve[i-1][1]
                curr = equity_curve[i][1]
                if prev > 0:
                    daily_returns.append((curr - prev) / prev)
            
            # Risk metrics
            import statistics
            if len(daily_returns) > 1:
                mean_ret = statistics.mean(daily_returns)
                std_ret = statistics.stdev(daily_returns)
                ann_sqrt = ANNUALIZATION_FACTOR ** 0.5
                sharpe = (mean_ret / std_ret * ann_sqrt) if std_ret > 0 else 0.0

                downside = [r for r in daily_returns if r < 0]
                if downside and len(downside) > 1:
                    downside_std = statistics.stdev(downside)
                    sortino = (mean_ret / downside_std * ann_sqrt) if downside_std > 0 else 0.0
                else:
                    sortino = sharpe
                volatility = std_ret * ann_sqrt
            else:
                sharpe = sortino = volatility = 0.0
            
            trading_days = (end_time - start_time).days
            annualized = total_return * (365 / trading_days) if trading_days > 0 else 0.0
            calmar = total_return / max_dd if max_dd > 0 else 0.0
            
            # Trade stats
            total_trades = len(fills)
            winning_trades = sum(1 for f in fills if f.get('pnl', 0) > 0)
            win_rate = winning_trades / total_trades if total_trades > 0 else 0.0
            
            metrics = BacktestMetrics(
                total_return=total_return,
                annualized_return=annualized,
                max_drawdown=max_dd,
                calmar_ratio=calmar,
                volatility=volatility,
                sharpe_ratio=sharpe,
                sortino_ratio=sortino,
                total_trades=total_trades,
                winning_trades=winning_trades,
                losing_trades=total_trades - winning_trades,
                win_rate=win_rate,
                avg_trade_pnl=(final_equity - initial_equity) / total_trades if total_trades > 0 else 0.0,
                avg_win=0.0,
                avg_loss=0.0,
                profit_factor=0.0,
                start_time=start_time,
                end_time=end_time,
                trading_days=trading_days,
                total_commission=float(total_commission),
            )
            
            self._results = BacktestResults(
                metrics=metrics,
                equity_curve=equity_curve,
                drawdown_curve=drawdown_curve,
                orders=orders,
                fills=fills,
                positions=[],
                config=BacktestConfig(
                    start_time=start_time,
                    end_time=end_time,
                ),
            )
            
            return self._results
            
        finally:
            self._is_running = False


# =============================================================================
# Exports
# =============================================================================

__all__ = [
    # Venue configs
    "HyperliquidVenueConfig",
    "VenueConfig",
    # Simulation
    "SimulationConfig",
    "BacktestConfig",
    # Results
    "BacktestMetrics",
    "BacktestResults",
    # Nodes
    "Node",
    "BacktestNode",
    "PaperNode",
    "LiveNode",
    # Hyperliquid
    "CandleInterval",
    "HyperliquidBacktestConfig",
    "HyperliquidBacktestNode",
    # Functions
    "backtest",
]
