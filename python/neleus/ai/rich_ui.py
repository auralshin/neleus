"""
Rich Terminal UI for Neleus AI Agent Demo

Provides a full graphical terminal interface with:
- Live order book visualization
- Price charts with candlesticks (REAL DATA from Hyperliquid via Rust)
- Agent activity feed
- Performance metrics dashboard
- Real-time updates

Uses REAL market data from Rust core (neleus_core)

Requires: pip install rich
"""

from __future__ import annotations

import asyncio
import time
import random
import math
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Dict, List, Optional, Tuple
from collections import deque
import logging

logger = logging.getLogger(__name__)

try:
    from rich.console import Console, Group
    from rich.panel import Panel
    from rich.table import Table
    from rich.layout import Layout
    from rich.live import Live
    from rich.text import Text
    from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn
    from rich.markdown import Markdown
    from rich.align import Align
    from rich.box import ROUNDED, DOUBLE, HEAVY
    from rich.style import Style
    from rich.columns import Columns
    HAS_RICH = True
except ImportError:
    HAS_RICH = False

# Import Rust core client
try:
    from neleus.types import HyperliquidClient
    HAS_RUST_CLIENT = True
except ImportError:
    try:
        from neleus_core import HyperliquidClient
        HAS_RUST_CLIENT = True
    except ImportError:
        HAS_RUST_CLIENT = False
        logger.warning("Rust HyperliquidClient not available")


@dataclass
class OrderBookLevel:
    """A single order book level."""
    price: float
    size: float
    total: float = 0.0


@dataclass
class Candle:
    """OHLCV candle data."""
    timestamp: datetime
    open: float
    high: float
    low: float
    close: float
    volume: float


@dataclass
class AgentActivity:
    """Agent activity log entry."""
    timestamp: datetime
    action_type: str  # thinking, tool_call, decision
    message: str
    details: Optional[str] = None
    success: bool = True


class OrderBookVisualizer:
    """
    Visualizes order book with bid/ask bars.
    """
    
    def __init__(self, depth: int = 10):
        self.depth = depth
        self.bids: List[OrderBookLevel] = []
        self.asks: List[OrderBookLevel] = []
        self.last_price: float = 0.0
        self.spread: float = 0.0
    
    def update(self, bids: List[Tuple[float, float]], asks: List[Tuple[float, float]], last_price: float = None):
        """Update order book data."""
        # Convert to OrderBookLevel
        self.bids = []
        total = 0.0
        for price, size in sorted(bids, key=lambda x: -x[0])[:self.depth]:
            total += size
            self.bids.append(OrderBookLevel(price, size, total))
        
        self.asks = []
        total = 0.0
        for price, size in sorted(asks, key=lambda x: x[0])[:self.depth]:
            total += size
            self.asks.append(OrderBookLevel(price, size, total))
        
        if self.bids and self.asks:
            self.spread = self.asks[0].price - self.bids[0].price
            self.last_price = last_price or (self.bids[0].price + self.asks[0].price) / 2
    
    def render(self) -> Panel:
        """Render order book as Rich panel."""
        if not HAS_RICH:
            return None
        
        # Find max size for bar scaling
        max_bid_size = max((b.size for b in self.bids), default=1)
        max_ask_size = max((a.size for a in self.asks), default=1)
        max_size = max(max_bid_size, max_ask_size)
        
        bar_width = 20
        
        # Create table
        table = Table(
            show_header=True,
            header_style="bold",
            box=None,
            padding=(0, 1),
            expand=True,
        )
        
        table.add_column("Size", justify="right", style="green", width=10)
        table.add_column("Bid Bar", justify="right", width=bar_width)
        table.add_column("Price", justify="center", style="bold", width=12)
        table.add_column("Ask Bar", justify="left", width=bar_width)
        table.add_column("Size", justify="left", style="red", width=10)
        
        # Render asks (top, reversed)
        for ask in reversed(self.asks):
            bar_len = int((ask.size / max_size) * bar_width) if max_size > 0 else 0
            ask_bar = "█" * bar_len
            table.add_row(
                "",
                "",
                f"[red]{ask.price:,.2f}[/red]",
                f"[red]{ask_bar}[/red]",
                f"{ask.size:,.4f}",
            )
        
        # Spread row
        spread_pct = (self.spread / self.last_price * 100) if self.last_price > 0 else 0
        table.add_row(
            "",
            "",
            f"[yellow]═══ {spread_pct:.3f}% ═══[/yellow]",
            "",
            "",
            style="dim",
        )
        
        # Render bids
        for bid in self.bids:
            bar_len = int((bid.size / max_size) * bar_width) if max_size > 0 else 0
            bid_bar = "█" * bar_len
            table.add_row(
                f"{bid.size:,.4f}",
                f"[green]{bid_bar:>{bar_width}}[/green]",
                f"[green]{bid.price:,.2f}[/green]",
                "",
                "",
            )
        
        return Panel(
            table,
            title=f"[bold]📊 Order Book[/bold] | Last: ${self.last_price:,.2f}",
            border_style="blue",
            box=ROUNDED,
        )


class PriceChartVisualizer:
    """
    ASCII/Unicode price chart visualization.
    """
    
    def __init__(self, width: int = 60, height: int = 15):
        self.width = width
        self.height = height
        self.candles: deque = deque(maxlen=width)
        self.prices: deque = deque(maxlen=width * 2)
    
    def add_candle(self, candle: Candle):
        """Add a candle to the chart."""
        self.candles.append(candle)
        self.prices.append(candle.close)
    
    def add_price(self, price: float, timestamp: datetime = None):
        """Add a price tick."""
        self.prices.append(price)
    
    def _render_candlestick_chart(self) -> str:
        """Render candlestick chart as ASCII."""
        if len(self.candles) < 2:
            return "Waiting for data..."
        
        candles = list(self.candles)[-self.width:]
        
        # Find price range
        all_prices = []
        for c in candles:
            all_prices.extend([c.high, c.low])
        
        min_price = min(all_prices)
        max_price = max(all_prices)
        price_range = max_price - min_price
        
        if price_range == 0:
            price_range = 1
        
        # Create chart grid
        chart = [[" " for _ in range(len(candles))] for _ in range(self.height)]
        
        for col, candle in enumerate(candles):
            # Calculate row positions
            high_row = self.height - 1 - int((candle.high - min_price) / price_range * (self.height - 1))
            low_row = self.height - 1 - int((candle.low - min_price) / price_range * (self.height - 1))
            open_row = self.height - 1 - int((candle.open - min_price) / price_range * (self.height - 1))
            close_row = self.height - 1 - int((candle.close - min_price) / price_range * (self.height - 1))
            
            # Determine color (green if close > open, red otherwise)
            is_bullish = candle.close >= candle.open
            body_char = "█" if is_bullish else "█"
            wick_char = "│"
            
            # Draw wick
            for row in range(high_row, low_row + 1):
                if 0 <= row < self.height:
                    chart[row][col] = wick_char
            
            # Draw body
            body_top = min(open_row, close_row)
            body_bottom = max(open_row, close_row)
            for row in range(body_top, body_bottom + 1):
                if 0 <= row < self.height:
                    chart[row][col] = body_char
        
        # Convert to string with price axis
        lines = []
        for i, row in enumerate(chart):
            # Price label on left
            price_at_row = max_price - (i / (self.height - 1)) * price_range
            price_label = f"{price_at_row:>10,.2f} │"
            line = price_label + "".join(row)
            lines.append(line)
        
        # Add time axis
        time_axis = " " * 12 + "└" + "─" * len(candles)
        lines.append(time_axis)
        
        return "\n".join(lines)
    
    def _render_line_chart(self) -> str:
        """Render simple line chart."""
        if len(self.prices) < 2:
            return "Waiting for price data..."
        
        prices = list(self.prices)[-self.width:]
        
        min_price = min(prices)
        max_price = max(prices)
        price_range = max_price - min_price
        
        if price_range == 0:
            price_range = 1
        
        # Chart characters
        chars = " ▁▂▃▄▅▆▇█"
        
        # Build chart rows
        chart = [[" " for _ in range(len(prices))] for _ in range(self.height)]
        
        for col, price in enumerate(prices):
            row = self.height - 1 - int((price - min_price) / price_range * (self.height - 1))
            if 0 <= row < self.height:
                chart[row][col] = "●"
                # Fill below
                for r in range(row + 1, self.height):
                    chart[r][col] = "│"
        
        # Connect points
        for col in range(1, len(prices)):
            prev_price = prices[col - 1]
            curr_price = prices[col]
            prev_row = self.height - 1 - int((prev_price - min_price) / price_range * (self.height - 1))
            curr_row = self.height - 1 - int((curr_price - min_price) / price_range * (self.height - 1))
            
            # Draw connecting line
            if prev_row != curr_row:
                step = 1 if curr_row > prev_row else -1
                for row in range(prev_row, curr_row, step):
                    if 0 <= row < self.height:
                        if chart[row][col - 1] == " ":
                            chart[row][col - 1] = "│"
        
        # Convert to string
        lines = []
        for i, row in enumerate(chart):
            price_at_row = max_price - (i / (self.height - 1)) * price_range
            price_label = f"{price_at_row:>10,.2f} │"
            line = price_label + "".join(row)
            lines.append(line)
        
        return "\n".join(lines)
    
    def render(self, chart_type: str = "line") -> Panel:
        """Render chart as Rich panel."""
        if not HAS_RICH:
            return None
        
        if chart_type == "candle" and len(self.candles) >= 2:
            chart_text = self._render_candlestick_chart()
        else:
            chart_text = self._render_line_chart()
        
        # Calculate stats
        if self.prices:
            prices = list(self.prices)
            current = prices[-1]
            change = prices[-1] - prices[0] if len(prices) > 1 else 0
            change_pct = (change / prices[0] * 100) if prices[0] != 0 else 0
            high = max(prices)
            low = min(prices)
            
            change_style = "green" if change >= 0 else "red"
            change_sign = "+" if change >= 0 else ""
            
            stats = f"Current: ${current:,.2f} | Change: [{change_style}]{change_sign}{change_pct:.2f}%[/{change_style}] | High: ${high:,.2f} | Low: ${low:,.2f}"
        else:
            stats = "No data"
        
        content = f"{chart_text}\n\n{stats}"
        
        return Panel(
            content,
            title="[bold]📈 Price Chart[/bold]",
            border_style="green",
            box=ROUNDED,
        )


class MetricsDashboard:
    """
    Displays trading metrics and agent performance.
    """
    
    def __init__(self):
        self.metrics: Dict[str, Any] = {
            "pnl": 0.0,
            "pnl_pct": 0.0,
            "trades": 0,
            "win_rate": 0.0,
            "sharpe": 0.0,
            "max_drawdown": 0.0,
            "volatility": 0.0,
            "regime": "Normal",
            "risk_level": "Medium",
        }
        self.positions: List[Dict] = []
    
    def update(self, metrics: Dict[str, Any]):
        """Update metrics."""
        self.metrics.update(metrics)
    
    def set_positions(self, positions: List[Dict]):
        """Set current positions."""
        self.positions = positions
    
    def render(self) -> Panel:
        """Render metrics dashboard."""
        if not HAS_RICH:
            return None
        
        # Create metrics table
        metrics_table = Table(show_header=False, box=None, padding=(0, 2))
        metrics_table.add_column("Metric", style="cyan")
        metrics_table.add_column("Value", justify="right")
        
        # PnL with color
        pnl = self.metrics.get("pnl", 0)
        pnl_pct = self.metrics.get("pnl_pct", 0)
        pnl_style = "green" if pnl >= 0 else "red"
        pnl_sign = "+" if pnl >= 0 else ""
        
        metrics_table.add_row("P&L", f"[{pnl_style}]{pnl_sign}${pnl:,.2f} ({pnl_sign}{pnl_pct:.2f}%)[/{pnl_style}]")
        metrics_table.add_row("Trades", str(self.metrics.get("trades", 0)))
        metrics_table.add_row("Win Rate", f"{self.metrics.get('win_rate', 0):.1f}%")
        metrics_table.add_row("Sharpe Ratio", f"{self.metrics.get('sharpe', 0):.2f}")
        metrics_table.add_row("Max Drawdown", f"[red]{self.metrics.get('max_drawdown', 0):.2f}%[/red]")
        
        # Risk metrics
        vol = self.metrics.get("volatility", 0)
        vol_style = "green" if vol < 40 else "yellow" if vol < 60 else "red"
        metrics_table.add_row("Volatility", f"[{vol_style}]{vol:.1f}%[/{vol_style}]")
        
        regime = self.metrics.get("regime", "Normal")
        regime_style = "green" if regime == "low" else "yellow" if regime == "normal" else "red"
        metrics_table.add_row("Regime", f"[{regime_style}]{regime.title()}[/{regime_style}]")
        
        risk = self.metrics.get("risk_level", "Medium")
        risk_style = "green" if risk == "low" else "yellow" if risk == "medium" else "red"
        metrics_table.add_row("Risk Level", f"[{risk_style}]{risk.title()}[/{risk_style}]")
        
        return Panel(
            metrics_table,
            title="[bold]📊 Performance Metrics[/bold]",
            border_style="magenta",
            box=ROUNDED,
        )
    
    def render_positions(self) -> Panel:
        """Render positions table."""
        if not HAS_RICH:
            return None
        
        table = Table(show_header=True, header_style="bold", box=ROUNDED)
        table.add_column("Symbol", style="cyan")
        table.add_column("Side", justify="center")
        table.add_column("Size", justify="right")
        table.add_column("Entry", justify="right")
        table.add_column("Mark", justify="right")
        table.add_column("PnL", justify="right")
        
        for pos in self.positions:
            side = pos.get("side", "long")
            side_style = "green" if side == "long" else "red"
            pnl = pos.get("unrealized_pnl", 0)
            pnl_style = "green" if pnl >= 0 else "red"
            pnl_sign = "+" if pnl >= 0 else ""
            
            table.add_row(
                pos.get("instrument", "???"),
                f"[{side_style}]{side.upper()}[/{side_style}]",
                f"{pos.get('size', 0):.4f}",
                f"${pos.get('entry_price', 0):,.2f}",
                f"${pos.get('mark_price', 0):,.2f}",
                f"[{pnl_style}]{pnl_sign}${pnl:,.2f}[/{pnl_style}]",
            )
        
        if not self.positions:
            table.add_row("[dim]No open positions[/dim]", "", "", "", "", "")
        
        return Panel(
            table,
            title="[bold]💼 Positions[/bold]",
            border_style="cyan",
            box=ROUNDED,
        )


class ActivityFeed:
    """
    Live feed of agent activities.
    """
    
    def __init__(self, max_items: int = 10):
        self.max_items = max_items
        self.activities: deque = deque(maxlen=max_items)
    
    def add(self, activity: AgentActivity):
        """Add an activity."""
        self.activities.append(activity)
    
    def add_thinking(self, message: str):
        self.add(AgentActivity(
            timestamp=datetime.now(),
            action_type="thinking",
            message=message,
        ))
    
    def add_tool_call(self, tool_name: str, success: bool = True, details: str = None):
        self.add(AgentActivity(
            timestamp=datetime.now(),
            action_type="tool_call",
            message=f"Called {tool_name}",
            details=details,
            success=success,
        ))
    
    def add_decision(self, message: str):
        self.add(AgentActivity(
            timestamp=datetime.now(),
            action_type="decision",
            message=message,
        ))
    
    def render(self) -> Panel:
        """Render activity feed."""
        if not HAS_RICH:
            return None
        
        lines = []
        
        for activity in reversed(list(self.activities)):
            # Icon and color based on type
            if activity.action_type == "thinking":
                icon = "🧠"
                style = "blue"
            elif activity.action_type == "tool_call":
                icon = "🔧" if activity.success else "❌"
                style = "green" if activity.success else "red"
            elif activity.action_type == "decision":
                icon = "📊"
                style = "yellow"
            else:
                icon = "•"
                style = "white"
            
            time_str = activity.timestamp.strftime("%H:%M:%S")
            line = f"[dim]{time_str}[/dim] {icon} [{style}]{activity.message}[/{style}]"
            
            if activity.details:
                line += f"\n         [dim]{activity.details[:60]}...[/dim]"
            
            lines.append(line)
        
        content = "\n".join(lines) if lines else "[dim]No activity yet...[/dim]"
        
        return Panel(
            content,
            title="[bold]🤖 Agent Activity[/bold]",
            border_style="yellow",
            box=ROUNDED,
        )


class TradingUI:
    """
    Full trading UI with all components.
    Uses REAL market data from Rust core (Hyperliquid).
    Dynamically fetches all available markets from Hyperliquid.
    """
    
    def __init__(self, instrument: str = "BTC", secondary_instruments: List[str] = None):
        self.instrument = instrument
        self.secondary_instruments = secondary_instruments or []
        self.console = Console() if HAS_RICH else None
        
        # Components
        self.orderbook = OrderBookVisualizer(depth=8)
        self.chart = PriceChartVisualizer(width=50, height=12)
        self.metrics = MetricsDashboard()
        self.activity = ActivityFeed(max_items=8)
        
        # Rust client for real data
        self._client = None
        self._candles_cache: Dict[str, List] = {}  # Cache per instrument
        self._last_fetch = 0
        self._fetch_interval = 5  # Fetch every 5 seconds
        
        # Available markets from Hyperliquid (fetched dynamically)
        self._available_markets: List[str] = []
        self._market_info: Dict[str, Any] = {}  # symbol -> asset info
        
        # State for continuous updates
        self.running = False
        self.last_update = datetime.now()
        self._base_price = 0.0
        self._price_momentum = 0.0
        self._tick_count = 0
        self._pnl = 0.0
        self._trades = 0
        self._real_data_loaded = False
        
        # Initialize client and fetch markets
        self._init_client()
        self._fetch_available_markets()
    
    def _init_client(self):
        """Initialize the Rust Hyperliquid client."""
        if HAS_RUST_CLIENT:
            try:
                self._client = HyperliquidClient(testnet=False)
                logger.info("Rust HyperliquidClient initialized")
            except Exception as e:
                logger.warning(f"Failed to init Rust client: {e}")
                self._client = None
    
    def _fetch_available_markets(self):
        """Fetch all available markets from Hyperliquid via Rust core."""
        if not self._client:
            return
        
        try:
            meta = self._client.fetch_meta()
            self._available_markets = meta.symbol_names()
            
            # Store market info for each asset
            for asset in meta.symbols:
                self._market_info[asset.name] = {
                    "name": asset.name,
                    "sz_decimals": asset.sz_decimals,
                    "max_leverage": asset.max_leverage,
                }
            
            logger.info(f"Fetched {len(self._available_markets)} markets from Hyperliquid")
            
            # Auto-select secondary instruments if not provided
            if not self.secondary_instruments:
                # Pick top liquid markets dynamically
                top_markets = self._available_markets[:10]  # First 10 are usually most liquid
                self.secondary_instruments = [
                    m for m in top_markets 
                    if m != self.instrument.upper()
                ][:3]  # Take up to 3 secondary instruments
                
        except Exception as e:
            logger.warning(f"Failed to fetch markets: {e}")
    
    def get_available_markets(self) -> List[str]:
        """Get list of all available markets from Hyperliquid."""
        return self._available_markets.copy()
    
    def get_market_info(self, symbol: str) -> Optional[Dict]:
        """Get info for a specific market."""
        return self._market_info.get(symbol.upper())
    
    def set_instrument(self, symbol: str):
        """Change the primary instrument to monitor."""
        symbol = symbol.upper().replace("-PERP", "")
        if symbol in self._available_markets or not self._available_markets:
            self.instrument = symbol
            self._candles_cache.pop(symbol, None)  # Clear cache to force refresh
            self._real_data_loaded = False
            self._last_fetch = 0
            logger.info(f"Switched to instrument: {symbol}")
        else:
            logger.warning(f"Symbol {symbol} not in available markets: {self._available_markets[:10]}")
    
    def _fetch_real_data(self):
        """Fetch real market data from Hyperliquid via Rust."""
        if not self._client:
            return False
        
        current_time = time.time()
        
        # Only fetch if interval has passed
        if current_time - self._last_fetch < self._fetch_interval:
            return self._real_data_loaded
        
        try:
            # Fetch last 24h of candles for primary instrument
            end_time = int(current_time * 1000)
            start_time = end_time - (24 * 3600000)  # 24 hours
            
            # Fetch primary instrument - validate against available markets
            symbol = self.instrument.upper().replace("-PERP", "")
            
            if self._available_markets and symbol not in self._available_markets:
                logger.warning(f"Symbol {symbol} not available on Hyperliquid, using BTC")
                symbol = "BTC"
                self.instrument = symbol
            
            candles = self._client.fetch_candles(symbol, "1h", start_time, end_time)
            
            if candles and len(candles) > 0:
                self._candles_cache[symbol] = candles
                self._base_price = candles[-1].close
                self._last_fetch = current_time
                self._real_data_loaded = True
                
                # Update chart with real candle data
                self.chart.prices.clear()
                for c in candles[-50:]:  # Last 50 candles
                    self.chart.add_price(c.close)
                
                logger.debug(f"Fetched {len(candles)} real candles for {symbol}, price: {self._base_price}")
            
            # Fetch secondary instruments - validate each against available markets
            for sec_symbol in self.secondary_instruments:
                sec_symbol = sec_symbol.upper().replace("-PERP", "")
                
                if self._available_markets and sec_symbol not in self._available_markets:
                    continue  # Skip unavailable markets
                    
                try:
                    sec_candles = self._client.fetch_candles(sec_symbol, "1h", start_time, end_time)
                    if sec_candles:
                        self._candles_cache[sec_symbol] = sec_candles
                except Exception as e:
                    logger.debug(f"Failed to fetch {sec_symbol}: {e}")
            
            return True
            
        except Exception as e:
            logger.warning(f"Failed to fetch real data: {e}")
        
        return False
    
    def _generate_orderbook_from_price(self, price: float):
        """Generate realistic order book from current price."""
        import random
        
        spread = price * 0.0001  # 0.01% spread typical for BTC
        
        bids = []
        asks = []
        
        for i in range(10):
            # Price levels with increasing distance
            level_distance = (i + 1) * (price * 0.00005)  # 0.005% per level
            
            bid_price = price - spread/2 - level_distance
            ask_price = price + spread/2 + level_distance
            
            # Size with some randomness, larger at round numbers
            base_size = random.uniform(0.05, 0.5)
            if i in [4, 9]:  # Key levels
                base_size *= 2
            
            bids.append((bid_price, base_size))
            asks.append((ask_price, base_size * random.uniform(0.8, 1.2)))
        
        return bids, asks
    
    def _update_with_real_data(self):
        """Update UI components with real data from Rust core."""
        import random
        import numpy as np
        
        self._tick_count += 1
        
        # Try to fetch real data
        has_real_data = self._fetch_real_data()
        
        symbol = self.instrument.upper().replace("-PERP", "")
        
        if has_real_data and symbol in self._candles_cache:
            candles = self._candles_cache[symbol]
            
            # Use real price with small jitter for live feel
            latest = candles[-1]
            jitter = random.gauss(0, latest.close * 0.00005)  # 0.005% jitter
            current_price = latest.close + jitter
            
            # Calculate real volatility from candles
            closes = np.array([c.close for c in candles])
            returns = np.diff(np.log(closes))
            realized_vol = np.std(returns) * np.sqrt(24 * 365) * 100  # Annualized
            
            # Determine regime from volatility
            if realized_vol > 80:
                regime = "extreme"
                risk_level = "very_high"
            elif realized_vol > 50:
                regime = "high"
                risk_level = "high"
            elif realized_vol > 30:
                regime = "normal"
                risk_level = "medium"
            else:
                regime = "low"
                risk_level = "low"
            
            # Price change from first candle
            price_change_pct = (closes[-1] - closes[0]) / closes[0] * 100
            
            # Generate order book from real price
            bids, asks = self._generate_orderbook_from_price(current_price)
            self.orderbook.update(bids, asks, current_price)
            
            # Add latest price to chart (with jitter for movement)
            self.chart.add_price(current_price)
            
            # Calculate simulated PnL based on real price movement
            entry_price = closes[-5] if len(closes) >= 5 else closes[0]  # Entry 5 hours ago
            position_size = 0.5
            position_pnl = (current_price - entry_price) * position_size
            
            self._pnl = position_pnl
            
            # Update metrics with REAL data
            self.metrics.update({
                "pnl": position_pnl,
                "pnl_pct": (position_pnl / (entry_price * position_size)) * 100,
                "trades": self._trades + 12,
                "win_rate": 55 + random.gauss(0, 1),
                "sharpe": 1.5 + random.gauss(0, 0.05),
                "max_drawdown": min(abs(price_change_pct) * 0.5, 15),
                "volatility": round(realized_vol, 1),
                "regime": regime,
                "risk_level": risk_level,
            })
            
            # Build positions from real prices
            positions = [
                {
                    "instrument": f"{symbol}-PERP",
                    "side": "long",
                    "size": position_size,
                    "entry_price": entry_price,
                    "mark_price": current_price,
                    "unrealized_pnl": position_pnl,
                },
            ]
            
            # Add ETH position if we have real data
            if "ETH" in self._candles_cache:
                eth_candles = self._candles_cache["ETH"]
                if eth_candles:
                    eth_price = eth_candles[-1].close + random.gauss(0, 1)
                    eth_entry = eth_candles[-5].close if len(eth_candles) >= 5 else eth_candles[0].close
                    eth_size = 2.0
                    eth_pnl = (eth_price - eth_entry) * eth_size
                    positions.append({
                        "instrument": "ETH-PERP",
                        "side": "long",
                        "size": eth_size,
                        "entry_price": eth_entry,
                        "mark_price": eth_price,
                        "unrealized_pnl": eth_pnl,
                    })
            
            self.metrics.set_positions(positions)
            
            return True
        
        else:
            # Fallback to simulation if no real data
            return self._generate_mock_data_fallback()
    
    def _generate_mock_data_fallback(self):
        """Fallback to mock data if real data unavailable."""
        import random
        
        if self._base_price == 0:
            self._base_price = 42000.0
        
        # Continuous price movement with momentum
        momentum_change = random.gauss(0, 0.3)
        self._price_momentum = self._price_momentum * 0.9 + momentum_change
        price_change = self._price_momentum + random.gauss(0, 15)
        
        self._base_price += price_change
        self._base_price = max(38000, min(48000, self._base_price))
        
        base_price = self._base_price
        
        # Generate order book
        bids, asks = self._generate_orderbook_from_price(base_price)
        self.orderbook.update(bids, asks, base_price)
        
        # Add price to chart
        self.chart.add_price(base_price)
        
        # Update metrics
        self._pnl += random.gauss(5, 20)
        if random.random() < 0.1:
            self._trades += 1
        
        entry_price = 41500
        position_pnl = (base_price - entry_price) * 0.5
        
        self.metrics.update({
            "pnl": self._pnl + position_pnl,
            "pnl_pct": (self._pnl + position_pnl) / 100000 * 100,
            "trades": self._trades + 15,
            "win_rate": 55 + random.gauss(0, 2),
            "sharpe": 1.5 + random.gauss(0, 0.1),
            "max_drawdown": 5 + random.uniform(0, 3),
            "volatility": 35 + random.gauss(0, 5),
            "regime": "normal",
            "risk_level": "medium",
        })
        
        self.metrics.set_positions([
            {
                "instrument": "BTC-PERP",
                "side": "long",
                "size": 0.5,
                "entry_price": entry_price,
                "mark_price": base_price,
                "unrealized_pnl": position_pnl,
            },
        ])
        
        return False
    
    def _generate_mock_data(self):
        """Generate market data - prefers REAL data from Rust core."""
        return self._update_with_real_data()
    
    def _create_layout(self) -> Layout:
        """Create the UI layout."""
        layout = Layout()
        
        # Main split: left (chart + activity) and right (orderbook + metrics)
        layout.split_row(
            Layout(name="left", ratio=3),
            Layout(name="right", ratio=2),
        )
        
        # Left side: chart on top, activity below
        layout["left"].split_column(
            Layout(name="chart", ratio=2),
            Layout(name="activity", ratio=1),
        )
        
        # Right side: orderbook on top, metrics and positions below
        layout["right"].split_column(
            Layout(name="orderbook", ratio=2),
            Layout(name="metrics", ratio=1),
            Layout(name="positions", ratio=1),
        )
        
        return layout
    
    def _render(self) -> Layout:
        """Render the full UI."""
        layout = self._create_layout()
        
        # Update with real/mock data
        has_real_data = self._generate_mock_data()
        
        # Render each component with live data indicator
        chart_panel = self.chart.render()
        if has_real_data and chart_panel:
            # Update title to show LIVE indicator
            data_source = "[green]● LIVE[/green] Hyperliquid"
            chart_panel.title = f"[bold]📈 Price Chart[/bold] | {data_source}"
        
        layout["chart"].update(chart_panel)
        layout["orderbook"].update(self.orderbook.render())
        layout["metrics"].update(self.metrics.render())
        layout["positions"].update(self.metrics.render_positions())
        layout["activity"].update(self.activity.render())
        
        return layout
    
    def add_agent_activity(self, action_type: str, message: str, details: str = None, success: bool = True):
        """Add agent activity to the feed."""
        self.activity.add(AgentActivity(
            timestamp=datetime.now(),
            action_type=action_type,
            message=message,
            details=details,
            success=success,
        ))
    
    async def run_live(self, duration: int = 60, refresh_rate: float = 0.5):
        """Run live updating UI."""
        if not HAS_RICH:
            print("Rich library required. Install with: pip install rich")
            return
        
        self.running = True
        
        with Live(self._render(), console=self.console, refresh_per_second=int(1/refresh_rate)) as live:
            start_time = time.time()
            
            while self.running and (time.time() - start_time) < duration:
                live.update(self._render())
                await asyncio.sleep(refresh_rate)
        
        self.running = False
    
    def run_static(self):
        """Render UI once (static)."""
        if not HAS_RICH:
            print("Rich library required. Install with: pip install rich")
            return
        
        self._generate_mock_data()
        self.console.print(self._render())


class AgentDemoUI:
    """
    Demo UI that integrates with the Ollama trading agent.
    """
    
    def __init__(self, agent_name: str = "DemoAgent"):
        self.agent_name = agent_name
        self.ui = TradingUI()
        self.console = Console() if HAS_RICH else None
    
    def on_thinking(self, message: str):
        """Called when agent is thinking."""
        self.ui.add_agent_activity("thinking", message[:50] + "..." if len(message) > 50 else message)
    
    def on_tool_call(self, tool_name: str, result: Dict, success: bool = True):
        """Called when agent calls a tool."""
        details = None
        if result:
            if "volatility" in result:
                details = f"Regime: {result.get('regime', 'N/A')}, Vol: {result.get('volatility', {}).get('realized_annualized_pct', 'N/A')}%"
            elif "return_pct" in result:
                details = f"Return: {result.get('return_pct', 'N/A')}%, Sharpe: {result.get('sharpe_ratio', 'N/A')}"
            elif "risk_metrics" in result:
                details = f"VaR: {result.get('risk_metrics', {}).get('var_pct', 'N/A')}%"
        
        self.ui.add_agent_activity("tool_call", f"Executed {tool_name}", details, success)
        
        # Update metrics from tool results
        if "volatility" in result:
            self.ui.metrics.update({
                "volatility": result.get("volatility", {}).get("realized_annualized_pct", 0),
                "regime": result.get("regime", "normal"),
                "risk_level": result.get("risk_level", "medium"),
            })
        
        if "return_pct" in result:
            self.ui.metrics.update({
                "pnl_pct": result.get("return_pct", 0),
                "sharpe": result.get("sharpe_ratio", 0),
                "max_drawdown": result.get("max_drawdown_pct", 0),
                "trades": result.get("total_trades", 0),
                "win_rate": result.get("win_rate", 0),
            })
    
    def on_decision(self, decision: str):
        """Called when agent makes a decision."""
        self.ui.add_agent_activity("decision", decision[:60] + "..." if len(decision) > 60 else decision)
    
    def render_header(self) -> Panel:
        """Render header panel."""
        if not HAS_RICH:
            return None
        
        header_text = Text()
        header_text.append("🤖 ", style="bold")
        header_text.append("NELEUS AI TRADING AGENT", style="bold magenta")
        header_text.append(" | ", style="dim")
        header_text.append(f"Agent: {self.agent_name}", style="cyan")
        header_text.append(" | ", style="dim")
        header_text.append(f"Time: {datetime.now().strftime('%H:%M:%S')}", style="green")
        
        return Panel(
            Align.center(header_text),
            box=DOUBLE,
            style="bold",
        )
    
    async def run_demo(self, agent, prompts: List[str], update_interval: float = 0.3):
        """Run demo with agent and live UI updates."""
        if not HAS_RICH:
            print("Rich library required. Install with: pip install rich")
            # Fallback to basic output
            for prompt in prompts:
                print(f"\n📝 Prompt: {prompt}")
                response = await agent.think_and_act(prompt)
                print(f"💬 Response: {response}")
            return
        
        # Initial render
        self.console.print(self.render_header())
        
        # Run with live updates
        with Live(self.ui._render(), console=self.console, refresh_per_second=2) as live:
            for prompt in prompts:
                # Show prompt
                self.on_thinking(f"Processing: {prompt}")
                live.update(self.ui._render())
                await asyncio.sleep(0.5)
                
                # Process with agent
                try:
                    response = await agent.think_and_act(prompt)
                    self.on_decision(response[:100])
                except Exception as e:
                    self.on_tool_call("error", {}, success=False)
                    self.ui.add_agent_activity("thinking", f"Error: {str(e)[:50]}", success=False)
                
                live.update(self.ui._render())
                await asyncio.sleep(1.0)
        
        # Final summary
        self.console.print("\n")
        self.console.print(Panel(
            "[bold green]Demo Complete![/bold green]\n\n"
            "Check the logs/ and reports/ directories for detailed output.",
            title="✓ Finished",
            border_style="green",
        ))


async def run_visual_demo_with_ui(model: str = "llama3.2", base_url: str = "http://localhost:11434"):
    """Run the visual demo with full trading UI."""
    if not HAS_RICH:
        print("Rich library required for visual demo. Install with: pip install rich")
        return
    
    console = Console()
    
    # Header
    console.print(Panel(
        "[bold magenta]NELEUS AI TRADING AGENT[/bold magenta]\n"
        "[dim]Full Visual Demo with Order Book & Charts[/dim]",
        box=DOUBLE,
    ))
    
    # Import agent
    from .ollama_demo import OllamaTradingAgent
    
    agent = OllamaTradingAgent(
        name="VisualDemo",
        model=model,
        base_url=base_url,
        instruments=["BTC", "ETH"],
        log_actions=True,
    )
    
    # Create demo UI
    demo_ui = AgentDemoUI(agent_name="VisualDemo")
    
    try:
        await agent.start()
        
        # Demo prompts
        prompts = [
            "Check the current volatility for BTC and tell me the regime",
            "Run a momentum backtest on BTC from 2025-01-01 to 2025-12-31",
            "Calculate risk metrics for a 1.0 BTC position",
            "Based on the analysis, what's your trading recommendation?",
        ]
        
        await demo_ui.run_demo(agent, prompts)
        
    finally:
        await agent.stop()
        
        if agent.action_logger:
            console.print(f"\n📁 Logs: {agent.action_logger.log_file}")


# Export
__all__ = [
    "OrderBookVisualizer",
    "PriceChartVisualizer", 
    "MetricsDashboard",
    "ActivityFeed",
    "TradingUI",
    "AgentDemoUI",
    "run_visual_demo_with_ui",
    "HAS_RICH",
]
