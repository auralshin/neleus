"""
Neleus Core Types - Rust Bridge
================================

This module imports all types from the Rust extension (neleus_core via PyO3).
If the Rust extension is not available, it fails with a helpful error message.

The Rust bridge provides:
- Venue, InstrumentType, OrderSide, OrderType, TimeInForce, OrderState, PositionSide
- Network, FillModel, LatencyModel, SubscriptionType
- InstrumentId, TradeTick, QuoteTick, BookLevel, OrderBook, Bar
- Order, Fill, Position, OrderRequest
- StrategyContext with order management methods
- BacktestConfig, BacktestResults
- HyperliquidConfig, LighterConfig, RiskConfig
- Execution algo params (TWAP, VWAP, Iceberg)

To build the Rust extension:
    cd crates/pybridge && maturin develop --release
"""

from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any, Optional, Union
from decimal import Decimal

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
        LighterConfig,
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
        # Persistence - TimescaleDB
        PostgresEventStoreConfig,
        PostgresEventStore,
        TimescaleConfig,
        TimescaleStore,
        # Functions
        version as rust_version,
    )
    
    print(f"[neleus] Using Rust core (v{rust_version()})")
    
    # Market data union type
    MarketData = Union[Bar, TradeTick, QuoteTick, OrderBook]
    
    # Subscription types (Python-side enum since it's not in Rust yet)
    from enum import Enum
    class SubscriptionType(Enum):
        Bars = "bars"
        Trades = "trades"
        Quotes = "quotes"
        Book = "book"
    
    # Signal enum for strategy returns
    class Signal(Enum):
        """Trading signal returned by strategies."""
        BUY = "buy"
        SELL = "sell"
        HOLD = "hold"
    
    def using_rust_types() -> bool:
        """Check if using Rust types (always True in this version)."""
        return True

except ImportError as e:
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
        "     cd crates/pybridge\n"
        "     maturin develop --release\n"
        "\n"
        "  2. Or build the entire workspace:\n"
        "     cargo build --release\n"
        "     maturin develop --release -m crates/pybridge/Cargo.toml\n"
        "\n"
        "  3. For production:\n"
        "     maturin build --release\n"
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
    "LighterConfig",
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
    # Persistence - TimescaleDB
    "PostgresEventStoreConfig",
    "PostgresEventStore",
    "TimescaleConfig",
    "TimescaleStore",
    # Utils
    "using_rust_types",
]
