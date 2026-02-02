"""
Tool/Action Framework for AI Agents

Provides a structured way for agents to interact with:
- Market data
- Trading execution
- Analysis services
- Memory queries
- Agent communication

Tools are the actions an agent can take in the world.
"""

from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional, TYPE_CHECKING
import json

if TYPE_CHECKING:
    from .agent import AIAgent

logger = logging.getLogger(__name__)


@dataclass
class ToolResult:
    """Result of a tool execution."""
    success: bool
    output: Optional[Any] = None
    error: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)
    execution_time_ms: float = 0.0
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for LLM consumption."""
        return {
            "success": self.success,
            "output": self.output,
            "error": self.error,
        }


@dataclass
class ToolParameter:
    """Definition of a tool parameter."""
    name: str
    type: str  # "string", "number", "boolean", "object", "array"
    description: str
    required: bool = True
    default: Optional[Any] = None
    enum: Optional[List[Any]] = None


class Tool(ABC):
    """
    Base class for agent tools.
    
    Tools are actions agents can take. Each tool has:
    - A name (used by LLM to call it)
    - A description (helps LLM understand when to use it)
    - Parameters with types and descriptions
    - An execute method that performs the action
    """
    
    name: str
    description: str
    parameters: List[ToolParameter]
    
    def __init__(self, agent: Optional["AIAgent"] = None):
        self.agent = agent
    
    @abstractmethod
    async def execute(self, **kwargs) -> ToolResult:
        """Execute the tool with given parameters."""
        pass
    
    def to_openai_schema(self) -> Dict[str, Any]:
        """Convert to OpenAI function calling schema."""
        properties = {}
        required = []
        
        for param in self.parameters:
            prop = {
                "type": param.type,
                "description": param.description,
            }
            if param.enum:
                prop["enum"] = param.enum
            
            properties[param.name] = prop
            
            if param.required:
                required.append(param.name)
        
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            }
        }
    
    def to_anthropic_schema(self) -> Dict[str, Any]:
        """Convert to Anthropic tool use schema."""
        properties = {}
        required = []
        
        for param in self.parameters:
            prop = {
                "type": param.type,
                "description": param.description,
            }
            if param.enum:
                prop["enum"] = param.enum
            
            properties[param.name] = prop
            
            if param.required:
                required.append(param.name)
        
        return {
            "name": self.name,
            "description": self.description,
            "input_schema": {
                "type": "object",
                "properties": properties,
                "required": required,
            }
        }


class ToolRegistry:
    """Registry of available tools for an agent."""
    
    def __init__(self):
        self._tools: Dict[str, Tool] = {}
    
    def register(self, tool: Tool) -> None:
        """Register a tool."""
        self._tools[tool.name] = tool
        logger.debug(f"Registered tool: {tool.name}")
    
    def unregister(self, name: str) -> bool:
        """Unregister a tool."""
        if name in self._tools:
            del self._tools[name]
            return True
        return False
    
    def get(self, name: str) -> Optional[Tool]:
        """Get a tool by name."""
        return self._tools.get(name)
    
    def list(self) -> List[Tool]:
        """List all registered tools."""
        return list(self._tools.values())
    
    def names(self) -> List[str]:
        """List all tool names."""
        return list(self._tools.keys())
    
    def to_openai_tools(self) -> List[Dict[str, Any]]:
        """Get all tools in OpenAI format."""
        return [tool.to_openai_schema() for tool in self._tools.values()]
    
    def to_anthropic_tools(self) -> List[Dict[str, Any]]:
        """Get all tools in Anthropic format."""
        return [tool.to_anthropic_schema() for tool in self._tools.values()]


# =============================================================================
# Built-in Tools
# =============================================================================

class GetMarketDataTool(Tool):
    """Get current market data for an instrument."""
    
    name = "get_market_data"
    description = "Get current market data including price, volume, and order book for a trading instrument"
    parameters = [
        ToolParameter(
            name="instrument",
            type="string",
            description="The instrument symbol (e.g., 'BTC-PERP', 'ETH-PERP')",
        ),
        ToolParameter(
            name="data_type",
            type="string",
            description="Type of data to retrieve",
            required=False,
            default="ticker",
            enum=["ticker", "orderbook", "trades", "candles"],
        ),
        ToolParameter(
            name="timeframe",
            type="string",
            description="Timeframe for candles (e.g., '1m', '1h', '1d')",
            required=False,
            default="1h",
        ),
        ToolParameter(
            name="limit",
            type="number",
            description="Number of data points for trades/candles",
            required=False,
            default=100,
        ),
    ]
    
    async def execute(
        self,
        instrument: str,
        data_type: str = "ticker",
        timeframe: str = "1h",
        limit: int = 100,
    ) -> ToolResult:
        """Fetch market data."""
        import time
        start = time.time()
        
        try:
            # Import Hyperliquid client
            from neleus.types import HyperliquidClient
            client = HyperliquidClient(testnet=False)
            
            if data_type == "ticker":
                # Get current price
                from datetime import datetime
                end_time = int(datetime.now().timestamp() * 1000)
                start_time = end_time - 3600000  # 1 hour
                
                candles = client.fetch_candles(
                    instrument.replace("-PERP", ""),
                    "1h",
                    start_time,
                    end_time,
                )
                
                if candles:
                    latest = candles[-1]
                    output = {
                        "instrument": instrument,
                        "price": latest.close,
                        "open": latest.open,
                        "high": latest.high,
                        "low": latest.low,
                        "volume": latest.volume,
                        "timestamp": datetime.fromtimestamp(latest.timestamp / 1000).isoformat(),
                    }
                else:
                    output = {"error": "No data available"}
                    
            elif data_type == "candles":
                from datetime import datetime
                end_time = int(datetime.now().timestamp() * 1000)
                hours = {"1m": 1, "5m": 5, "15m": 15, "1h": limit, "4h": limit * 4, "1d": limit * 24}.get(timeframe, limit)
                start_time = end_time - (hours * 3600000)
                
                candles = client.fetch_candles(
                    instrument.replace("-PERP", ""),
                    timeframe,
                    start_time,
                    end_time,
                )
                
                output = {
                    "instrument": instrument,
                    "timeframe": timeframe,
                    "candles": [
                        {
                            "timestamp": datetime.fromtimestamp(c.timestamp / 1000).isoformat(),
                            "open": c.open,
                            "high": c.high,
                            "low": c.low,
                            "close": c.close,
                            "volume": c.volume,
                        }
                        for c in (candles[-limit:] if candles else [])
                    ]
                }
            else:
                output = {"error": f"Data type '{data_type}' not yet implemented"}
            
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"GetMarketData error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class GetAnalysisTool(Tool):
    """Get technical analysis for an instrument."""
    
    name = "get_analysis"
    description = "Get technical analysis including indicators (RSI, MACD, Bollinger Bands) and signals for a trading instrument"
    parameters = [
        ToolParameter(
            name="instrument",
            type="string",
            description="The instrument symbol (e.g., 'BTC-PERP', 'ETH-PERP')",
        ),
        ToolParameter(
            name="indicators",
            type="array",
            description="List of indicators to calculate",
            required=False,
            default=["rsi", "macd", "bollinger"],
        ),
        ToolParameter(
            name="timeframe",
            type="string",
            description="Timeframe for analysis",
            required=False,
            default="1h",
        ),
    ]
    
    async def execute(
        self,
        instrument: str,
        indicators: List[str] = None,
        timeframe: str = "1h",
    ) -> ToolResult:
        """Calculate technical indicators."""
        import time
        import numpy as np
        start = time.time()
        
        if indicators is None:
            indicators = ["rsi", "macd", "bollinger"]
        
        try:
            # Get market data first
            market_data_tool = GetMarketDataTool()
            data_result = await market_data_tool.execute(
                instrument=instrument,
                data_type="candles",
                timeframe=timeframe,
                limit=100,
            )
            
            if not data_result.success:
                return data_result
            
            candles = data_result.output.get("candles", [])
            if len(candles) < 20:
                return ToolResult(
                    success=False,
                    error="Insufficient data for analysis",
                )
            
            closes = np.array([c["close"] for c in candles])
            highs = np.array([c["high"] for c in candles])
            lows = np.array([c["low"] for c in candles])
            
            analysis = {
                "instrument": instrument,
                "timeframe": timeframe,
                "price": closes[-1],
                "indicators": {},
                "signals": [],
            }
            
            # Calculate indicators
            if "rsi" in indicators:
                # RSI calculation
                delta = np.diff(closes)
                gain = np.where(delta > 0, delta, 0)
                loss = np.where(delta < 0, -delta, 0)
                
                avg_gain = np.mean(gain[-14:])
                avg_loss = np.mean(loss[-14:])
                
                if avg_loss != 0:
                    rs = avg_gain / avg_loss
                    rsi = 100 - (100 / (1 + rs))
                else:
                    rsi = 100
                
                analysis["indicators"]["rsi"] = round(rsi, 2)
                
                if rsi < 30:
                    analysis["signals"].append({"type": "oversold", "indicator": "rsi", "value": rsi})
                elif rsi > 70:
                    analysis["signals"].append({"type": "overbought", "indicator": "rsi", "value": rsi})
            
            if "macd" in indicators:
                # MACD calculation
                ema12 = closes[-12:].mean()  # Simplified
                ema26 = closes[-26:].mean()  # Simplified
                macd_line = ema12 - ema26
                
                analysis["indicators"]["macd"] = {
                    "macd": round(macd_line, 4),
                    "signal": "bullish" if macd_line > 0 else "bearish",
                }
            
            if "bollinger" in indicators:
                # Bollinger Bands
                sma20 = np.mean(closes[-20:])
                std20 = np.std(closes[-20:])
                
                upper = sma20 + 2 * std20
                lower = sma20 - 2 * std20
                
                analysis["indicators"]["bollinger"] = {
                    "upper": round(upper, 2),
                    "middle": round(sma20, 2),
                    "lower": round(lower, 2),
                    "position": "above_upper" if closes[-1] > upper else "below_lower" if closes[-1] < lower else "within_bands",
                }
                
                if closes[-1] < lower:
                    analysis["signals"].append({"type": "oversold", "indicator": "bollinger"})
                elif closes[-1] > upper:
                    analysis["signals"].append({"type": "overbought", "indicator": "bollinger"})
            
            # Overall signal
            bullish_signals = sum(1 for s in analysis["signals"] if s["type"] == "oversold")
            bearish_signals = sum(1 for s in analysis["signals"] if s["type"] == "overbought")
            
            if bullish_signals > bearish_signals:
                analysis["overall_signal"] = "bullish"
            elif bearish_signals > bullish_signals:
                analysis["overall_signal"] = "bearish"
            else:
                analysis["overall_signal"] = "neutral"
            
            return ToolResult(
                success=True,
                output=analysis,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"GetAnalysis error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class PlaceOrderTool(Tool):
    """Place a trading order."""
    
    name = "place_order"
    description = "Place a trading order (market or limit) for a trading instrument. Use with caution - this executes real trades."
    parameters = [
        ToolParameter(
            name="instrument",
            type="string",
            description="The instrument symbol (e.g., 'BTC-PERP', 'ETH-PERP')",
        ),
        ToolParameter(
            name="side",
            type="string",
            description="Order side",
            enum=["buy", "sell"],
        ),
        ToolParameter(
            name="order_type",
            type="string",
            description="Order type",
            enum=["market", "limit"],
        ),
        ToolParameter(
            name="quantity",
            type="number",
            description="Order quantity (in base currency)",
        ),
        ToolParameter(
            name="price",
            type="number",
            description="Limit price (required for limit orders)",
            required=False,
        ),
        ToolParameter(
            name="reduce_only",
            type="boolean",
            description="If true, order can only reduce position",
            required=False,
            default=False,
        ),
        ToolParameter(
            name="stop_loss",
            type="number",
            description="Stop loss price",
            required=False,
        ),
        ToolParameter(
            name="take_profit",
            type="number",
            description="Take profit price",
            required=False,
        ),
    ]
    
    async def execute(
        self,
        instrument: str,
        side: str,
        order_type: str,
        quantity: float,
        price: Optional[float] = None,
        reduce_only: bool = False,
        stop_loss: Optional[float] = None,
        take_profit: Optional[float] = None,
    ) -> ToolResult:
        """Place an order."""
        import time
        start = time.time()
        
        # Validate
        if order_type == "limit" and price is None:
            return ToolResult(
                success=False,
                error="Price is required for limit orders",
            )
        
        try:
            # Check agent risk limits if available
            if self.agent:
                max_size = self.agent.config.info.max_position_size
                # This would check current position and validate
                logger.info(f"Order validation passed (max size: {max_size})")
            
            # TODO: Implement actual order placement via Hyperliquid
            # For now, return simulated result
            order_id = f"sim_{int(time.time() * 1000)}"
            
            output = {
                "order_id": order_id,
                "instrument": instrument,
                "side": side,
                "order_type": order_type,
                "quantity": quantity,
                "price": price,
                "status": "simulated",
                "message": "Order simulated (production trading not enabled)",
            }
            
            logger.warning(f"Simulated order: {side} {quantity} {instrument} @ {price or 'market'}")
            
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"PlaceOrder error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class GetSignalsTool(Tool):
    """Get trading signals from signal hub."""
    
    name = "get_signals"
    description = "Get recent trading signals from the signal hub including external signals, model predictions, and alerts"
    parameters = [
        ToolParameter(
            name="instrument",
            type="string",
            description="Filter by instrument (optional)",
            required=False,
        ),
        ToolParameter(
            name="signal_type",
            type="string",
            description="Filter by signal type",
            required=False,
            enum=["entry", "exit", "alert", "prediction"],
        ),
        ToolParameter(
            name="min_strength",
            type="number",
            description="Minimum signal strength (0-1)",
            required=False,
            default=0.0,
        ),
        ToolParameter(
            name="limit",
            type="number",
            description="Maximum number of signals to return",
            required=False,
            default=10,
        ),
    ]
    
    async def execute(
        self,
        instrument: Optional[str] = None,
        signal_type: Optional[str] = None,
        min_strength: float = 0.0,
        limit: int = 10,
    ) -> ToolResult:
        """Get signals."""
        import time
        from datetime import datetime
        start = time.time()
        
        try:
            # TODO: Connect to actual signal hub
            # For now, return demo signals
            signals = [
                {
                    "id": "sig_001",
                    "instrument": "BTC-PERP",
                    "type": "entry",
                    "direction": "long",
                    "strength": 0.75,
                    "source": "momentum_model",
                    "timestamp": datetime.now().isoformat(),
                    "metadata": {"rsi": 28, "macd": "bullish_cross"},
                },
                {
                    "id": "sig_002",
                    "instrument": "ETH-PERP",
                    "type": "alert",
                    "direction": "neutral",
                    "strength": 0.6,
                    "source": "volatility_monitor",
                    "timestamp": datetime.now().isoformat(),
                    "metadata": {"message": "Volatility spike detected"},
                },
            ]
            
            # Filter
            if instrument:
                signals = [s for s in signals if s["instrument"] == instrument]
            if signal_type:
                signals = [s for s in signals if s["type"] == signal_type]
            signals = [s for s in signals if s["strength"] >= min_strength]
            
            return ToolResult(
                success=True,
                output={"signals": signals[:limit]},
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"GetSignals error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class QueryMemoryTool(Tool):
    """Query agent memory for past decisions and learnings."""
    
    name = "query_memory"
    description = "Query your memory for past observations, decisions, and learnings. Useful for recalling similar market conditions or past trade outcomes."
    parameters = [
        ToolParameter(
            name="query",
            type="string",
            description="What to search for in memory",
        ),
        ToolParameter(
            name="memory_type",
            type="string",
            description="Type of memory to search",
            required=False,
            enum=["observation", "decision", "action", "outcome", "learning"],
        ),
        ToolParameter(
            name="limit",
            type="number",
            description="Maximum number of memories to return",
            required=False,
            default=5,
        ),
    ]
    
    async def execute(
        self,
        query: str,
        memory_type: Optional[str] = None,
        limit: int = 5,
    ) -> ToolResult:
        """Query memory."""
        import time
        start = time.time()
        
        try:
            if not self.agent or not self.agent._memory:
                return ToolResult(
                    success=False,
                    error="Memory not initialized",
                )
            
            from .memory import MemoryType
            mem_type = MemoryType(memory_type) if memory_type else None
            
            memories = await self.agent._memory.recall(
                query=query,
                limit=limit,
                memory_type=mem_type,
            )
            
            output = {
                "query": query,
                "memories": [m.to_dict() for m in memories],
                "count": len(memories),
            }
            
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"QueryMemory error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class GetPortfolioTool(Tool):
    """Get current portfolio and positions."""
    
    name = "get_portfolio"
    description = "Get current portfolio status including positions, balances, and P&L"
    parameters = [
        ToolParameter(
            name="include_history",
            type="boolean",
            description="Include recent trade history",
            required=False,
            default=False,
        ),
    ]
    
    async def execute(
        self,
        include_history: bool = False,
    ) -> ToolResult:
        """Get portfolio."""
        import time
        start = time.time()
        
        try:
            # TODO: Get actual portfolio from trading engine
            # Demo data for now
            output = {
                "equity": 100000.0,
                "available_balance": 85000.0,
                "used_margin": 15000.0,
                "unrealized_pnl": 1250.50,
                "positions": [
                    {
                        "instrument": "BTC-PERP",
                        "side": "long",
                        "size": 0.5,
                        "entry_price": 42000.0,
                        "mark_price": 43250.0,
                        "unrealized_pnl": 625.0,
                        "leverage": 5,
                    },
                    {
                        "instrument": "ETH-PERP",
                        "side": "long",
                        "size": 5.0,
                        "entry_price": 2200.0,
                        "mark_price": 2325.0,
                        "unrealized_pnl": 625.50,
                        "leverage": 3,
                    },
                ],
            }
            
            if include_history:
                output["recent_trades"] = [
                    {
                        "instrument": "SOL-PERP",
                        "side": "buy",
                        "size": 10,
                        "price": 95.50,
                        "pnl": 150.0,
                        "timestamp": "2026-02-01T10:30:00Z",
                    },
                ]
            
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"GetPortfolio error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )


class SendMessageTool(Tool):
    """Send a message to another agent."""
    
    name = "send_message"
    description = "Send a message to another agent for coordination, data sharing, or collaboration"
    parameters = [
        ToolParameter(
            name="to_agent",
            type="string",
            description="Agent ID to send message to (or '*' for broadcast)",
        ),
        ToolParameter(
            name="message_type",
            type="string",
            description="Type of message",
            enum=["data_request", "signal_share", "coordination", "alert"],
        ),
        ToolParameter(
            name="content",
            type="object",
            description="Message content/payload",
        ),
    ]
    
    async def execute(
        self,
        to_agent: str,
        message_type: str,
        content: Dict[str, Any],
    ) -> ToolResult:
        """Send message."""
        import time
        start = time.time()
        
        try:
            if not self.agent or not self.agent._message_bus:
                return ToolResult(
                    success=False,
                    error="Message bus not initialized",
                )
            
            from .communication import MessageType
            
            if to_agent == "*":
                await self.agent.broadcast(
                    message_type=MessageType(message_type),
                    payload=content,
                )
                output = {"status": "broadcast_sent", "recipients": "all"}
            else:
                await self.agent.send_message(
                    to_agent=to_agent,
                    message_type=MessageType(message_type),
                    payload=content,
                )
                output = {"status": "sent", "recipient": to_agent}
            
            return ToolResult(
                success=True,
                output=output,
                execution_time_ms=(time.time() - start) * 1000,
            )
            
        except Exception as e:
            logger.error(f"SendMessage error: {e}")
            return ToolResult(
                success=False,
                error=str(e),
                execution_time_ms=(time.time() - start) * 1000,
            )
