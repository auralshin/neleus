"""
Neleus Signal Integration
=========================

Python interface for receiving and publishing trading signals from external
AI/ML models, quant systems, and other signal sources.

Example:
    # Connect to signal hub
    from neleus.signals import SignalClient, Signal, SignalType, SignalDirection
    
    client = SignalClient("http://localhost:8082")
    
    # Publish a signal from your AI model
    signal = Signal(
        source_id="my-ai-model",
        signal_type=SignalType.ENTRY,
        direction=SignalDirection.LONG,
        instruments=["BTC-PERP"],
        strength=0.85,
        target_price=45000.0,
        stop_loss=43000.0,
        take_profit=50000.0,
    )
    
    response = client.publish(signal)
    print(f"Signal published: {response.signal_id}")
    
    # Subscribe to signals in a strategy
    class MyStrategy(Strategy):
        def on_signal(self, ctx: StrategyContext, signal: Signal):
            if signal.strength > 0.7:
                ctx.market_order(signal.instruments[0], signal.direction.to_order_side(), 1.0)
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, TYPE_CHECKING
import json
import uuid
import requests
from threading import Thread
import time

from .constants import DEFAULT_POLL_INTERVAL_SECS, DEFAULT_SIGNAL_TTL_HOURS

logger = logging.getLogger(__name__)

if TYPE_CHECKING:
    from .types import OrderSide


class SignalType(Enum):
    """Type of trading signal."""
    ENTRY = "entry"
    EXIT = "exit"
    SCALE_IN = "scale_in"
    SCALE_OUT = "scale_out"
    REBALANCE = "rebalance"
    RISK_ALERT = "risk_alert"
    PRICE_TARGET = "price_target"
    SENTIMENT = "sentiment"
    CUSTOM = "custom"


class SignalDirection(Enum):
    """Signal direction."""
    LONG = "long"
    SHORT = "short"
    NEUTRAL = "neutral"
    UNSPECIFIED = "unspecified"
    
    def to_order_side(self) -> "OrderSide":
        """Convert to OrderSide for order placement."""
        from .types import OrderSide
        if self == SignalDirection.LONG:
            return OrderSide.Buy
        elif self == SignalDirection.SHORT:
            return OrderSide.Sell
        else:
            raise ValueError(f"Cannot convert {self} to OrderSide")


class SignalPriority(Enum):
    """Signal priority level."""
    LOW = "low"
    NORMAL = "normal"
    HIGH = "high"
    URGENT = "urgent"
    CRITICAL = "critical"


@dataclass
class SignalFeatures:
    """Features and reasoning for signal explainability."""
    key_features: Dict[str, float] = field(default_factory=dict)
    reasoning: Optional[str] = None
    confidence_breakdown: Optional[Dict[str, float]] = None
    historical_accuracy: Optional[float] = None


@dataclass
class Signal:
    """
    Trading signal from an external source.
    
    Signals represent trading recommendations from AI/ML models, quant systems,
    or other external sources. They can be published to the Signal Hub and
    consumed by trading strategies.
    """
    # Required fields
    source_id: str
    signal_type: SignalType
    direction: SignalDirection
    
    # Target instruments
    instruments: List[str] = field(default_factory=list)
    
    # Signal metadata
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    strength: float = 1.0  # 0.0 to 1.0
    priority: SignalPriority = SignalPriority.NORMAL
    
    # Price levels
    target_price: Optional[float] = None
    stop_loss: Optional[float] = None
    take_profit: Optional[float] = None
    position_size: Optional[float] = None
    
    # Timing
    timestamp: datetime = field(default_factory=datetime.utcnow)
    ttl_seconds: Optional[int] = None
    expires_at: Optional[datetime] = None
    
    # Tags and metadata
    tags: List[str] = field(default_factory=list)
    metadata: Dict[str, str] = field(default_factory=dict)
    
    # Model information (for AI signals)
    model_id: Optional[str] = None
    model_version: Optional[str] = None
    features: Optional[SignalFeatures] = None
    
    def __post_init__(self):
        """Set expiration based on TTL if not provided."""
        if self.ttl_seconds and not self.expires_at:
            self.expires_at = self.timestamp + timedelta(seconds=self.ttl_seconds)
    
    @property
    def is_expired(self) -> bool:
        """Check if signal has expired."""
        if self.expires_at:
            return datetime.utcnow() > self.expires_at
        return False
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "id": self.id,
            "source_id": self.source_id,
            "signal_type": self.signal_type.value,
            "direction": self.direction.value,
            "instruments": self.instruments,
            "strength": self.strength,
            "priority": self.priority.value,
            "target_price": self.target_price,
            "stop_loss": self.stop_loss,
            "take_profit": self.take_profit,
            "position_size": self.position_size,
            "timestamp": self.timestamp.isoformat(),
            "ttl_seconds": self.ttl_seconds,
            "expires_at": self.expires_at.isoformat() if self.expires_at else None,
            "tags": self.tags,
            "metadata": self.metadata,
            "model_id": self.model_id,
            "model_version": self.model_version,
            "features": {
                "key_features": self.features.key_features,
                "reasoning": self.features.reasoning,
                "confidence_breakdown": self.features.confidence_breakdown,
                "historical_accuracy": self.features.historical_accuracy,
            } if self.features else None,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Signal":
        """Create Signal from dictionary."""
        features = None
        if data.get("features"):
            features = SignalFeatures(
                key_features=data["features"].get("key_features", {}),
                reasoning=data["features"].get("reasoning"),
                confidence_breakdown=data["features"].get("confidence_breakdown"),
                historical_accuracy=data["features"].get("historical_accuracy"),
            )
        
        return cls(
            id=data.get("id", str(uuid.uuid4())),
            source_id=data["source_id"],
            signal_type=SignalType(data["signal_type"]),
            direction=SignalDirection(data["direction"]),
            instruments=data.get("instruments", []),
            strength=data.get("strength", 1.0),
            priority=SignalPriority(data.get("priority", "normal")),
            target_price=data.get("target_price"),
            stop_loss=data.get("stop_loss"),
            take_profit=data.get("take_profit"),
            position_size=data.get("position_size"),
            timestamp=datetime.fromisoformat(data["timestamp"]) if data.get("timestamp") else datetime.utcnow(),
            ttl_seconds=data.get("ttl_seconds"),
            expires_at=datetime.fromisoformat(data["expires_at"]) if data.get("expires_at") else None,
            tags=data.get("tags", []),
            metadata=data.get("metadata", {}),
            model_id=data.get("model_id"),
            model_version=data.get("model_version"),
            features=features,
        )


@dataclass
class SignalSubscription:
    """Subscription filter for signals."""
    source_ids: Optional[List[str]] = None
    signal_types: Optional[List[SignalType]] = None
    instruments: Optional[List[str]] = None
    tags: Optional[List[str]] = None
    min_strength: Optional[float] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "source_ids": self.source_ids,
            "signal_types": [t.value for t in self.signal_types] if self.signal_types else None,
            "instruments": self.instruments,
            "tags": self.tags,
            "min_strength": self.min_strength,
        }


@dataclass
class SignalResponse:
    """Response from signal publication."""
    signal_id: str
    status: str
    message: Optional[str] = None


class SignalClient:
    """
    Client for interacting with the Neleus Signal Hub.
    
    Example:
        client = SignalClient("http://localhost:8082", api_key="your-api-key")
        
        # Publish a signal
        signal = Signal(
            source_id="my-model",
            signal_type=SignalType.ENTRY,
            direction=SignalDirection.LONG,
            instruments=["BTC-PERP"],
            strength=0.8
        )
        client.publish(signal)
        
        # Query historical signals
        signals = client.query(source_id="my-model", limit=100)
    """
    
    def __init__(
        self,
        hub_url: str = "http://localhost:8082",
        api_key: Optional[str] = None,
        timeout: int = 30,
    ):
        """
        Initialize signal client.
        
        Args:
            hub_url: URL of the Signal Hub
            api_key: API key for authentication
            timeout: Request timeout in seconds
        """
        self.hub_url = hub_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self._session = requests.Session()
        
        if api_key:
            self._session.headers["X-API-Key"] = api_key
    
    def publish(self, signal: Signal) -> SignalResponse:
        """
        Publish a signal to the Signal Hub.
        
        Args:
            signal: The signal to publish
            
        Returns:
            SignalResponse with status
        """
        data = signal.to_dict()
        if self.api_key:
            data["api_key"] = self.api_key
        
        response = self._session.post(
            f"{self.hub_url}/api/v1/signals",
            json=data,
            timeout=self.timeout,
        )
        response.raise_for_status()
        
        result = response.json()
        return SignalResponse(
            signal_id=result.get("signal_id", ""),
            status=result.get("status", "unknown"),
            message=result.get("message"),
        )
    
    def publish_batch(self, signals: List[Signal]) -> Dict[str, Any]:
        """
        Publish multiple signals in a batch.
        
        Args:
            signals: List of signals to publish
            
        Returns:
            Dict with accepted and rejected signal IDs
        """
        data = {
            "signals": [s.to_dict() for s in signals],
        }
        if self.api_key:
            data["api_key"] = self.api_key
        
        response = self._session.post(
            f"{self.hub_url}/api/v1/signals/batch",
            json=data,
            timeout=self.timeout,
        )
        response.raise_for_status()
        
        return response.json()
    
    def query(
        self,
        source_id: Optional[str] = None,
        signal_type: Optional[SignalType] = None,
        instruments: Optional[List[str]] = None,
        start_time: Optional[datetime] = None,
        end_time: Optional[datetime] = None,
        min_strength: Optional[float] = None,
        limit: int = 100,
    ) -> List[Signal]:
        """
        Query historical signals.
        
        Args:
            source_id: Filter by source ID
            signal_type: Filter by signal type
            instruments: Filter by instruments
            start_time: Filter by start time
            end_time: Filter by end time
            min_strength: Filter by minimum strength
            limit: Maximum number of signals to return
            
        Returns:
            List of matching signals
        """
        query = {
            "source_id": source_id,
            "signal_type": signal_type.value if signal_type else None,
            "instruments": instruments,
            "start_time": start_time.isoformat() if start_time else None,
            "end_time": end_time.isoformat() if end_time else None,
            "min_strength": min_strength,
            "limit": limit,
        }
        
        # Remove None values
        query = {k: v for k, v in query.items() if v is not None}
        
        response = self._session.post(
            f"{self.hub_url}/api/v1/signals/query",
            json=query,
            timeout=self.timeout,
        )
        response.raise_for_status()
        
        return [Signal.from_dict(s) for s in response.json()]
    
    def health_check(self) -> bool:
        """Check if Signal Hub is healthy."""
        try:
            response = self._session.get(
                f"{self.hub_url}/health",
                timeout=5,
            )
            return response.status_code == 200
        except Exception:
            return False


class SignalListener:
    """
    Listener for receiving signals via polling or websocket.
    
    Example:
        def handle_signal(signal: Signal):
            print(f"Received signal: {signal.id}")
            
        listener = SignalListener("http://localhost:8082")
        listener.subscribe(
            SignalSubscription(instruments=["BTC-PERP"]),
            handle_signal
        )
        listener.start()
    """
    
    def __init__(
        self,
        hub_url: str = "http://localhost:8082",
        api_key: Optional[str] = None,
        poll_interval: float = DEFAULT_POLL_INTERVAL_SECS,
    ):
        self.client = SignalClient(hub_url, api_key)
        self.poll_interval = poll_interval
        self._handlers: List[tuple[SignalSubscription, Callable[[Signal], None]]] = []
        self._running = False
        self._thread: Optional[Thread] = None
        self._last_seen: Dict[str, datetime] = {}
    
    def subscribe(
        self,
        subscription: SignalSubscription,
        handler: Callable[[Signal], None],
    ) -> None:
        """
        Subscribe to signals matching the subscription filter.
        
        Args:
            subscription: Filter for signals
            handler: Callback function for received signals
        """
        self._handlers.append((subscription, handler))
    
    def start(self) -> None:
        """Start listening for signals."""
        self._running = True
        self._thread = Thread(target=self._poll_loop, daemon=True)
        self._thread.start()
    
    def stop(self) -> None:
        """Stop listening for signals."""
        self._running = False
        if self._thread:
            self._thread.join(timeout=5.0)
    
    def _poll_loop(self) -> None:
        """Internal polling loop."""
        while self._running:
            try:
                for subscription, handler in self._handlers:
                    signals = self.client.query(
                        source_id=subscription.source_ids[0] if subscription.source_ids else None,
                        signal_type=subscription.signal_types[0] if subscription.signal_types else None,
                        instruments=subscription.instruments,
                        min_strength=subscription.min_strength,
                        limit=100,
                    )
                    
                    for signal in signals:
                        # Check if we've already seen this signal
                        if signal.id not in self._last_seen:
                            self._last_seen[signal.id] = signal.timestamp
                            handler(signal)
                    
                    # Cleanup old entries
                    cutoff = datetime.utcnow() - timedelta(hours=DEFAULT_SIGNAL_TTL_HOURS)
                    self._last_seen = {
                        k: v for k, v in self._last_seen.items()
                        if v > cutoff
                    }
            except Exception as e:
                logger.error("Error polling signals: %s", e)
            
            time.sleep(self.poll_interval)


# Convenience functions for creating signals

def entry_signal(
    source_id: str,
    instrument: str,
    direction: SignalDirection,
    strength: float = 1.0,
    target_price: Optional[float] = None,
    stop_loss: Optional[float] = None,
    take_profit: Optional[float] = None,
    **kwargs,
) -> Signal:
    """Create an entry signal."""
    return Signal(
        source_id=source_id,
        signal_type=SignalType.ENTRY,
        direction=direction,
        instruments=[instrument],
        strength=strength,
        target_price=target_price,
        stop_loss=stop_loss,
        take_profit=take_profit,
        **kwargs,
    )


def exit_signal(
    source_id: str,
    instrument: str,
    strength: float = 1.0,
    **kwargs,
) -> Signal:
    """Create an exit signal."""
    return Signal(
        source_id=source_id,
        signal_type=SignalType.EXIT,
        direction=SignalDirection.NEUTRAL,
        instruments=[instrument],
        strength=strength,
        **kwargs,
    )


def risk_alert(
    source_id: str,
    message: str,
    priority: SignalPriority = SignalPriority.HIGH,
    **kwargs,
) -> Signal:
    """Create a risk alert signal."""
    signal = Signal(
        source_id=source_id,
        signal_type=SignalType.RISK_ALERT,
        direction=SignalDirection.NEUTRAL,
        priority=priority,
        **kwargs,
    )
    signal.metadata["message"] = message
    return signal


__all__ = [
    "SignalType",
    "SignalDirection",
    "SignalPriority",
    "SignalFeatures",
    "Signal",
    "SignalSubscription",
    "SignalResponse",
    "SignalClient",
    "SignalListener",
    "entry_signal",
    "exit_signal",
    "risk_alert",
]
