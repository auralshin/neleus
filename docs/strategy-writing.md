# Writing Strategies

This page shows how to write strategies that work with the current Neleus project workflow.

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

## Strategy Lifecycle

The base class lives in the Python package and exposes these main callbacks:

- `on_start(ctx)`
- `on_stop(ctx)`
- `on_data(ctx, data)`
- `on_bar(ctx, bar)`
- `on_trade(ctx, trade)`
- `on_quote(ctx, quote)`
- `on_book(ctx, book)`

For the current project runtime, the important ones are:

- `on_start`
- `on_bar`
- `on_data`
- `on_stop`

## How `neleus run` Calls Your Strategy

The current project runtime is bar-driven:

1. it loads a `Strategy` subclass from `strategies/<name>.py`
2. it instantiates the class with no constructor arguments
3. it calls `on_start(...)`
4. it fetches candles from Hyperliquid
5. it converts each candle into a `Bar`
6. it calls `on_bar(...)`
7. it also calls `on_data(...)` with the same `Bar`
8. it drains order requests from the context
9. it calls `on_stop(...)`

This matters for strategy authors:

- keep persistent state on `self`, not in `StrategyContext`
- give constructor parameters default values if you want `neleus run` to work
- treat the current runtime as a signal/order-request generator over bars
- when project trade monitoring is enabled, generated orders are recorded automatically after each `ctx.drain_order_requests()` call

## Where Strategy State Should Live

Good:

```python
self.prices.append(float(bar.close))
self.in_position = True
self.last_signal_ts = bar.timestamp_ns
```

Not good for the current project runtime:

- assuming `StrategyContext` is persistent across bars
- assuming `ctx.get_position(...)` is a reliable live portfolio source in `neleus run`

The project runtime creates a fresh `StrategyContext` for each bar. Your strategy instance persists; the context does not.

## Order APIs You Can Use

The current order methods exposed through `StrategyContext` are:

### Market order

```python
ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
ctx.market_order(bar.instrument_id, OrderSide.Sell, 0.01, reduce_only=True)
```

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

### Cancel order

```python
ctx.cancel_order(order_id)
```

### Subscribe methods

These methods exist on `StrategyContext`:

- `subscribe_trades(...)`
- `subscribe_quotes(...)`
- `subscribe_book(...)`

But the current project runtime path is bar-driven and does not yet feed `on_trade`, `on_quote`, or `on_book` for `neleus run`.

## What Happens To Generated Orders

When your strategy calls:

```python
ctx.market_order(...)
ctx.limit_order(...)
```

the runtime drains those order requests after each bar.

If project trade monitoring is enabled, the runtime also:

- creates a `TradeMonitor` once at startup
- assigns a UUID `cloid` to each generated order
- records the order into the configured database
- tags the record with the runtime `testnet` flag

That means strategy authors do not need explicit DB calls just to persist generated orders.

## Use `bar.instrument_id`

The simplest pattern is to reuse the instrument from the incoming bar:

```python
ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
```

That avoids constructing instrument IDs manually.

## Example: Mean Reversion Strategy

This example uses a simple rolling z-score style read from recent closes:

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

## Constructor Defaults Matter

The current project runtime calls your strategy like this:

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
def __init__(self, lookback: int, threshold: float):
    ...
```

If you want custom parameters in backtests, combine defaults with a strategy config file.

## Strategy Config Example

Create `configs/mean_reversion.yaml`:

```yaml
strategy:
  enabled: true
  class: MeanReversionStrategy
  parameters:
    lookback: 30
    entry_threshold: 0.015
```

`neleus backtest` can use those parameters when it instantiates the strategy.

## Practical Tips

- start with one symbol and one timeframe
- keep all persistent state on the strategy instance
- use `bar.instrument_id` when placing orders
- prefer constructors with defaults
- backtest first, then run once, then run daemon mode
- use `neleus strategy show <name>` to quickly inspect the active source

For full code examples, continue to [Strategy Examples](strategy-examples.md).
