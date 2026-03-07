"""
Project runtime helpers for one-shot and long-running strategy execution.
"""

from __future__ import annotations

import asyncio
import importlib.util
import logging
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Type

from .config import get_db_config, load_project_config
from .market import fetch_market_candles
from .strategy import Strategy
from .types import Bar, InstrumentId, InstrumentType, StrategyContext, TradeMonitor, Venue

logger = logging.getLogger(__name__)


def _normalize_symbol(symbol: str) -> str:
    return symbol[:-5] if symbol.endswith("-PERP") else symbol


def _default_strategy_name(project_path: Path) -> str:
    for strategy_file in sorted((project_path / "strategies").glob("*.py")):
        if strategy_file.name.startswith("_"):
            continue
        return strategy_file.stem
    raise FileNotFoundError(f"No strategies found in {(project_path / 'strategies')}")


def load_strategy_class(project_path: Path, strategy_name: Optional[str] = None) -> Type[Strategy]:
    strategy_name = strategy_name or _default_strategy_name(project_path)
    strategies_dir = project_path / "strategies"

    candidates = [
        strategies_dir / f"{strategy_name}.py",
        strategies_dir / f"{strategy_name}_strategy.py",
    ]
    strategy_file = next((candidate for candidate in candidates if candidate.exists()), None)
    if strategy_file is None:
        raise FileNotFoundError(f"Strategy '{strategy_name}' not found in {strategies_dir}")

    spec = importlib.util.spec_from_file_location(strategy_name, strategy_file)
    if spec is None or spec.loader is None:
        raise ImportError(f"Could not load strategy module from {strategy_file}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    for attr_name in dir(module):
        attr = getattr(module, attr_name)
        if isinstance(attr, type) and issubclass(attr, Strategy) and attr is not Strategy:
            return attr

    raise ValueError(f"No Strategy subclass found in {strategy_file}")


def _resolve_runtime_defaults(project_path: Path) -> Dict[str, Any]:
    config = load_project_config(project_path / "neleus.toml")
    market_config = config.get("market", {})
    runtime_config = config.get("runtime", {})
    db_cfg = get_db_config(config)
    return {
        "symbol": market_config.get("symbol", "BTC-PERP"),
        "timeframe": market_config.get("timeframe", "1h"),
        "lookback_bars": int(market_config.get("lookback_bars", 200)),
        "poll_interval_seconds": int(runtime_config.get("poll_interval_seconds", 60)),
        "testnet": bool(config.get("hyperliquid", {}).get("testnet", False)),
        "db_cfg": db_cfg,
    }


def _make_trade_monitor(db_cfg: Any, testnet: bool) -> Optional["TradeMonitor"]:
    """Return a TradeMonitor if the project config enables trade monitoring, else None."""
    if db_cfg.trade_monitoring and db_cfg.dsn:
        return TradeMonitor(
            connection_string=db_cfg.dsn,
            testnet=testnet,
            pool_size=db_cfg.pool_size,
        )
    return None


def _record_orders(monitor: Optional["TradeMonitor"], order_dicts: List[Dict[str, Any]]) -> None:
    if monitor is None:
        return
    import uuid
    for order in order_dicts:
        try:
            monitor.record_order(
                cloid=str(uuid.uuid4()),
                coin=order.get("instrument", ""),
                side=order.get("side", "buy").lower().replace("orderside.", ""),
                order_type=order.get("order_type", "market").lower().replace("ordertype.", ""),
                size=float(order.get("quantity", 0)),
                price=float(order.get("price")) if order.get("price") is not None else None,
                reduce_only=bool(order.get("reduce_only", False)),
            )
        except Exception as exc:
            logger.warning("Failed to record generated order via TradeMonitor: %s", exc)


def _bar_from_candle(symbol: str, candle: Dict[str, Any]) -> Bar:
    instrument_id = InstrumentId(
        venue=Venue.Hyperliquid,
        symbol=_normalize_symbol(symbol),
        instrument_type=InstrumentType.Perp,
    )
    return Bar(
        instrument_id=instrument_id,
        timestamp_ns=int(candle["timestamp"]) * 1_000_000,
        open=float(candle["open"]),
        high=float(candle["high"]),
        low=float(candle["low"]),
        close=float(candle["close"]),
        volume=float(candle["volume"]),
    )


@dataclass
class RuntimeResult:
    strategy: str
    symbol: str
    timeframe: str
    candles_processed: int
    last_price: float
    generated_orders: List[Dict[str, Any]] = field(default_factory=list)
    started_at: str = ""
    finished_at: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "strategy": self.strategy,
            "symbol": self.symbol,
            "timeframe": self.timeframe,
            "candles_processed": self.candles_processed,
            "last_price": self.last_price,
            "generated_orders": self.generated_orders,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
        }


def _orders_to_dicts(order_requests: List[Any]) -> List[Dict[str, Any]]:
    orders = []
    for request in order_requests:
        orders.append(
            {
                "instrument": request.instrument_id.symbol,
                "side": str(request.side),
                "order_type": str(request.order_type),
                "price": request.price,
                "quantity": request.quantity,
                "reduce_only": request.reduce_only,
            }
        )
    return orders


def run_project_once(
    project_path: Path,
    strategy_name: Optional[str] = None,
    symbol: Optional[str] = None,
    timeframe: Optional[str] = None,
    lookback_bars: Optional[int] = None,
    testnet: Optional[bool] = None,
) -> RuntimeResult:
    defaults = _resolve_runtime_defaults(project_path)
    symbol = symbol or defaults["symbol"]
    timeframe = timeframe or defaults["timeframe"]
    lookback_bars = lookback_bars or defaults["lookback_bars"]
    testnet = defaults["testnet"] if testnet is None else testnet

    monitor = _make_trade_monitor(defaults["db_cfg"], testnet)

    strategy_class = load_strategy_class(project_path, strategy_name)
    strategy = strategy_class()

    started_at = datetime.now(timezone.utc)
    bootstrap_ctx = StrategyContext(int(started_at.timestamp() * 1_000_000_000))
    strategy.on_start(bootstrap_ctx)

    candles = fetch_market_candles(
        symbol=symbol,
        timeframe=timeframe,
        lookback_bars=lookback_bars,
        testnet=testnet,
    )

    generated_orders: List[Dict[str, Any]] = []
    for candle in candles:
        bar = _bar_from_candle(symbol, candle)
        ctx = StrategyContext(bar.timestamp_ns)
        strategy.on_bar(ctx, bar)
        if hasattr(strategy, "on_data"):
            strategy.on_data(ctx, bar)
        order_dicts = _orders_to_dicts(ctx.drain_order_requests())
        _record_orders(monitor, order_dicts)
        generated_orders.extend(order_dicts)

    shutdown_ctx = StrategyContext(int(datetime.now(timezone.utc).timestamp() * 1_000_000_000))
    strategy.on_stop(shutdown_ctx)

    return RuntimeResult(
        strategy=strategy.__class__.__name__,
        symbol=symbol,
        timeframe=timeframe,
        candles_processed=len(candles),
        last_price=float(candles[-1]["close"]) if candles else 0.0,
        generated_orders=generated_orders,
        started_at=started_at.isoformat(),
        finished_at=datetime.now(timezone.utc).isoformat(),
    )


async def run_project_daemon(
    project_path: Path,
    strategy_name: Optional[str] = None,
    symbol: Optional[str] = None,
    timeframe: Optional[str] = None,
    lookback_bars: Optional[int] = None,
    poll_interval_seconds: Optional[int] = None,
    testnet: Optional[bool] = None,
    on_iteration: Optional[Any] = None,
) -> None:
    defaults = _resolve_runtime_defaults(project_path)
    symbol = symbol or defaults["symbol"]
    timeframe = timeframe or defaults["timeframe"]
    lookback_bars = lookback_bars or defaults["lookback_bars"]
    poll_interval_seconds = poll_interval_seconds or defaults["poll_interval_seconds"]
    testnet = defaults["testnet"] if testnet is None else testnet

    monitor = _make_trade_monitor(defaults["db_cfg"], testnet)

    strategy_class = load_strategy_class(project_path, strategy_name)
    strategy = strategy_class()
    seen_timestamps: set[int] = set()

    bootstrap_ctx = StrategyContext(int(datetime.now(timezone.utc).timestamp() * 1_000_000_000))
    strategy.on_start(bootstrap_ctx)

    try:
        while True:
            candles = fetch_market_candles(
                symbol=symbol,
                timeframe=timeframe,
                lookback_bars=lookback_bars,
                testnet=testnet,
            )
            new_candles = [candle for candle in candles if candle["timestamp"] not in seen_timestamps]

            for candle in new_candles:
                seen_timestamps.add(candle["timestamp"])
                bar = _bar_from_candle(symbol, candle)
                ctx = StrategyContext(bar.timestamp_ns)
                strategy.on_bar(ctx, bar)
                if hasattr(strategy, "on_data"):
                    strategy.on_data(ctx, bar)

                order_dicts = _orders_to_dicts(ctx.drain_order_requests())
                _record_orders(monitor, order_dicts)

                if on_iteration is not None:
                    result = RuntimeResult(
                        strategy=strategy.__class__.__name__,
                        symbol=symbol,
                        timeframe=timeframe,
                        candles_processed=1,
                        last_price=float(candle["close"]),
                        generated_orders=order_dicts,
                        started_at=datetime.now(timezone.utc).isoformat(),
                        finished_at=datetime.now(timezone.utc).isoformat(),
                    )
                    maybe_coro = on_iteration(result)
                    if asyncio.iscoroutine(maybe_coro):
                        await maybe_coro

            await asyncio.sleep(poll_interval_seconds)
    finally:
        shutdown_ctx = StrategyContext(int(datetime.now(timezone.utc).timestamp() * 1_000_000_000))
        strategy.on_stop(shutdown_ctx)


__all__ = [
    "RuntimeResult",
    "load_strategy_class",
    "run_project_daemon",
    "run_project_once",
]
