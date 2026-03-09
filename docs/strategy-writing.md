# Writing Strategies

This page explains how to write strategies that work with the Neleus project runtime, what happens to the orders they generate, and how to use the live execution path.

---

## The Core Pattern

A Neleus strategy is a Python class that subclasses `Strategy`.

Minimal example:

```python
from neleus import Bar, OrderSide, Strategy, StrategyContext


class SimpleMomentumStrategy(Strategy):
    def __init__(self, lookback: int = 20):
        super().__init__("simple_momentum")
        self.lookback = lookback
        self.prices: list[float] = []

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(float(bar.close))
        if len(self.prices) < self.lookback:
            return

        average_price = sum(self.prices[-self.lookback:]) / self.lookback
        if bar.close > average_price * 1.01:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
```

---

## Strategy Lifecycle

The base class exposes these callbacks:

| Callback | When it is called |
| --- | --- |
| `on_start(ctx)` | Once at the beginning of a run, before any bars |
| `on_bar(ctx, bar)` | For every `Bar` in the candle feed |
| `on_data(ctx, data)` | Called with the same `Bar` immediately after `on_bar` |
| `on_stop(ctx)` | Once at the end of a run, after all bars |
| `on_trade(ctx, trade)` | Not yet fed by the bar-driven project runtime |
| `on_quote(ctx, quote)` | Not yet fed by the bar-driven project runtime |
| `on_book(ctx, book)` | Not yet fed by the bar-driven project runtime |

For the current project runtime, implement `on_start`, `on_bar`, and `on_stop`.

---

## How `neleus run` Calls Your Strategy

The runtime is bar-driven:

1. Loads the strategy class from `strategies/<name>.py`
2. Instantiates it with no constructor arguments: `strategy_class()`
3. Calls `on_start(ctx)`
4. Fetches candles from Hyperliquid (count: `lookback_bars` from config)
5. For each candle:
   - Converts it to a `Bar`
   - Calls `on_bar(ctx, bar)` then `on_data(ctx, bar)`
   - Drains order requests from `ctx`
   - **If `--live`:** submits each order to the exchange immediately
   - If trade monitoring is enabled: records each order to the database
6. Calls `on_stop(ctx)`

Key implications for strategy authors:

- All persistent state (price history, position flags) must live on `self`
- Constructor parameters must have defaults — `neleus run` passes no arguments
- A fresh `StrategyContext` is created per bar; do not cache context objects across bars
- `ctx.get_position(...)` is not a reliable live portfolio source in the bar-driven runtime

---

## Order APIs

### Market order

```python
ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
ctx.market_order(bar.instrument_id, OrderSide.Sell, 0.01, reduce_only=True)
```

In live mode this becomes `place_market_order(coin, is_buy, size, slippage_bps=50)` — an IOC limit order placed at mid-price ± slippage to guarantee execution.

### Limit order

```python
from neleus import TimeInForce

ctx.limit_order(
    bar.instrument_id,
    OrderSide.Buy,
    price=float(bar.close) * 0.995,
    quantity=0.01,
    time_in_force=TimeInForce.GTC,
)
```

In live mode this becomes `place_limit_order(coin, is_buy, size, price, post_only=False, reduce_only=False)`.

### Cancel order

```python
ctx.cancel_order(order_id)
```

---

## What Happens To Generated Orders

When your strategy calls `ctx.market_order(...)` or `ctx.limit_order(...)`, the `OrderRequest` is queued inside the context object. After `on_bar` returns, the runtime drains those requests.

### Without `--live` (default)

```text
strategy.on_bar(ctx, bar)
  → ctx.drain_order_requests()   → order_dicts
  → _record_orders(monitor, ...) → written to DB (if trade monitoring is on)
  → returned in RuntimeResult    → displayed in terminal
```

Orders are collected and displayed. Nothing touches the exchange.

### With `--live`

```text
strategy.on_bar(ctx, bar)
  → ctx.drain_order_requests()       → order_dicts
  → _execute_orders(trader, ...)     → submitted to Hyperliquid /exchange
  → _record_orders(monitor, ...)     → written to DB (if trade monitoring is on)
  → returned in RuntimeResult        → displayed in terminal
```

`HyperliquidTrader` is created once at startup using `HYPERLIQUID_SIGNER_PRIVATE_KEY` from `.env`. Each order is signed with EIP-712 and POSTed to `/exchange`. The exchange response (order ID, fill status, rejection reason) is logged before the next bar is processed.

If the exchange rejects an order, the error is logged and the runtime continues — a single rejection does not crash the strategy.

---

## Where Strategy State Should Live

Good:

```python
self.prices.append(float(bar.close))
self.in_position = True
self.last_signal_ts = bar.timestamp_ns
```

Avoid assuming `StrategyContext` is persistent across bars. It is not — a new context is created per bar.

---

## Use `bar.instrument_id`

The simplest way to identify the instrument when placing orders is to reuse the ID from the incoming bar:

```python
ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
```

This avoids constructing `InstrumentId` objects manually.

---

## Constructor Defaults Are Required

The project runtime calls your strategy with no arguments:

```python
strategy = strategy_class()
```

So this is safe:

```python
def __init__(self, lookback: int = 20, threshold: float = 0.02):
    ...
```

This will break `neleus run`:

```python
def __init__(self, lookback: int, threshold: float):  # no defaults
    ...
```

If you want custom parameters in backtests, combine defaults with a strategy config file (see [Projects](projects.md#strategy-specific-config-files)).

---

## Example: Mean Reversion Strategy

```python
from neleus import Bar, OrderSide, Strategy, StrategyContext


class MeanReversionStrategy(Strategy):
    def __init__(self, lookback: int = 20, entry_threshold: float = 0.02):
        super().__init__("mean_reversion")
        self.lookback = lookback
        self.entry_threshold = entry_threshold
        self.prices: list[float] = []
        self.in_position = False

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        close = float(bar.close)
        self.prices.append(close)

        if len(self.prices) < self.lookback:
            return

        window = self.prices[-self.lookback:]
        average = sum(window) / len(window)
        deviation = (close / average) - 1.0

        if not self.in_position and deviation <= -self.entry_threshold:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
            self.in_position = True
            return

        if self.in_position and close >= average:
            ctx.market_order(bar.instrument_id, OrderSide.Sell, 0.01, reduce_only=True)
            self.in_position = False
```

---

## Example: Breakout Strategy With Limit Orders

```python
from neleus import Bar, OrderSide, Strategy, StrategyContext, TimeInForce


class BreakoutStrategy(Strategy):
    def __init__(self, lookback: int = 30):
        super().__init__("breakout")
        self.lookback = lookback
        self.highs: list[float] = []
        self.lows: list[float] = []

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.highs.append(float(bar.high))
        self.lows.append(float(bar.low))

        if len(self.highs) < self.lookback:
            return

        breakout_level = max(self.highs[-self.lookback:-1])
        breakdown_level = min(self.lows[-self.lookback:-1])

        if float(bar.close) > breakout_level:
            ctx.limit_order(
                bar.instrument_id,
                OrderSide.Buy,
                price=float(bar.close),
                quantity=0.01,
                time_in_force=TimeInForce.GTC,
            )
        elif float(bar.close) < breakdown_level:
            ctx.limit_order(
                bar.instrument_id,
                OrderSide.Sell,
                price=float(bar.close),
                quantity=0.01,
                time_in_force=TimeInForce.GTC,
            )
```

---

## Example: Strategy With on_start and on_stop

```python
import logging
from neleus import Bar, OrderSide, Strategy, StrategyContext

logger = logging.getLogger(__name__)


class LoggingMomentumStrategy(Strategy):
    def __init__(self, lookback: int = 20):
        super().__init__("logging_momentum")
        self.lookback = lookback
        self.prices: list[float] = []
        self.bars_processed = 0

    def on_start(self, ctx: StrategyContext) -> None:
        logger.info("Strategy starting, lookback=%d", self.lookback)

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(float(bar.close))
        self.bars_processed += 1

        if len(self.prices) < self.lookback:
            return

        average = sum(self.prices[-self.lookback:]) / self.lookback
        if bar.close > average * 1.01:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
            logger.info("BUY signal at %.4f (avg %.4f)", float(bar.close), average)

    def on_stop(self, ctx: StrategyContext) -> None:
        logger.info("Strategy stopped after %d bars", self.bars_processed)
```

---

## Practical Tips

- Start with one symbol and one timeframe
- Use `bar.instrument_id` when placing orders — it's always correct for the active bar
- Give all constructor parameters default values
- Backtest first: `neleus backtest --strategy <name>`
- Dry-run next: `neleus run --mode once` (no `--live`)
- Test live on testnet: `neleus run --mode once --testnet --live`
- Only go to mainnet after testnet works correctly
- Use `neleus strategy show <name>` to quickly inspect source without leaving the terminal

For full strategy code examples, continue to [Strategy Examples](strategy-examples.md).
