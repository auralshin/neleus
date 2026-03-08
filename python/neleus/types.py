"""
Neleus Core Types - Rust bridge bindings.

The Python package expects the compiled `neleus_core` extension produced from
`crates/pybridge`.
"""

from __future__ import annotations

import logging
from enum import Enum
from typing import TYPE_CHECKING, Any, Union

if TYPE_CHECKING:
    Venue: Any
    InstrumentType: Any
    OrderSide: Any
    OrderType: Any
    TimeInForce: Any
    OrderState: Any
    PositionSide: Any
    Network: Any
    FillModel: Any
    LatencyModel: Any
    InstrumentId: Any
    TradeTick: Any
    QuoteTick: Any
    BookLevel: Any
    OrderBook: Any
    Bar: Any
    Order: Any
    Fill: Any
    Position: Any
    OrderRequest: Any
    StrategyContext: Any
    BacktestResults: Any
    HyperliquidConfig: Any
    BacktestConfig: Any
    RiskConfig: Any
    TwapParams: Any
    VwapParams: Any
    IcebergParams: Any
    HyperliquidClient: Any
    HyperliquidCandle: Any
    HyperliquidMeta: Any
    HyperliquidAsset: Any
    HyperliquidSpotMeta: Any
    HyperliquidSpotToken: Any
    HyperliquidSpotMarket: Any
    HyperliquidL2Level: Any
    HyperliquidL2BookUpdate: Any
    HyperliquidL2BookStream: Any
    PostgresEventStoreConfig: Any
    PostgresEventStore: Any
    TimescaleConfig: Any
    TimescaleStore: Any
    OrderResult: Any
    OpenOrder: Any
    FillRecord: Any
    HyperliquidTrader: Any
    DbOrderRecord: Any
    DbFillRecord: Any
    PnlSummary: Any
    TradeMonitor: Any
    rust_version: Any


class SubscriptionType(Enum):
    Bars = "bars"
    Trades = "trades"
    Quotes = "quotes"
    Book = "book"


class Signal(Enum):
    """Trading signal returned by strategies."""

    BUY = "buy"
    SELL = "sell"
    HOLD = "hold"

try:
    from .neleus_core import (
        # Enums
        Venue,
        InstrumentType,
        OrderSide,
        OrderType,
        TimeInForce,
        OrderState,
        PositionSide,
        Network,
        FillModel,
        LatencyModel,
        # Market Data
        InstrumentId,
        TradeTick,
        QuoteTick,
        BookLevel,
        OrderBook,
        Bar,
        # Trading
        Order,
        Fill,
        Position,
        OrderRequest,
        # Engine
        StrategyContext,
        BacktestResults,
        # Config
        HyperliquidConfig,
        BacktestConfig,
        RiskConfig,
        # Algos
        TwapParams,
        VwapParams,
        IcebergParams,
        # Hyperliquid client
        HyperliquidClient,
        HyperliquidCandle,
        HyperliquidMeta,
        HyperliquidAsset,
        HyperliquidSpotMeta,
        HyperliquidSpotToken,
        HyperliquidSpotMarket,
        HyperliquidL2Level,
        HyperliquidL2BookUpdate,
        HyperliquidL2BookStream,
        # Persistence - TimescaleDB
        PostgresEventStoreConfig,
        PostgresEventStore,
        TimescaleConfig,
        TimescaleStore,
        # Live trading
        OrderResult,
        OpenOrder,
        FillRecord,
        HyperliquidTrader,
        # DB-backed trade monitoring
        DbOrderRecord,
        DbFillRecord,
        PnlSummary,
        TradeMonitor,
        # Functions
        version as rust_version,
    )

    logging.getLogger(__name__).info("Using Rust core (v%s)", rust_version())

    # Market data union type
    MarketData = Union[Bar, TradeTick, QuoteTick, OrderBook]

    def using_rust_types() -> bool:
        """Check if using Rust types (always True in this version)."""
        return True

except ImportError as e:
    try:
        from neleus_core import (
            # Enums
            Venue,
            InstrumentType,
            OrderSide,
            OrderType,
            TimeInForce,
            OrderState,
            PositionSide,
            Network,
            FillModel,
            LatencyModel,
            # Market Data
            InstrumentId,
            TradeTick,
            QuoteTick,
            BookLevel,
            OrderBook,
            Bar,
            # Trading
            Order,
            Fill,
            Position,
            OrderRequest,
            # Engine
            StrategyContext,
            BacktestResults,
            # Config
            HyperliquidConfig,
            BacktestConfig,
            RiskConfig,
            # Algos
            TwapParams,
            VwapParams,
            IcebergParams,
            # Hyperliquid client
            HyperliquidClient,
            HyperliquidCandle,
            HyperliquidMeta,
            HyperliquidAsset,
            HyperliquidSpotMeta,
            HyperliquidSpotToken,
            HyperliquidSpotMarket,
            HyperliquidL2Level,
            HyperliquidL2BookUpdate,
            HyperliquidL2BookStream,
            # Persistence - TimescaleDB
            PostgresEventStoreConfig,
            PostgresEventStore,
            TimescaleConfig,
            TimescaleStore,
            # Live trading
            OrderResult,
            OpenOrder,
            FillRecord,
            HyperliquidTrader,
            # DB-backed trade monitoring
            DbOrderRecord,
            DbFillRecord,
            PnlSummary,
            TradeMonitor,
            # Functions
            version as rust_version,
        )

        logging.getLogger(__name__).info(
            "Using Rust core (v%s)", rust_version())

        MarketData = Union[Bar, TradeTick, QuoteTick, OrderBook]

        def using_rust_types() -> bool:
            """Check if using Rust types (always True in this version)."""
            return True

    except ImportError:
        raise ImportError(
            "\n\n"
            "╔════════════════════════════════════════════════════════════════╗\n"
            "║  Neleus Rust Extension Not Available                           ║\n"
            "╚════════════════════════════════════════════════════════════════╝\n"
            "\n"
            "The Rust core (neleus_core) is required for Neleus to run.\n"
            "\n"
            "To build and install the Rust extension:\n"
            "\n"
            "  1. Build the PyBridge:\n"
            "     cd python\n"
            "     maturin develop --release\n"
            "\n"
            "  2. Or build the entire workspace:\n"
            "     cargo build --release\n"
            "     cd python && maturin develop --release\n"
            "\n"
            "  3. For production:\n"
            "     cd python && maturin build --profile packaging --strip\n"
            "\n"
            f"Original error: {e}\n"
        ) from e


# Export all types
__all__ = [
    # Enums
    "Venue",
    "InstrumentType",
    "OrderSide",
    "OrderType",
    "TimeInForce",
    "OrderState",
    "PositionSide",
    "Network",
    "FillModel",
    "LatencyModel",
    "SubscriptionType",
    "Signal",
    # Market Data
    "InstrumentId",
    "TradeTick",
    "QuoteTick",
    "BookLevel",
    "OrderBook",
    "Bar",
    "MarketData",
    # Trading
    "Order",
    "Fill",
    "Position",
    "OrderRequest",
    # Engine
    "StrategyContext",
    "BacktestResults",
    # Config
    "HyperliquidConfig",
    "BacktestConfig",
    "RiskConfig",
    # Algos
    "TwapParams",
    "VwapParams",
    "IcebergParams",
    # Hyperliquid client
    "HyperliquidClient",
    "HyperliquidCandle",
    "HyperliquidMeta",
    "HyperliquidAsset",
    "HyperliquidSpotMeta",
    "HyperliquidSpotToken",
    "HyperliquidSpotMarket",
    "HyperliquidL2Level",
    "HyperliquidL2BookUpdate",
    "HyperliquidL2BookStream",
    # Persistence - TimescaleDB
    "PostgresEventStoreConfig",
    "PostgresEventStore",
    "TimescaleConfig",
    "TimescaleStore",
    # Live trading
    "OrderResult",
    "OpenOrder",
    "FillRecord",
    "HyperliquidTrader",
    # DB-backed trade monitoring
    "DbOrderRecord",
    "DbFillRecord",
    "PnlSummary",
    "TradeMonitor",
    # Utils
    "using_rust_types",
]
