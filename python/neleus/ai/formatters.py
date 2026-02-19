"""
Data Formatters for LLM Consumption

Convert raw trading data into structured formats that LLMs can understand:
- Market data (prices, candles, orderbook)
- Signals (trading signals, alerts)
- Portfolio (positions, P&L, balances)
- Analysis (technical indicators, patterns)

Formatters produce both structured JSON and natural language summaries.
"""

from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Dict, List, Optional
import json

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Formatter threshold constants
# ---------------------------------------------------------------------------
IMBALANCE_THRESHOLD = 0.1
DIRECTION_CHANGE_PCT = 2.0
VOLATILITY_HIGH_THRESHOLD = 2.0
SIGNAL_STRONG_THRESHOLD = 0.7
SIGNAL_MODERATE_THRESHOLD = 0.4


class DataFormatter(ABC):
    """Base class for data formatters."""
    
    @abstractmethod
    def to_json(self, data: Any) -> Dict[str, Any]:
        """Convert data to JSON-serializable dict."""
        pass
    
    @abstractmethod
    def to_text(self, data: Any) -> str:
        """Convert data to natural language text."""
        pass
    
    def format(self, data: Any, output_format: str = "json") -> str:
        """
        Format data for LLM consumption.
        
        Args:
            data: Raw data to format
            output_format: "json" or "text"
        
        Returns:
            Formatted string
        """
        if output_format == "json":
            return json.dumps(self.to_json(data), indent=2)
        else:
            return self.to_text(data)


class MarketDataFormatter(DataFormatter):
    """
    Format market data for LLM consumption.
    
    Handles:
    - Ticker data (current price, volume)
    - OHLCV candles
    - Order book snapshots
    - Recent trades
    """
    
    def to_json(self, data: Any) -> Dict[str, Any]:
        """Convert market data to structured JSON."""
        if isinstance(data, dict):
            data_type = data.get("type", "ticker")
            
            if data_type == "ticker":
                return self._format_ticker(data)
            elif data_type == "candles":
                return self._format_candles(data)
            elif data_type == "orderbook":
                return self._format_orderbook(data)
            elif data_type == "trades":
                return self._format_trades(data)
        
        return {"raw": data}
    
    def to_text(self, data: Any) -> str:
        """Convert market data to natural language."""
        if isinstance(data, dict):
            data_type = data.get("type", "ticker")
            
            if data_type == "ticker":
                return self._ticker_to_text(data)
            elif data_type == "candles":
                return self._candles_to_text(data)
            elif data_type == "orderbook":
                return self._orderbook_to_text(data)
        
        return json.dumps(data)
    
    def _format_ticker(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Format ticker data."""
        return {
            "type": "ticker",
            "instrument": data.get("instrument"),
            "price": data.get("price"),
            "change_24h": data.get("change_24h"),
            "change_24h_pct": data.get("change_24h_pct"),
            "high_24h": data.get("high_24h"),
            "low_24h": data.get("low_24h"),
            "volume_24h": data.get("volume_24h"),
            "timestamp": data.get("timestamp"),
        }
    
    def _format_candles(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Format candle data with summary statistics."""
        candles = data.get("candles", [])
        
        if not candles:
            return {"type": "candles", "candles": [], "summary": {}}
        
        closes = [c.get("close", 0) for c in candles]
        highs = [c.get("high", 0) for c in candles]
        lows = [c.get("low", 0) for c in candles]
        volumes = [c.get("volume", 0) for c in candles]
        
        # Calculate summary
        current = closes[-1] if closes else 0
        start = closes[0] if closes else 0
        pct_change = ((current - start) / start * 100) if start else 0
        
        return {
            "type": "candles",
            "instrument": data.get("instrument"),
            "timeframe": data.get("timeframe"),
            "count": len(candles),
            "summary": {
                "current_price": current,
                "period_high": max(highs) if highs else 0,
                "period_low": min(lows) if lows else 0,
                "period_change_pct": round(pct_change, 2),
                "avg_volume": sum(volumes) / len(volumes) if volumes else 0,
                "volatility": self._calculate_volatility(closes),
            },
            "recent_candles": candles[-5:],  # Last 5 candles
        }
    
    def _format_orderbook(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Format orderbook data with imbalance analysis."""
        bids = data.get("bids", [])
        asks = data.get("asks", [])
        
        bid_volume = sum(b.get("size", 0) for b in bids[:10])
        ask_volume = sum(a.get("size", 0) for a in asks[:10])
        
        imbalance = 0
        if bid_volume + ask_volume > 0:
            imbalance = (bid_volume - ask_volume) / (bid_volume + ask_volume)
        
        return {
            "type": "orderbook",
            "instrument": data.get("instrument"),
            "best_bid": bids[0].get("price") if bids else None,
            "best_ask": asks[0].get("price") if asks else None,
            "spread": (asks[0].get("price", 0) - bids[0].get("price", 0)) if bids and asks else None,
            "bid_depth_10": bid_volume,
            "ask_depth_10": ask_volume,
            "imbalance": round(imbalance, 3),  # Positive = more bids, negative = more asks
            "imbalance_signal": "bullish" if imbalance > IMBALANCE_THRESHOLD else "bearish" if imbalance < -IMBALANCE_THRESHOLD else "neutral",
        }
    
    def _format_trades(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Format recent trades."""
        trades = data.get("trades", [])
        
        buy_volume = sum(t.get("size", 0) for t in trades if t.get("side") == "buy")
        sell_volume = sum(t.get("size", 0) for t in trades if t.get("side") == "sell")
        
        return {
            "type": "trades",
            "instrument": data.get("instrument"),
            "count": len(trades),
            "buy_volume": buy_volume,
            "sell_volume": sell_volume,
            "net_flow": buy_volume - sell_volume,
            "flow_signal": "buying" if buy_volume > sell_volume * 1.1 else "selling" if sell_volume > buy_volume * 1.1 else "balanced",
        }
    
    def _calculate_volatility(self, prices: List[float]) -> float:
        """Calculate simple volatility (standard deviation of returns)."""
        if len(prices) < 2:
            return 0.0
        
        returns = [(prices[i] - prices[i-1]) / prices[i-1] for i in range(1, len(prices))]
        mean_return = sum(returns) / len(returns)
        variance = sum((r - mean_return) ** 2 for r in returns) / len(returns)
        return round((variance ** 0.5) * 100, 4)  # As percentage
    
    def _ticker_to_text(self, data: Dict[str, Any]) -> str:
        """Convert ticker to natural language."""
        instrument = data.get("instrument", "Unknown")
        price = data.get("price", 0)
        change = data.get("change_24h_pct", 0)
        volume = data.get("volume_24h", 0)
        
        direction = "up" if change > 0 else "down" if change < 0 else "unchanged"
        
        return f"""{instrument} is currently trading at ${price:,.2f}, {direction} {abs(change):.2f}% in the last 24 hours.
24h High: ${data.get('high_24h', 0):,.2f} | 24h Low: ${data.get('low_24h', 0):,.2f}
24h Volume: ${volume:,.0f}"""
    
    def _candles_to_text(self, data: Dict[str, Any]) -> str:
        """Convert candles to natural language summary."""
        formatted = self.to_json(data)
        summary = formatted.get("summary", {})
        
        instrument = data.get("instrument", "Unknown")
        timeframe = data.get("timeframe", "1h")
        
        current = summary.get("current_price", 0)
        change = summary.get("period_change_pct", 0)
        high = summary.get("period_high", 0)
        low = summary.get("period_low", 0)
        vol = summary.get("volatility", 0)
        
        direction = "bullish" if change > DIRECTION_CHANGE_PCT else "bearish" if change < -DIRECTION_CHANGE_PCT else "sideways"
        vol_desc = "high" if vol > VOLATILITY_HIGH_THRESHOLD else "moderate" if vol > 1 else "low"
        
        return f"""{instrument} ({timeframe} timeframe):
Current Price: ${current:,.2f}
Period Change: {change:+.2f}%
Range: ${low:,.2f} - ${high:,.2f}
Volatility: {vol:.2f}% ({vol_desc})
Trend: {direction}"""
    
    def _orderbook_to_text(self, data: Dict[str, Any]) -> str:
        """Convert orderbook to natural language."""
        formatted = self.to_json(data)
        
        instrument = data.get("instrument", "Unknown")
        bid = formatted.get("best_bid", 0)
        ask = formatted.get("best_ask", 0)
        spread = formatted.get("spread", 0)
        imbalance = formatted.get("imbalance", 0)
        signal = formatted.get("imbalance_signal", "neutral")
        
        return f"""{instrument} Order Book:
Best Bid: ${bid:,.2f} | Best Ask: ${ask:,.2f} | Spread: ${spread:,.2f}
Order Imbalance: {imbalance:+.3f} ({signal})
{"More buyers than sellers" if imbalance > 0 else "More sellers than buyers" if imbalance < 0 else "Balanced flow"}"""


class SignalFormatter(DataFormatter):
    """
    Format trading signals for LLM consumption.
    
    Handles:
    - Entry/exit signals
    - Alert signals
    - Model predictions
    """
    
    def to_json(self, data: Any) -> Dict[str, Any]:
        """Convert signal to structured JSON."""
        if isinstance(data, list):
            return {
                "type": "signals",
                "count": len(data),
                "signals": [self._format_signal(s) for s in data],
                "summary": self._summarize_signals(data),
            }
        elif isinstance(data, dict):
            return self._format_signal(data)
        return {"raw": data}
    
    def to_text(self, data: Any) -> str:
        """Convert signals to natural language."""
        if isinstance(data, list):
            if not data:
                return "No active signals."
            
            lines = ["Active Trading Signals:"]
            for signal in data:
                lines.append(self._signal_to_text(signal))
            
            summary = self._summarize_signals(data)
            lines.append(f"\nSummary: {summary.get('bullish_count', 0)} bullish, {summary.get('bearish_count', 0)} bearish signals")
            
            return "\n".join(lines)
        elif isinstance(data, dict):
            return self._signal_to_text(data)
        return str(data)
    
    def _format_signal(self, signal: Dict[str, Any]) -> Dict[str, Any]:
        """Format a single signal."""
        return {
            "id": signal.get("id"),
            "instrument": signal.get("instrument"),
            "type": signal.get("type"),  # entry, exit, alert
            "direction": signal.get("direction"),  # long, short, neutral
            "strength": signal.get("strength"),  # 0-1
            "source": signal.get("source"),
            "timestamp": signal.get("timestamp"),
            "entry_price": signal.get("entry_price"),
            "target_price": signal.get("target_price"),
            "stop_loss": signal.get("stop_loss"),
            "metadata": signal.get("metadata", {}),
        }
    
    def _summarize_signals(self, signals: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Summarize multiple signals."""
        bullish = [s for s in signals if s.get("direction") == "long"]
        bearish = [s for s in signals if s.get("direction") == "short"]
        
        avg_strength = sum(s.get("strength", 0) for s in signals) / len(signals) if signals else 0
        
        return {
            "bullish_count": len(bullish),
            "bearish_count": len(bearish),
            "neutral_count": len(signals) - len(bullish) - len(bearish),
            "average_strength": round(avg_strength, 2),
            "consensus": "bullish" if len(bullish) > len(bearish) else "bearish" if len(bearish) > len(bullish) else "mixed",
        }
    
    def _signal_to_text(self, signal: Dict[str, Any]) -> str:
        """Convert a single signal to text."""
        instrument = signal.get("instrument", "Unknown")
        direction = signal.get("direction", "neutral")
        strength = signal.get("strength", 0)
        source = signal.get("source", "unknown")
        signal_type = signal.get("type", "signal")
        
        strength_desc = "strong" if strength > SIGNAL_STRONG_THRESHOLD else "moderate" if strength > SIGNAL_MODERATE_THRESHOLD else "weak"
        
        return f"  - {instrument}: {strength_desc} {direction} {signal_type} (strength: {strength:.2f}, source: {source})"


class PortfolioFormatter(DataFormatter):
    """
    Format portfolio data for LLM consumption.
    
    Handles:
    - Account balances
    - Open positions
    - P&L summaries
    - Risk metrics
    """
    
    def to_json(self, data: Any) -> Dict[str, Any]:
        """Convert portfolio to structured JSON."""
        if not isinstance(data, dict):
            return {"raw": data}
        
        positions = data.get("positions", [])
        
        return {
            "type": "portfolio",
            "account": {
                "equity": data.get("equity", 0),
                "available_balance": data.get("available_balance", 0),
                "used_margin": data.get("used_margin", 0),
                "margin_ratio": data.get("margin_ratio", 0),
            },
            "pnl": {
                "unrealized": data.get("unrealized_pnl", 0),
                "realized_today": data.get("realized_pnl_today", 0),
                "total_return_pct": data.get("total_return_pct", 0),
            },
            "positions": [self._format_position(p) for p in positions],
            "position_summary": self._summarize_positions(positions),
            "risk_metrics": {
                "total_exposure": sum(abs(p.get("notional", 0)) for p in positions),
                "long_exposure": sum(p.get("notional", 0) for p in positions if p.get("side") == "long"),
                "short_exposure": sum(abs(p.get("notional", 0)) for p in positions if p.get("side") == "short"),
                "position_count": len(positions),
            }
        }
    
    def to_text(self, data: Any) -> str:
        """Convert portfolio to natural language."""
        formatted = self.to_json(data)
        
        account = formatted.get("account", {})
        pnl = formatted.get("pnl", {})
        positions = formatted.get("positions", [])
        risk = formatted.get("risk_metrics", {})
        
        lines = [
            "Portfolio Summary:",
            f"  Equity: ${account.get('equity', 0):,.2f}",
            f"  Available: ${account.get('available_balance', 0):,.2f}",
            f"  Unrealized P&L: ${pnl.get('unrealized', 0):+,.2f}",
            "",
        ]
        
        if positions:
            lines.append(f"Open Positions ({len(positions)}):")
            for pos in positions:
                side_emoji = "LONG" if pos.get("side") == "long" else "SHORT"
                lines.append(
                    f"  {pos.get('instrument')}: {side_emoji} {pos.get('size', 0)} @ ${pos.get('entry_price', 0):,.2f} "
                    f"(P&L: ${pos.get('unrealized_pnl', 0):+,.2f})"
                )
        else:
            lines.append("No open positions.")
        
        lines.extend([
            "",
            "Risk Exposure:",
            f"  Total: ${risk.get('total_exposure', 0):,.2f}",
            f"  Long: ${risk.get('long_exposure', 0):,.2f} | Short: ${abs(risk.get('short_exposure', 0)):,.2f}",
        ])
        
        return "\n".join(lines)
    
    def _format_position(self, position: Dict[str, Any]) -> Dict[str, Any]:
        """Format a single position."""
        entry = position.get("entry_price", 0)
        mark = position.get("mark_price", 0)
        size = position.get("size", 0)
        
        return {
            "instrument": position.get("instrument"),
            "side": position.get("side"),
            "size": size,
            "entry_price": entry,
            "mark_price": mark,
            "notional": abs(size * mark),
            "unrealized_pnl": position.get("unrealized_pnl", 0),
            "pnl_pct": ((mark - entry) / entry * 100) if entry else 0,
            "leverage": position.get("leverage", 1),
        }
    
    def _summarize_positions(self, positions: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Summarize position metrics."""
        if not positions:
            return {"count": 0, "net_pnl": 0}
        
        net_pnl = sum(p.get("unrealized_pnl", 0) for p in positions)
        winning = [p for p in positions if p.get("unrealized_pnl", 0) > 0]
        
        return {
            "count": len(positions),
            "winning_count": len(winning),
            "losing_count": len(positions) - len(winning),
            "net_pnl": net_pnl,
            "best_position": max(positions, key=lambda p: p.get("unrealized_pnl", 0)).get("instrument") if positions else None,
            "worst_position": min(positions, key=lambda p: p.get("unrealized_pnl", 0)).get("instrument") if positions else None,
        }


class AnalysisFormatter(DataFormatter):
    """
    Format technical analysis for LLM consumption.
    
    Handles:
    - Technical indicators (RSI, MACD, etc.)
    - Pattern detection
    - Support/resistance levels
    - Trend analysis
    """
    
    def to_json(self, data: Any) -> Dict[str, Any]:
        """Convert analysis to structured JSON."""
        if not isinstance(data, dict):
            return {"raw": data}
        
        indicators = data.get("indicators", {})
        signals = data.get("signals", [])
        
        return {
            "type": "analysis",
            "instrument": data.get("instrument"),
            "timeframe": data.get("timeframe"),
            "price": data.get("price"),
            "indicators": {
                "rsi": self._format_rsi(indicators.get("rsi")),
                "macd": self._format_macd(indicators.get("macd")),
                "bollinger": self._format_bollinger(indicators.get("bollinger")),
                "moving_averages": indicators.get("moving_averages"),
            },
            "signals": signals,
            "overall_signal": data.get("overall_signal", "neutral"),
            "confidence": self._calculate_confidence(signals),
        }
    
    def to_text(self, data: Any) -> str:
        """Convert analysis to natural language."""
        formatted = self.to_json(data)
        
        instrument = data.get("instrument", "Unknown")
        timeframe = data.get("timeframe", "1h")
        price = data.get("price", 0)
        overall = formatted.get("overall_signal", "neutral")
        confidence = formatted.get("confidence", 0)
        
        lines = [
            f"Technical Analysis: {instrument} ({timeframe})",
            f"Current Price: ${price:,.2f}",
            "",
        ]
        
        # RSI
        rsi = formatted.get("indicators", {}).get("rsi")
        if rsi:
            lines.append(f"RSI(14): {rsi.get('value', 0):.1f} - {rsi.get('condition', 'neutral')}")
        
        # MACD
        macd = formatted.get("indicators", {}).get("macd")
        if macd:
            lines.append(f"MACD: {macd.get('signal', 'neutral')} (histogram: {macd.get('histogram', 0):.4f})")
        
        # Bollinger
        bb = formatted.get("indicators", {}).get("bollinger")
        if bb:
            lines.append(f"Bollinger: Price {bb.get('position', 'within bands')}")
        
        # Signals
        signals = formatted.get("signals", [])
        if signals:
            lines.append("")
            lines.append("Active Signals:")
            for sig in signals:
                lines.append(f"  - {sig.get('type', 'signal')}: {sig.get('indicator', 'unknown')}")
        
        lines.extend([
            "",
            f"Overall: {overall.upper()} (confidence: {confidence:.0%})",
        ])
        
        return "\n".join(lines)
    
    def _format_rsi(self, rsi: Any) -> Optional[Dict[str, Any]]:
        """Format RSI indicator."""
        if rsi is None:
            return None
        
        if isinstance(rsi, (int, float)):
            value = float(rsi)
            condition = "oversold" if value < 30 else "overbought" if value > 70 else "neutral"
            return {"value": value, "condition": condition}
        
        return rsi
    
    def _format_macd(self, macd: Any) -> Optional[Dict[str, Any]]:
        """Format MACD indicator."""
        if macd is None:
            return None
        
        if isinstance(macd, dict):
            return {
                "macd_line": macd.get("macd", 0),
                "signal_line": macd.get("signal_line", 0),
                "histogram": macd.get("histogram", macd.get("macd", 0)),
                "signal": macd.get("signal", "bullish" if macd.get("macd", 0) > 0 else "bearish"),
            }
        
        return {"value": macd}
    
    def _format_bollinger(self, bb: Any) -> Optional[Dict[str, Any]]:
        """Format Bollinger Bands."""
        if bb is None:
            return None
        
        if isinstance(bb, dict):
            return {
                "upper": bb.get("upper"),
                "middle": bb.get("middle"),
                "lower": bb.get("lower"),
                "position": bb.get("position", "within_bands"),
            }
        
        return bb
    
    def _calculate_confidence(self, signals: List[Dict[str, Any]]) -> float:
        """Calculate confidence based on signal agreement."""
        if not signals:
            return 0.5
        
        bullish = sum(1 for s in signals if s.get("type") == "oversold" or "bullish" in str(s))
        bearish = sum(1 for s in signals if s.get("type") == "overbought" or "bearish" in str(s))
        
        if bullish + bearish == 0:
            return 0.5
        
        return max(bullish, bearish) / (bullish + bearish)


__all__ = [
    "IMBALANCE_THRESHOLD",
    "DIRECTION_CHANGE_PCT",
    "VOLATILITY_HIGH_THRESHOLD",
    "SIGNAL_STRONG_THRESHOLD",
    "SIGNAL_MODERATE_THRESHOLD",
    "DataFormatter",
    "MarketDataFormatter",
    "SignalFormatter",
    "PortfolioFormatter",
    "AnalysisFormatter",
]
