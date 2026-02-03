"""
Competitive Multi-Agent Trading Demo

Three AI agents compete with different strategies:
- Momentum Agent: Trend-following, breakout strategies
- Mean Reversion Agent: Counter-trend, RSI-based strategies  
- Volatility Agent: Volatility-adaptive, regime-based strategies

All agents use REAL data from Hyperliquid via Rust core.
"""

from __future__ import annotations

import asyncio
import logging
import time
import numpy as np
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Any, Dict, List, Optional, Tuple
from enum import Enum

logger = logging.getLogger(__name__)

# Check for Rich library
try:
    from rich.console import Console
    from rich.live import Live
    from rich.layout import Layout
    from rich.panel import Panel
    from rich.table import Table
    from rich.text import Text
    from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn
    from rich.align import Align
    from rich.style import Style
    HAS_RICH = True
except ImportError:
    HAS_RICH = False

# Check for Rust core
try:
    from neleus_core import HyperliquidClient
    HAS_RUST_CLIENT = True
except ImportError:
    HAS_RUST_CLIENT = False


class AgentStrategy(Enum):
    """Agent trading strategies."""
    MOMENTUM = "momentum"
    MEAN_REVERSION = "mean_reversion"
    VOLATILITY = "volatility"


@dataclass
class TradeRecord:
    """Record of a simulated trade."""
    timestamp: datetime
    symbol: str
    side: str  # "long" or "short"
    entry_price: float
    exit_price: Optional[float] = None
    size: float = 1.0
    pnl: float = 0.0
    closed: bool = False


@dataclass
class AgentState:
    """State of a competing agent."""
    name: str
    strategy: AgentStrategy
    color: str
    emoji: str
    
    # Performance metrics
    initial_capital: float = 100000.0
    current_capital: float = 100000.0
    total_pnl: float = 0.0
    total_trades: int = 0
    winning_trades: int = 0
    losing_trades: int = 0
    
    # Current positions
    positions: Dict[str, TradeRecord] = field(default_factory=dict)
    trade_history: List[TradeRecord] = field(default_factory=list)
    
    # Strategy signals
    last_signal: str = "neutral"
    confidence: float = 0.0
    reasoning: str = ""
    
    # Timing
    last_action_time: datetime = field(default_factory=datetime.now)
    actions_count: int = 0
    
    @property
    def win_rate(self) -> float:
        if self.total_trades == 0:
            return 0.0
        return (self.winning_trades / self.total_trades) * 100
    
    @property
    def return_pct(self) -> float:
        return ((self.current_capital - self.initial_capital) / self.initial_capital) * 100
    
    @property
    def sharpe_ratio(self) -> float:
        if len(self.trade_history) < 2:
            return 0.0
        returns = [t.pnl / self.initial_capital for t in self.trade_history if t.closed]
        if not returns:
            return 0.0
        return (np.mean(returns) / (np.std(returns) + 0.0001)) * np.sqrt(252)


class CompetingAgent:
    """An AI agent that competes using a specific strategy."""
    
    def __init__(
        self,
        name: str,
        strategy: AgentStrategy,
        color: str,
        emoji: str,
        symbols: List[str],
        backtest_mode: bool = False,
    ):
        self.state = AgentState(
            name=name,
            strategy=strategy,
            color=color,
            emoji=emoji,
        )
        self.symbols = symbols
        self.backtest_mode = backtest_mode
        self._client: Optional[HyperliquidClient] = None
        self._market_data: Dict[str, Dict] = {}
        self._historical_candles: Dict[str, List] = {}
        self._candle_index: int = 0
        
    def _get_client(self) -> Optional[HyperliquidClient]:
        """Get or create Hyperliquid client."""
        if self._client is None and HAS_RUST_CLIENT:
            try:
                self._client = HyperliquidClient(testnet=False)
            except Exception as e:
                logger.warning(f"Failed to create client: {e}")
        return self._client
    
    async def load_historical_data(self, days: int = 7) -> bool:
        """Load historical data for backtest mode."""
        client = self._get_client()
        if not client:
            return False
        
        end_time = int(datetime.now().timestamp() * 1000)
        start_time = end_time - (days * 24 * 3600000)
        
        for symbol in self.symbols:
            try:
                candles = client.fetch_candles(symbol, "1h", start_time, end_time)
                if candles and len(candles) >= 50:
                    self._historical_candles[symbol] = candles
                    logger.info(f"Loaded {len(candles)} candles for {symbol}")
            except Exception as e:
                logger.warning(f"Failed to load history for {symbol}: {e}")
        
        return len(self._historical_candles) > 0
    
    def _calculate_indicators(self, closes: List[float], highs: List[float], lows: List[float], volumes: List[float]) -> Dict:
        """Calculate technical indicators from price data."""
        # Calculate returns
        returns = np.diff(np.log(closes))
        volatility = np.std(returns) * np.sqrt(24 * 365) * 100
        
        # RSI
        gains = np.maximum(np.diff(closes), 0)
        losses = np.abs(np.minimum(np.diff(closes), 0))
        avg_gain = np.mean(gains[-14:]) if len(gains) >= 14 else np.mean(gains)
        avg_loss = np.mean(losses[-14:]) if len(losses) >= 14 else np.mean(losses)
        rs = avg_gain / (avg_loss + 0.0001)
        rsi = 100 - (100 / (1 + rs))
        
        # Momentum
        sma_short = np.mean(closes[-6:])
        sma_long = np.mean(closes[-24:]) if len(closes) >= 24 else np.mean(closes)
        momentum = (sma_short - sma_long) / sma_long * 100
        
        # Trend
        trend = "bullish" if sma_short > sma_long else "bearish"
        
        return {
            "price": closes[-1],
            "change_24h": (closes[-1] - closes[0]) / closes[0] * 100 if len(closes) > 1 else 0,
            "high_24h": max(highs),
            "low_24h": min(lows),
            "volatility": volatility,
            "rsi": rsi,
            "momentum": momentum,
            "trend": trend,
            "volume": sum(volumes),
        }
    
    async def fetch_market_data(self, step: int = -1) -> Dict[str, Dict]:
        """Fetch market data - either live or from historical backtest."""
        client = self._get_client()
        if not client:
            return {}
        
        data = {}
        
        # Backtest mode: use historical candles
        if self.backtest_mode and self._historical_candles:
            self._candle_index = step if step >= 0 else self._candle_index
            
            for symbol, candles in self._historical_candles.items():
                # Use candles up to current step
                end_idx = min(self._candle_index + 24, len(candles))
                start_idx = max(0, end_idx - 24)
                
                if end_idx > start_idx:
                    window = candles[start_idx:end_idx]
                    closes = [c.close for c in window]
                    highs = [c.high for c in window]
                    lows = [c.low for c in window]
                    volumes = [c.volume for c in window]
                    
                    if len(closes) >= 10:
                        data[symbol] = self._calculate_indicators(closes, highs, lows, volumes)
            
            self._candle_index += 1
        else:
            # Live mode
            end_time = int(datetime.now().timestamp() * 1000)
            start_time = end_time - (24 * 3600000)  # 24 hours
            
            for symbol in self.symbols:
                try:
                    candles = client.fetch_candles(symbol, "1h", start_time, end_time)
                    if candles and len(candles) >= 10:
                        closes = [c.close for c in candles]
                        highs = [c.high for c in candles]
                        lows = [c.low for c in candles]
                        volumes = [c.volume for c in candles]
                        
                        data[symbol] = self._calculate_indicators(closes, highs, lows, volumes)
                except Exception as e:
                    logger.warning(f"Failed to fetch {symbol}: {e}")
        
        self._market_data = data
        return data
    
    def _generate_momentum_signal(self, data: Dict) -> Tuple[str, float, str]:
        """Generate signal for momentum strategy."""
        momentum = data.get("momentum", 0)
        trend = data.get("trend", "neutral")
        volatility = data.get("volatility", 50)
        
        if momentum > 2 and trend == "bullish":
            signal = "long"
            confidence = min(0.9, 0.5 + momentum / 10)
            reasoning = f"Strong bullish momentum ({momentum:.1f}%), trend confirmed"
        elif momentum < -2 and trend == "bearish":
            signal = "short"
            confidence = min(0.9, 0.5 + abs(momentum) / 10)
            reasoning = f"Strong bearish momentum ({momentum:.1f}%), trend confirmed"
        else:
            signal = "neutral"
            confidence = 0.3
            reasoning = "No clear momentum signal"
        
        return signal, confidence, reasoning
    
    def _generate_mean_reversion_signal(self, data: Dict) -> Tuple[str, float, str]:
        """Generate signal for mean reversion strategy."""
        rsi = data.get("rsi", 50)
        volatility = data.get("volatility", 50)
        change = data.get("change_24h", 0)
        
        if rsi < 30 and change < -3:
            signal = "long"
            confidence = min(0.9, 0.5 + (30 - rsi) / 30)
            reasoning = f"Oversold (RSI={rsi:.0f}), expecting bounce"
        elif rsi > 70 and change > 3:
            signal = "short"
            confidence = min(0.9, 0.5 + (rsi - 70) / 30)
            reasoning = f"Overbought (RSI={rsi:.0f}), expecting pullback"
        else:
            signal = "neutral"
            confidence = 0.3
            reasoning = f"RSI neutral ({rsi:.0f}), waiting for extremes"
        
        return signal, confidence, reasoning
    
    def _generate_volatility_signal(self, data: Dict) -> Tuple[str, float, str]:
        """Generate signal for volatility-adaptive strategy."""
        volatility = data.get("volatility", 50)
        momentum = data.get("momentum", 0)
        trend = data.get("trend", "neutral")
        
        # High volatility: Trade breakouts with smaller size
        if volatility > 80:
            if abs(momentum) > 3:
                signal = "long" if momentum > 0 else "short"
                confidence = 0.6  # Lower confidence in high vol
                reasoning = f"High vol ({volatility:.0f}%) breakout, trading momentum"
            else:
                signal = "neutral"
                confidence = 0.2
                reasoning = f"High vol ({volatility:.0f}%), waiting for breakout"
        # Low volatility: Anticipate expansion
        elif volatility < 30:
            if trend == "bullish":
                signal = "long"
                confidence = 0.7
                reasoning = f"Low vol ({volatility:.0f}%), bullish expansion likely"
            else:
                signal = "short"
                confidence = 0.7
                reasoning = f"Low vol ({volatility:.0f}%), bearish expansion likely"
        else:
            # Normal volatility: Follow trend
            signal = "long" if trend == "bullish" else "short"
            confidence = 0.5
            reasoning = f"Normal vol ({volatility:.0f}%), following {trend} trend"
        
        return signal, confidence, reasoning
    
    async def analyze_and_trade(self, step: int = -1) -> Dict[str, Any]:
        """Analyze market and execute trades based on strategy."""
        await self.fetch_market_data(step=step)
        
        results = {
            "agent": self.state.name,
            "strategy": self.state.strategy.value,
            "actions": [],
        }
        
        for symbol, data in self._market_data.items():
            # Generate signal based on strategy
            if self.state.strategy == AgentStrategy.MOMENTUM:
                signal, confidence, reasoning = self._generate_momentum_signal(data)
            elif self.state.strategy == AgentStrategy.MEAN_REVERSION:
                signal, confidence, reasoning = self._generate_mean_reversion_signal(data)
            else:  # VOLATILITY
                signal, confidence, reasoning = self._generate_volatility_signal(data)
            
            self.state.last_signal = signal
            self.state.confidence = confidence
            self.state.reasoning = reasoning
            
            # Check existing position
            existing = self.state.positions.get(symbol)
            
            # Execute trade logic
            action = None
            if existing and not existing.closed:
                # Check if we should close
                pnl_pct = 0
                if existing.side == "long":
                    pnl_pct = (data["price"] - existing.entry_price) / existing.entry_price * 100
                else:
                    pnl_pct = (existing.entry_price - data["price"]) / existing.entry_price * 100
                
                # Close if signal reverses or take profit/stop loss
                should_close = (
                    (existing.side == "long" and signal == "short") or
                    (existing.side == "short" and signal == "long") or
                    pnl_pct > 2 or pnl_pct < -1.5  # Tighter TP/SL for faster action
                )
                
                if should_close:
                    existing.exit_price = data["price"]
                    existing.pnl = pnl_pct * self.state.initial_capital / 100 * existing.size
                    existing.closed = True
                    
                    self.state.current_capital += existing.pnl
                    self.state.total_pnl += existing.pnl
                    self.state.total_trades += 1
                    
                    if existing.pnl > 0:
                        self.state.winning_trades += 1
                    else:
                        self.state.losing_trades += 1
                    
                    self.state.trade_history.append(existing)
                    del self.state.positions[symbol]
                    
                    action = {
                        "type": "close",
                        "symbol": symbol,
                        "side": existing.side,
                        "pnl": existing.pnl,
                        "pnl_pct": pnl_pct,
                    }
            
            elif signal in ["long", "short"] and confidence > 0.5:
                # Open new position
                size = 0.5 if self._market_data.get(symbol, {}).get("volatility", 50) > 80 else 1.0
                
                trade = TradeRecord(
                    timestamp=datetime.now(),
                    symbol=symbol,
                    side=signal,
                    entry_price=data["price"],
                    size=size,
                )
                self.state.positions[symbol] = trade
                
                action = {
                    "type": "open",
                    "symbol": symbol,
                    "side": signal,
                    "price": data["price"],
                    "confidence": confidence,
                    "reasoning": reasoning,
                }
            
            if action:
                results["actions"].append(action)
                self.state.actions_count += 1
                self.state.last_action_time = datetime.now()
        
        return results


class CompetitionVisualizer:
    """Rich terminal UI for the agent competition."""
    
    def __init__(self, agents: List[CompetingAgent], symbols: List[str]):
        self.agents = agents
        self.symbols = symbols
        self.console = Console()
        self.start_time = datetime.now()
        self.round_number = 0
        self._market_data: Dict[str, Dict] = {}
        
    def _create_header(self) -> Panel:
        """Create competition header."""
        elapsed = datetime.now() - self.start_time
        elapsed_str = f"{int(elapsed.total_seconds() // 60)}m {int(elapsed.total_seconds() % 60)}s"
        
        header_text = Text()
        header_text.append("🏆 ", style="bold yellow")
        header_text.append("NELEUS AI AGENT COMPETITION", style="bold white")
        header_text.append(" 🏆\n", style="bold yellow")
        header_text.append(f"Round {self.round_number} | Elapsed: {elapsed_str} | ", style="dim")
        header_text.append(f"Markets: {', '.join(self.symbols)}", style="cyan")
        
        return Panel(
            Align.center(header_text),
            style="bold blue",
            border_style="blue",
        )
    
    def _create_leaderboard(self) -> Panel:
        """Create leaderboard table."""
        # Sort agents by current capital
        sorted_agents = sorted(
            self.agents,
            key=lambda a: a.state.current_capital,
            reverse=True
        )
        
        table = Table(
            title="📊 LEADERBOARD",
            show_header=True,
            header_style="bold magenta",
            border_style="magenta",
            expand=True,
        )
        
        table.add_column("Rank", justify="center", width=6)
        table.add_column("Agent", justify="left", width=20)
        table.add_column("Strategy", justify="center", width=14)
        table.add_column("Capital", justify="right", width=14)
        table.add_column("P&L", justify="right", width=12)
        table.add_column("Return", justify="right", width=10)
        table.add_column("Trades", justify="center", width=8)
        table.add_column("Win Rate", justify="right", width=10)
        table.add_column("Sharpe", justify="right", width=8)
        
        for i, agent in enumerate(sorted_agents):
            state = agent.state
            rank_emoji = ["🥇", "🥈", "🥉"][i] if i < 3 else f"#{i+1}"
            
            pnl_style = "green" if state.total_pnl >= 0 else "red"
            return_style = "green" if state.return_pct >= 0 else "red"
            
            table.add_row(
                rank_emoji,
                f"{state.emoji} {state.name}",
                state.strategy.value,
                f"${state.current_capital:,.0f}",
                Text(f"${state.total_pnl:+,.0f}", style=pnl_style),
                Text(f"{state.return_pct:+.2f}%", style=return_style),
                str(state.total_trades),
                f"{state.win_rate:.1f}%",
                f"{state.sharpe_ratio:.2f}",
            )
        
        return Panel(table, border_style="magenta")
    
    def _create_agent_panel(self, agent: CompetingAgent) -> Panel:
        """Create detailed panel for an agent."""
        state = agent.state
        
        # Agent info
        content = Text()
        content.append(f"{state.emoji} {state.name}\n", style=f"bold {state.color}")
        content.append(f"Strategy: ", style="dim")
        content.append(f"{state.strategy.value.upper()}\n", style=state.color)
        content.append("─" * 30 + "\n", style="dim")
        
        # Performance
        pnl_style = "green" if state.total_pnl >= 0 else "red"
        content.append(f"Capital: ", style="dim")
        content.append(f"${state.current_capital:,.0f}\n", style="white")
        content.append(f"P&L: ", style="dim")
        content.append(f"${state.total_pnl:+,.0f} ({state.return_pct:+.2f}%)\n", style=pnl_style)
        content.append(f"Trades: ", style="dim")
        content.append(f"{state.total_trades} (W:{state.winning_trades} L:{state.losing_trades})\n", style="white")
        content.append(f"Win Rate: ", style="dim")
        content.append(f"{state.win_rate:.1f}%\n", style="yellow" if state.win_rate > 50 else "white")
        
        # Current signal
        content.append("─" * 30 + "\n", style="dim")
        signal_style = "green" if state.last_signal == "long" else "red" if state.last_signal == "short" else "yellow"
        content.append(f"Signal: ", style="dim")
        content.append(f"{state.last_signal.upper()} ", style=signal_style)
        content.append(f"({state.confidence:.0%})\n", style="dim")
        
        # Reasoning (truncated)
        if state.reasoning:
            reasoning = state.reasoning[:40] + "..." if len(state.reasoning) > 40 else state.reasoning
            content.append(f"💭 {reasoning}\n", style="italic dim")
        
        # Positions
        if state.positions:
            content.append("─" * 30 + "\n", style="dim")
            content.append("📈 Open Positions:\n", style="bold")
            for sym, pos in list(state.positions.items())[:3]:
                pos_style = "green" if pos.side == "long" else "red"
                current_price = agent._market_data.get(sym, {}).get("price", pos.entry_price)
                if pos.side == "long":
                    unrealized = (current_price - pos.entry_price) / pos.entry_price * 100
                else:
                    unrealized = (pos.entry_price - current_price) / pos.entry_price * 100
                pnl_style = "green" if unrealized >= 0 else "red"
                content.append(f"  {sym} ", style=pos_style)
                content.append(f"{pos.side.upper()} ", style=pos_style)
                content.append(f"@ ${pos.entry_price:,.2f} ", style="dim")
                content.append(f"({unrealized:+.2f}%)\n", style=pnl_style)
        
        return Panel(
            content,
            title=f"[{state.color}]{state.name}[/{state.color}]",
            border_style=state.color,
        )
    
    def _create_market_panel(self) -> Panel:
        """Create market data panel."""
        table = Table(
            show_header=True,
            header_style="bold cyan",
            border_style="cyan",
            expand=True,
        )
        
        table.add_column("Symbol", justify="left", width=8)
        table.add_column("Price", justify="right", width=12)
        table.add_column("24h", justify="right", width=10)
        table.add_column("Vol", justify="right", width=8)
        table.add_column("RSI", justify="right", width=6)
        table.add_column("Trend", justify="center", width=8)
        
        for symbol, data in self._market_data.items():
            change = data.get("change_24h", 0)
            change_style = "green" if change >= 0 else "red"
            trend = data.get("trend", "neutral")
            trend_style = "green" if trend == "bullish" else "red"
            
            vol = data.get("volatility", 0)
            vol_style = "red" if vol > 80 else "yellow" if vol > 50 else "green"
            
            rsi = data.get("rsi", 50)
            rsi_style = "red" if rsi > 70 else "green" if rsi < 30 else "white"
            
            table.add_row(
                symbol,
                f"${data.get('price', 0):,.2f}",
                Text(f"{change:+.2f}%", style=change_style),
                Text(f"{vol:.0f}%", style=vol_style),
                Text(f"{rsi:.0f}", style=rsi_style),
                Text(trend[:4].upper(), style=trend_style),
            )
        
        return Panel(table, title="[cyan]📈 Market Data[/cyan]", border_style="cyan")
    
    def _create_activity_feed(self, actions: List[Dict]) -> Panel:
        """Create activity feed panel."""
        content = Text()
        
        # Show last 10 actions
        for action in actions[-10:]:
            agent_name = action.get("agent", "?")
            action_type = action.get("type", "?")
            symbol = action.get("symbol", "?")
            
            # Find agent for color
            agent = next((a for a in self.agents if a.state.name == agent_name), None)
            color = agent.state.color if agent else "white"
            emoji = agent.state.emoji if agent else "🤖"
            
            timestamp = datetime.now().strftime("%H:%M:%S")
            content.append(f"{timestamp} ", style="dim")
            content.append(f"{emoji} ", style=color)
            
            if action_type == "open":
                side = action.get("side", "?")
                price = action.get("price", 0)
                side_style = "green" if side == "long" else "red"
                content.append(f"{agent_name} ", style=color)
                content.append(f"OPEN {side.upper()} ", style=side_style)
                content.append(f"{symbol} @ ${price:,.2f}\n")
            elif action_type == "close":
                pnl = action.get("pnl", 0)
                pnl_style = "green" if pnl >= 0 else "red"
                content.append(f"{agent_name} ", style=color)
                content.append(f"CLOSE ", style="yellow")
                content.append(f"{symbol} ", style="white")
                content.append(f"P&L: ${pnl:+,.0f}\n", style=pnl_style)
        
        if not actions:
            content.append("Waiting for agent actions...\n", style="dim italic")
        
        return Panel(
            content,
            title="[yellow]⚡ Activity Feed[/yellow]",
            border_style="yellow",
        )
    
    def _create_performance_chart(self) -> Panel:
        """Create ASCII performance comparison."""
        content = Text()
        
        # Get max capital for scaling
        max_capital = max(a.state.current_capital for a in self.agents)
        min_capital = min(a.state.current_capital for a in self.agents)
        range_capital = max_capital - min_capital + 1
        
        content.append("Capital Comparison\n", style="bold")
        content.append("─" * 40 + "\n", style="dim")
        
        for agent in sorted(self.agents, key=lambda a: a.state.current_capital, reverse=True):
            state = agent.state
            
            # Calculate bar width (max 30 chars)
            bar_width = int((state.current_capital - min_capital + 1) / range_capital * 30)
            bar = "█" * max(1, bar_width)
            
            pnl_style = "green" if state.return_pct >= 0 else "red"
            
            content.append(f"{state.emoji} ", style=state.color)
            content.append(bar, style=state.color)
            content.append(f" ${state.current_capital:,.0f} ", style="white")
            content.append(f"({state.return_pct:+.1f}%)\n", style=pnl_style)
        
        return Panel(content, title="[white]📊 Capital Chart[/white]", border_style="white")
    
    def create_layout(self, actions: List[Dict]) -> Layout:
        """Create the full layout."""
        layout = Layout()
        
        # Main structure
        layout.split_column(
            Layout(name="header", size=5),
            Layout(name="main", ratio=1),
            Layout(name="footer", size=15),
        )
        
        # Main section
        layout["main"].split_row(
            Layout(name="agents", ratio=3),
            Layout(name="right", ratio=2),
        )
        
        # Agent panels
        layout["agents"].split_row(
            *[Layout(name=f"agent_{i}") for i in range(len(self.agents))]
        )
        
        for i, agent in enumerate(self.agents):
            layout[f"agent_{i}"].update(self._create_agent_panel(agent))
        
        # Right side
        layout["right"].split_column(
            Layout(name="market", ratio=1),
            Layout(name="chart", ratio=1),
        )
        
        # Footer
        layout["footer"].split_row(
            Layout(name="leaderboard", ratio=2),
            Layout(name="activity", ratio=1),
        )
        
        # Update panels
        layout["header"].update(self._create_header())
        layout["market"].update(self._create_market_panel())
        layout["chart"].update(self._create_performance_chart())
        layout["leaderboard"].update(self._create_leaderboard())
        layout["activity"].update(self._create_activity_feed(actions))
        
        return layout


async def run_competition(
    symbols: List[str] = None,
    rounds: int = 20,
    delay: float = 3.0,
    backtest: bool = False,
) -> Dict[str, Any]:
    """Run the agent competition.
    
    Args:
        symbols: List of symbols to trade
        rounds: Number of competition rounds
        delay: Delay between rounds in seconds
        backtest: If True, replay historical data for faster/more dramatic results
    """
    
    if not HAS_RICH:
        print("Rich library required for visual competition. Install with: pip install rich")
        return {}
    
    if not HAS_RUST_CLIENT:
        print("Rust core required for real market data.")
        return {}
    
    # Default symbols
    if symbols is None:
        symbols = ["BTC", "ETH", "SOL"]
    
    # Create competing agents
    agents = [
        CompetingAgent(
            name="MomentumMax",
            strategy=AgentStrategy.MOMENTUM,
            color="green",
            emoji="🚀",
            symbols=symbols,
            backtest_mode=backtest,
        ),
        CompetingAgent(
            name="ReversionRex",
            strategy=AgentStrategy.MEAN_REVERSION,
            color="blue",
            emoji="🔄",
            symbols=symbols,
            backtest_mode=backtest,
        ),
        CompetingAgent(
            name="VolatilityViper",
            strategy=AgentStrategy.VOLATILITY,
            color="magenta",
            emoji="🐍",
            symbols=symbols,
            backtest_mode=backtest,
        ),
    ]
    
    visualizer = CompetitionVisualizer(agents, symbols)
    all_actions: List[Dict] = []
    
    console = Console()
    
    # Print startup banner
    console.print("\n" + "═" * 70, style="bold blue")
    console.print("🏆 NELEUS AI AGENT COMPETITION 🏆", style="bold yellow", justify="center")
    console.print("═" * 70 + "\n", style="bold blue")
    
    mode_str = "[magenta]BACKTEST MODE[/magenta] (7 days historical)" if backtest else "[cyan]LIVE MODE[/cyan]"
    console.print(f"[cyan]Mode:[/cyan] {mode_str}")
    console.print(f"[cyan]Markets:[/cyan] {', '.join(symbols)}")
    console.print(f"[cyan]Rounds:[/cyan] {rounds}")
    console.print(f"[cyan]Agents:[/cyan]")
    for agent in agents:
        console.print(f"  {agent.state.emoji} {agent.state.name} ({agent.state.strategy.value})")
    console.print()
    
    # Load historical data if in backtest mode
    if backtest:
        console.print("[yellow]Loading 7 days of historical data...[/yellow]")
        for agent in agents:
            await agent.load_historical_data(days=7)
        console.print(f"[green]Loaded historical candles for {len(symbols)} symbols[/green]")
    
    # Initial data fetch
    console.print("[yellow]Fetching initial market data...[/yellow]")
    for agent in agents:
        await agent.fetch_market_data(step=0 if backtest else -1)
        visualizer._market_data.update(agent._market_data)
    
    console.print("[green]Starting competition![/green]\n")
    await asyncio.sleep(1)
    
    try:
        with Live(visualizer.create_layout(all_actions), console=console, refresh_per_second=2) as live:
            for round_num in range(1, rounds + 1):
                visualizer.round_number = round_num
                
                # Calculate step for backtest mode
                step = round_num * 4 if backtest else -1  # 4 hours per round in backtest
                
                # Each agent analyzes and trades
                for agent in agents:
                    result = await agent.analyze_and_trade(step=step)
                    
                    # Update shared market data
                    visualizer._market_data.update(agent._market_data)
                    
                    # Record actions
                    for action in result.get("actions", []):
                        action["agent"] = agent.state.name
                        all_actions.append(action)
                
                # Update display
                live.update(visualizer.create_layout(all_actions))
                
                # Faster delay in backtest mode
                actual_delay = delay * 0.3 if backtest else delay
                await asyncio.sleep(actual_delay)
        
    except KeyboardInterrupt:
        console.print("\n[yellow]Competition interrupted by user[/yellow]")
    
    # Final results
    console.print("\n" + "═" * 70, style="bold blue")
    console.print("🏁 FINAL RESULTS 🏁", style="bold yellow", justify="center")
    console.print("═" * 70 + "\n", style="bold blue")
    
    sorted_agents = sorted(agents, key=lambda a: a.state.current_capital, reverse=True)
    
    for i, agent in enumerate(sorted_agents):
        state = agent.state
        rank = ["🥇 1st", "🥈 2nd", "🥉 3rd"][i]
        pnl_color = "green" if state.total_pnl >= 0 else "red"
        
        console.print(f"{rank} Place: {state.emoji} {state.name}")
        console.print(f"  Strategy: {state.strategy.value}")
        console.print(f"  Final Capital: ${state.current_capital:,.2f}")
        console.print(f"  Total P&L: [{pnl_color}]${state.total_pnl:+,.2f} ({state.return_pct:+.2f}%)[/{pnl_color}]")
        console.print(f"  Trades: {state.total_trades} (Win Rate: {state.win_rate:.1f}%)")
        console.print(f"  Sharpe Ratio: {state.sharpe_ratio:.2f}")
        console.print()
    
    return {
        "winner": sorted_agents[0].state.name,
        "results": {
            a.state.name: {
                "strategy": a.state.strategy.value,
                "final_capital": a.state.current_capital,
                "pnl": a.state.total_pnl,
                "return_pct": a.state.return_pct,
                "trades": a.state.total_trades,
                "win_rate": a.state.win_rate,
            }
            for a in agents
        },
    }


def main():
    """CLI entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Run AI Agent Trading Competition")
    parser.add_argument(
        "--symbols",
        "-s",
        nargs="+",
        default=["BTC", "ETH", "SOL"],
        help="Symbols to trade (default: BTC ETH SOL)",
    )
    parser.add_argument(
        "--rounds",
        "-r",
        type=int,
        default=20,
        help="Number of competition rounds (default: 20)",
    )
    parser.add_argument(
        "--delay",
        "-d",
        type=float,
        default=3.0,
        help="Delay between rounds in seconds (default: 3.0)",
    )
    parser.add_argument(
        "--backtest",
        "-b",
        action="store_true",
        help="Run in backtest mode using 7 days of historical data",
    )
    
    args = parser.parse_args()
    
    print("[neleus] AI Agent Competition")
    print(f"Symbols: {args.symbols}")
    print(f"Rounds: {args.rounds}")
    print(f"Mode: {'Backtest' if args.backtest else 'Live'}")
    print()
    
    asyncio.run(run_competition(
        symbols=args.symbols,
        rounds=args.rounds,
        delay=args.delay,
        backtest=args.backtest,
    ))


if __name__ == "__main__":
    main()
