"""
Latency Benchmark - Measure Strategy Reaction Time

Tests how fast strategies can react to market data:
1. Data ingestion → Strategy processing → Order generation
2. Mocked execution (no network I/O)
3. Pure code performance measurement

Metrics:
- p50, p95, p99 latency
- Throughput (events/second)
- Orders per second

This measures the core event processing loop speed.
"""
import time
import statistics
from decimal import Decimal
from typing import List, Tuple

from neleus import (
    Strategy,
    StrategyContext,
    Bar,
    OrderSide,
)


class SimpleLatencyStrategy(Strategy):
    """Simple strategy - just checks last price."""
    
    def __init__(self):
        super().__init__()
        self.latencies: List[float] = []
        self.orders_placed = 0
        self.last_price = None
        self.instrument = None
        
    def on_data(self, ctx: StrategyContext, data) -> None:
        start_time = time.perf_counter_ns()
        
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        current_price = Decimal(str(data.close))
        
        # Simple logic: buy if price increased
        if self.last_price and current_price > self.last_price:
            ctx.market_order(self.instrument, OrderSide.Buy, 0.01)
            self.orders_placed += 1
        
        self.last_price = current_price
        
        end_time = time.perf_counter_ns()
        self.latencies.append(end_time - start_time)


class MediumLatencyStrategy(Strategy):
    """Medium complexity - SMA calculation."""
    
    def __init__(self):
        super().__init__()
        self.latencies: List[float] = []
        self.orders_placed = 0
        self.closes: List[Decimal] = []
        self.instrument = None
        
    def on_data(self, ctx: StrategyContext, data) -> None:
        start_time = time.perf_counter_ns()
        
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        self.closes.append(Decimal(str(data.close)))
        
        if len(self.closes) >= 20:
            sma = sum(self.closes[-20:]) / Decimal(20)
            if self.closes[-1] > sma:
                ctx.market_order(self.instrument, OrderSide.Buy, 0.01)
                self.orders_placed += 1
            elif self.closes[-1] < sma:
                ctx.market_order(self.instrument, OrderSide.Sell, 0.01)
                self.orders_placed += 1
        
        end_time = time.perf_counter_ns()
        self.latencies.append(end_time - start_time)


class ComplexLatencyStrategy(Strategy):
    """Complex - multiple indicators."""
    
    def __init__(self):
        super().__init__()
        self.latencies: List[float] = []
        self.orders_placed = 0
        self.closes: List[Decimal] = []
        self.instrument = None
        
    def on_data(self, ctx: StrategyContext, data) -> None:
        start_time = time.perf_counter_ns()
        
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        self.closes.append(Decimal(str(data.close)))
        
        if len(self.closes) >= 50:
            # Calculate multiple indicators
            sma20 = sum(self.closes[-20:]) / Decimal(20)
            sma50 = sum(self.closes[-50:]) / Decimal(50)
            
            # Volatility
            returns = [(self.closes[i] - self.closes[i-1]) / self.closes[i-1] 
                      for i in range(-20, 0)]
            volatility = Decimal(str(statistics.stdev([float(r) for r in returns])))
            
            # Trade based on conditions
            if sma20 > sma50 and volatility < Decimal("0.02"):
                ctx.market_order(self.instrument, OrderSide.Buy, 0.01)
                self.orders_placed += 1
            elif sma20 < sma50:
                ctx.market_order(self.instrument, OrderSide.Sell, 0.01)
                self.orders_placed += 1
        
        end_time = time.perf_counter_ns()
        self.latencies.append(end_time - start_time)


def run_latency_test(
    strategy: Strategy,
    name: str,
    num_events: int = 10000,
) -> Tuple[List[float], int]:
    """Run latency test using actual backtest."""
    
    print(f"\n{'='*60}")
    print(f"Testing: {name}")
    print(f"Events: {num_events:,}")
    print('='*60)
    
    from neleus import HyperliquidBacktestConfig, HyperliquidBacktestNode, CandleInterval
    
    # Use recent data
    config = HyperliquidBacktestConfig(
        coin="BTC",
        interval=CandleInterval.MIN_1,  # 1-minute bars for more events
        lookback_days=7,  # Last week of data
        initial_capital=Decimal("10000"),
        taker_fee_bps=0.0,  # No fees for pure speed test
        slippage_bps=0.0,  # No slippage
    )
    
    print("Fetching market data...")
    import asyncio
    
    async def run_test():
        node = HyperliquidBacktestNode(config)
        node.add_strategy(strategy)
        
        start_time = time.perf_counter()
        result = await node.run_async()
        end_time = time.perf_counter()
        
        return result, end_time - start_time
    
    result, total_time = asyncio.run(run_test())
    
    # Get latencies from strategy
    latencies_us = [lat / 1000 for lat in strategy.latencies]  # ns to μs
    throughput = len(latencies_us) / total_time if total_time > 0 else 0
    
    # Calculate percentiles
    if latencies_us:
        p50 = statistics.median(latencies_us)
        p95 = statistics.quantiles(latencies_us, n=20)[18]
        p99 = statistics.quantiles(latencies_us, n=100)[98]
        mean = statistics.mean(latencies_us)
        max_lat = max(latencies_us)
    else:
        p50 = p95 = p99 = mean = max_lat = 0
    
    # Print results
    print(f"\nResults:")
    print(f"  Total time:        {total_time:.3f}s")
    print(f"  Events processed:  {len(latencies_us):,}")
    print(f"  Throughput:        {throughput:,.0f} events/sec")
    print(f"  Orders placed:     {strategy.orders_placed:,}")
    print(f"\nLatency Distribution (microseconds):")
    print(f"  Mean:              {mean:>8.2f} μs")
    print(f"  Median (p50):      {p50:>8.2f} μs")
    print(f"  p95:               {p95:>8.2f} μs")
    print(f"  p99:               {p99:>8.2f} μs")
    print(f"  Max:               {max_lat:>8.2f} μs")
    
    # Show nanoseconds for ultra-fast
    if mean < 10:
        mean_ns = mean * 1000
        p50_ns = p50 * 1000
        print(f"\nUltra-low latency (nanoseconds):")
        print(f"  Mean:              {mean_ns:>8.0f} ns")
        print(f"  Median:            {p50_ns:>8.0f} ns")
    
    return latencies_us, strategy.orders_placed


def main():
    """Run full latency benchmark suite."""
    
    print("=" * 60)
    print("NELEUS LATENCY BENCHMARK")
    print("=" * 60)
    print("\nMeasuring strategy reaction time:")
    print("- Data ingestion → Processing → Order generation")
    print("- Uses real Rust core engine")
    print("- Mocked execution (no network latency)")
    print("- Tests with real market data from Hyperliquid")
    
    # Test different complexity levels
    results = {}
    
    # Simple strategy
    strategy1 = SimpleLatencyStrategy()
    latencies1, orders1 = run_latency_test(strategy1, "SIMPLE (price check)", 10000)
    results["simple"] = {"latencies": latencies1, "orders": orders1}
    
    # Medium strategy
    strategy2 = MediumLatencyStrategy()
    latencies2, orders2 = run_latency_test(strategy2, "MEDIUM (SMA)", 10000)
    results["medium"] = {"latencies": latencies2, "orders": orders2}
    
    # Complex strategy
    strategy3 = ComplexLatencyStrategy()
    latencies3, orders3 = run_latency_test(strategy3, "COMPLEX (multi-indicator)", 10000)
    results["complex"] = {"latencies": latencies3, "orders": orders3}
    
    # Summary comparison
    print(f"\n{'='*60}")
    print("SUMMARY COMPARISON")
    print('='*60)
    print(f"{'Strategy':<20} {'Mean μs':>12} {'p95 μs':>12} {'p99 μs':>12}")
    print('-'*60)
    
    for name, complexity in [("Simple", "simple"), ("Medium", "medium"), ("Complex", "complex")]:
        lats = results[complexity]["latencies"]
        if lats:
            mean = statistics.mean(lats)
            p95 = statistics.quantiles(lats, n=20)[18]
            p99 = statistics.quantiles(lats, n=100)[98]
            print(f"{name:<20} {mean:>12.2f} {p95:>12.2f} {p99:>12.2f}")
    
    print(f"\n{'='*60}")
    print("INTERPRETATION")
    print('='*60)
    print("Latency categories:")
    print("  • Ultra-low (HFT):      < 1 μs (microsecond)")
    print("  • Low latency:          1-100 μs")
    print("  • Standard:             100-1000 μs (1 ms)")
    print("  • High latency:         > 1 ms")
    print("\nWhat this means:")
    print("  • Lower latency = Faster reaction to market moves")
    print("  • p95/p99 = Consistency under load")
    print("  • Throughput = Max events you can handle per second")
    print("\nYour Rust core is:")
    mean_simple = statistics.mean(results["simple"]["latencies"]) if results["simple"]["latencies"] else 0
    if mean_simple < 1:
        print("  ⚡ ULTRA-LOW LATENCY (HFT-grade)")
    elif mean_simple < 100:
        print("    LOW LATENCY (Professional-grade)")
    elif mean_simple < 1000:
        print("    STANDARD LATENCY (Retail-grade)")
    else:
        print("  ⚠️  HIGH LATENCY (Consider optimization)")


if __name__ == "__main__":
    main()
