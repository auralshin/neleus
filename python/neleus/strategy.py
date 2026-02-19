"""
Neleus Strategy Framework
========================

Base classes for implementing trading strategies. Strategies receive market data
callbacks and interact with the Rust engine through StrategyContext.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Dict, Optional, TYPE_CHECKING

from .types import (
    StrategyContext,
    InstrumentId,
    OrderSide,
    OrderType,
    TimeInForce,
    Bar,
    TradeTick,
    QuoteTick,
    OrderBook,
    Fill,
    Position,
    MarketData,
)


class Strategy(ABC):
    """
    Base class for trading strategies.
    
    Subclass this to implement your trading logic. The engine calls lifecycle
    methods (on_start, on_stop) and data callbacks (on_bar, on_quote, etc.)
    as events arrive.
    
    Example:
        class MomentumStrategy(Strategy):
            def __init__(self):
                super().__init__("momentum")
                self.lookback = 20
                self.threshold = 0.02
            
            def on_bar(self, ctx: StrategyContext, bar: Bar):
                # Your logic here
                if some_condition:
                    ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
    """
    
    def __init__(self, strategy_id: Optional[str] = None, config: Optional[Dict[str, Any]] = None):
        """
        Initialize the strategy.
        
        Args:
            strategy_id: Unique identifier for this strategy instance
            config: Configuration parameters
        """
        self.strategy_id = strategy_id or self.__class__.__name__
        self.config = config or {}
        self._is_running = False
    
    @property
    def id(self) -> str:
        """Alias for strategy_id for compatibility."""
        return self.strategy_id
    
    @property
    def is_running(self) -> bool:
        """Whether the strategy is currently active."""
        return self._is_running
    
    # =========================================================================
    # Lifecycle Methods
    # =========================================================================
    
    def on_start(self, ctx: StrategyContext) -> None:
        """
        Called when the strategy starts.
        
        Use this to set up subscriptions and initialize state.
        
        Args:
            ctx: Strategy context for placing orders and subscriptions
        """
        pass
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """
        Called when the strategy stops.
        
        Use this for cleanup and final position handling.
        
        Args:
            ctx: Strategy context
        """
        pass
    
    # =========================================================================
    # Data Callbacks
    # =========================================================================
    
    def on_data(self, ctx: StrategyContext, data: MarketData) -> None:
        """
        Called for any market data event.
        
        This is a catch-all handler. For specific data types, override
        on_bar, on_trade, on_quote, or on_book instead.
        
        Args:
            ctx: Strategy context
            data: Market data (Bar, TradeTick, QuoteTick, or OrderBook)
        """
        pass
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """
        Called when a new bar (OHLCV candle) completes.

        Args:
            ctx: Strategy context
            bar: The completed bar
        """
        pass

    def on_trade(self, ctx: StrategyContext, trade: TradeTick) -> None:
        """
        Called for each trade tick.

        Args:
            ctx: Strategy context
            trade: Trade tick data
        """
        pass

    def on_quote(self, ctx: StrategyContext, quote: QuoteTick) -> None:
        """
        Called for quote (BBO) updates.

        Args:
            ctx: Strategy context
            quote: Quote tick with bid/ask
        """
        pass

    def on_book(self, ctx: StrategyContext, book: OrderBook) -> None:
        """
        Called for order book updates.

        Args:
            ctx: Strategy context
            book: Order book snapshot
        """
        pass
    
    # =========================================================================
    # Event Callbacks
    # =========================================================================
    
    def on_order_accepted(self, ctx: StrategyContext, order_id: str, venue_order_id: str) -> None:
        """Called when an order is accepted by the venue."""
        pass
    
    def on_order_rejected(self, ctx: StrategyContext, order_id: str, reason: str) -> None:
        """Called when an order is rejected."""
        pass
    
    def on_order_filled(self, ctx: StrategyContext, order_id: str, fill: Fill) -> None:
        """Called when an order is filled (fully or partially)."""
        pass

    def on_order_canceled(self, ctx: StrategyContext, order_id: str) -> None:
        """Called when an order is canceled."""
        pass

    def on_position_changed(self, ctx: StrategyContext, position: Position) -> None:
        """Called when a position changes."""
        pass
    
    # =========================================================================
    # Timer Callbacks  
    # =========================================================================
    
    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(id={self.strategy_id!r})"

    def on_timer(self, ctx: StrategyContext, timer_name: str) -> None:
        """
        Called when a scheduled timer fires.
        
        Args:
            ctx: Strategy context
            timer_name: Name of the timer that fired
        """
        pass


class Actor(ABC):
    """
    Lightweight actor for background tasks.
    
    Similar to Strategy but without order management. Use for:
    - Data collection
    - Monitoring
    - Signal generation that feeds into strategies
    """
    
    def __init__(self, actor_id: Optional[str] = None):
        self.actor_id = actor_id or self.__class__.__name__
    
    def on_start(self, ctx: StrategyContext) -> None:
        """Called when the actor starts."""
        pass
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Called when the actor stops."""
        pass
    
    def on_data(self, ctx: StrategyContext, data: MarketData) -> None:
        """Called for market data events."""
        pass

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(id={self.actor_id!r})"


__all__ = [
    "Strategy",
    "Actor",
]
