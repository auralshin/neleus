"""
RSI Mean Reversion AI Agent
===========================

An intelligent trading agent that uses RSI analysis with LLM-powered reasoning
to make mean reversion trading decisions.

This demonstrates the Neleus Agent Orchestrator:
- Memory persistence for learning from past decisions
- Tool framework for market data and execution
- LLM-powered reasoning for trading decisions
- Communication with other agents

Usage:
    # Create the agent project
    neleus new-agent rsi_agent
    
    # Copy this file as main.py
    cp examples/rsi_ai_agent.py rsi_agent/main.py
    
    # Run the agent
    neleus agent run rsi_agent/

Author: Neleus Team
Motto: "Make Your Agent Trade Smarter"
"""

import asyncio
import logging
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

# Set up logging first
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("RSIAgent")

# =============================================================================
# Imports from Neleus AI Framework
# =============================================================================

try:
    from neleus.ai import (
        AIAgent,
        AgentConfig,
        PersonalityConfig,
        InfoConfig,
        AgentState,
        MemoryType,
        MemoryEntry,
        ToolResult,
        MessageType,
        AgentMessage,
        MarketDataFormatter,
        AnalysisFormatter,
    )
except ImportError:
    logger.error("Neleus AI framework not installed. Run: pip install neleus")
    raise


# =============================================================================
# RSI Mean Reversion Agent
# =============================================================================

class RSIMeanReversionAgent(AIAgent):
    """
    RSI Mean Reversion Agent - An AI-powered trading agent
    
    Strategy Logic:
    - Uses RSI (14) to identify overbought/oversold conditions
    - RSI < 30: Oversold → Look for long opportunities
    - RSI > 70: Overbought → Look for short/exit opportunities
    
    AI Enhancement:
    - Uses LLM reasoning to evaluate market context
    - Remembers past decisions and their outcomes
    - Learns from mistakes through memory analysis
    """
    
    # Custom parameters
    RSI_OVERSOLD = 30
    RSI_OVERBOUGHT = 70
    POSITION_SIZE_PCT = 0.05  # 5% of portfolio per trade
    
    def __init__(self, config: AgentConfig):
        super().__init__(config)
        
        # Agent state
        self._current_position: Optional[str] = None  # 'long', 'short', or None
        self._entry_price: Optional[float] = None
        self._trade_count = 0
        self._win_count = 0
        self._loss_count = 0
    
    async def on_start(self) -> None:
        """Called when the agent starts."""
        logger.info(f"🚀 {self.name} starting up...")
        
        # Load past performance from memory
        past_outcomes = await self.recall(
            "trade outcomes",
            limit=50,
            memory_type=MemoryType.OUTCOME,
        )
        
        if past_outcomes:
            # Calculate past performance
            wins = sum(1 for o in past_outcomes if "profit" in o.content.lower())
            losses = sum(1 for o in past_outcomes if "loss" in o.content.lower())
            
            logger.info(f"📊 Loaded past performance: {wins} wins, {losses} losses")
            
            await self.remember(
                f"Loaded past performance on startup: {wins} wins, {losses} losses",
                memory_type=MemoryType.CONTEXT,
            )
        
        # Remember startup
        await self.remember(
            f"Agent started. RSI thresholds: oversold={self.RSI_OVERSOLD}, "
            f"overbought={self.RSI_OVERBOUGHT}",
            memory_type=MemoryType.CONTEXT,
        )
        
        logger.info(f"✅ {self.name} ready to trade!")
    
    async def on_stop(self) -> None:
        """Called when the agent stops."""
        logger.info(f"🛑 {self.name} shutting down...")
        
        # Save final statistics
        if self._trade_count > 0:
            win_rate = (self._win_count / self._trade_count) * 100
            await self.remember(
                f"Session summary: {self._trade_count} trades, "
                f"{self._win_count} wins, {self._loss_count} losses, "
                f"{win_rate:.1f}% win rate",
                memory_type=MemoryType.OUTCOME,
            )
        
        logger.info(f"💾 State saved to memory")
    
    async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Make a trading decision based on RSI and LLM reasoning.
        
        This is where the magic happens:
        1. Get market data and RSI
        2. Check current position
        3. Use LLM to reason about the situation
        4. Execute trade if appropriate
        """
        instrument = context.get("instrument", "BTC-PERP")
        
        # =====================================================================
        # Step 1: Gather Market Data
        # =====================================================================
        
        market_result = await self.execute_tool(
            "get_market_data",
            instrument=instrument,
            data_type="ticker",
        )
        
        if not market_result.success:
            logger.warning(f"⚠️ Failed to get market data: {market_result.error}")
            return {"action": "wait", "reason": "No market data available"}
        
        current_price = market_result.output.get("price", 0)
        
        # =====================================================================
        # Step 2: Get Technical Analysis
        # =====================================================================
        
        analysis_result = await self.execute_tool(
            "get_analysis",
            instrument=instrument,
            indicators=["rsi", "macd", "bollinger"],
        )
        
        if not analysis_result.success:
            logger.warning(f"⚠️ Failed to get analysis: {analysis_result.error}")
            return {"action": "wait", "reason": "No analysis available"}
        
        indicators = analysis_result.output.get("indicators", {})
        rsi = indicators.get("rsi", 50)  # Default to neutral
        macd = indicators.get("macd", {})
        bollinger = indicators.get("bollinger", {})
        
        logger.info(f"📈 {instrument}: Price=${current_price:.2f}, RSI={rsi:.1f}")
        
        # =====================================================================
        # Step 3: Check Current Portfolio
        # =====================================================================
        
        portfolio_result = await self.execute_tool("get_portfolio")
        portfolio = portfolio_result.output if portfolio_result.success else {}
        
        equity = portfolio.get("equity", 100000)
        unrealized_pnl = portfolio.get("unrealized_pnl", 0)
        
        # =====================================================================
        # Step 4: Recall Similar Situations
        # =====================================================================
        
        similar_situations = await self.recall(
            f"RSI near {int(rsi)} for {instrument}",
            limit=5,
            memory_type=MemoryType.DECISION,
        )
        
        past_context = ""
        if similar_situations:
            past_context = "\n".join([
                f"- {entry.content[:100]}..." 
                for entry in similar_situations[:3]
            ])
        
        # =====================================================================
        # Step 5: LLM Reasoning
        # =====================================================================
        
        # Format data for LLM consumption
        market_text = MarketDataFormatter.format_ticker(market_result.output)
        analysis_text = AnalysisFormatter.format_full_analysis(analysis_result.output)
        
        reasoning_prompt = f"""
You are an RSI Mean Reversion trading agent analyzing {instrument}.

**Current Market State:**
{market_text}

**Technical Analysis:**
{analysis_text}

**RSI Strategy Rules:**
- RSI < {self.RSI_OVERSOLD}: Oversold (potential buy zone)
- RSI > {self.RSI_OVERBOUGHT}: Overbought (potential sell zone)
- Current RSI: {rsi:.1f}

**Current Position:**
- Position: {self._current_position or 'None'}
- Entry Price: ${self._entry_price or 'N/A'}
- Unrealized P&L: ${unrealized_pnl:.2f}

**Portfolio:**
- Equity: ${equity:.2f}
- Max position size: {self.POSITION_SIZE_PCT * 100:.0f}% = ${equity * self.POSITION_SIZE_PCT:.2f}

**Past Similar Situations:**
{past_context or 'No similar situations in memory'}

**Your Task:**
Based on the RSI mean reversion strategy and the current market conditions:
1. Should we BUY (go long), SELL (close/reverse), or HOLD?
2. What is your confidence level (low/medium/high)?
3. If trading, what size (as fraction of max)?
4. What is your key reasoning?

Be conservative - only trade when RSI gives a clear signal.
"""

        reasoning = await self.think(reasoning_prompt)
        
        logger.debug(f"🤖 LLM Reasoning: {reasoning[:200]}...")
        
        # =====================================================================
        # Step 6: Parse Decision and Execute
        # =====================================================================
        
        decision = self._parse_decision(reasoning, rsi, current_price, equity)
        
        # Execute the trade if needed
        if decision["action"] in ["buy", "sell"]:
            await self._execute_trade(
                decision, 
                instrument, 
                current_price, 
                reasoning
            )
        
        # Remember the decision
        await self.remember(
            f"Decision: {decision['action']} for {instrument} at ${current_price:.2f}. "
            f"RSI={rsi:.1f}. Confidence: {decision.get('confidence', 'unknown')}. "
            f"Reasoning: {decision.get('reason', reasoning[:100])}",
            memory_type=MemoryType.DECISION,
            metadata={
                "instrument": instrument,
                "price": current_price,
                "rsi": rsi,
                "action": decision["action"],
                "confidence": decision.get("confidence"),
            },
        )
        
        self._decision_count += 1
        
        return decision
    
    def _parse_decision(
        self, 
        reasoning: str, 
        rsi: float, 
        price: float, 
        equity: float
    ) -> Dict[str, Any]:
        """
        Parse the LLM reasoning to extract a trading decision.
        
        Falls back to rule-based decision if LLM response is unclear.
        """
        reasoning_lower = reasoning.lower()
        
        # Try to extract action from LLM response
        action = "hold"
        confidence = "medium"
        size = 0.0
        reason = ""
        
        if "buy" in reasoning_lower and "don't buy" not in reasoning_lower:
            action = "buy"
        elif "sell" in reasoning_lower and "don't sell" not in reasoning_lower:
            action = "sell"
        elif "hold" in reasoning_lower or "wait" in reasoning_lower:
            action = "hold"
        
        # Extract confidence
        if "high confidence" in reasoning_lower or "strongly" in reasoning_lower:
            confidence = "high"
        elif "low confidence" in reasoning_lower or "uncertain" in reasoning_lower:
            confidence = "low"
        
        # Fall back to rule-based if LLM is unclear
        if action == "hold":
            if rsi < self.RSI_OVERSOLD and self._current_position != "long":
                action = "buy"
                reason = f"RSI ({rsi:.1f}) below oversold threshold ({self.RSI_OVERSOLD})"
            elif rsi > self.RSI_OVERBOUGHT and self._current_position == "long":
                action = "sell"
                reason = f"RSI ({rsi:.1f}) above overbought threshold ({self.RSI_OVERBOUGHT})"
        
        # Calculate position size
        if action in ["buy", "sell"]:
            base_size = equity * self.POSITION_SIZE_PCT
            
            # Adjust by confidence
            confidence_multiplier = {"low": 0.5, "medium": 1.0, "high": 1.5}
            size = base_size * confidence_multiplier.get(confidence, 1.0)
        
        return {
            "action": action,
            "confidence": confidence,
            "size": size,
            "reason": reason or reasoning[:200],
            "rsi": rsi,
            "price": price,
        }
    
    async def _execute_trade(
        self,
        decision: Dict[str, Any],
        instrument: str,
        price: float,
        reasoning: str,
    ) -> None:
        """Execute a trade based on the decision."""
        action = decision["action"]
        size = decision["size"]
        
        if action == "buy" and self._current_position != "long":
            # Enter long position
            result = await self.execute_tool(
                "place_order",
                instrument=instrument,
                side="buy",
                order_type="market",
                size=size,
            )
            
            if result.success:
                self._current_position = "long"
                self._entry_price = price
                self._trade_count += 1
                
                logger.info(f"✅ Opened LONG position: {size:.4f} @ ${price:.2f}")
                
                await self.remember(
                    f"Opened LONG: {size:.4f} {instrument} @ ${price:.2f}. {reasoning[:100]}",
                    memory_type=MemoryType.ACTION,
                    metadata={"order_id": result.output.get("order_id")},
                )
            else:
                logger.error(f"❌ Failed to place order: {result.error}")
        
        elif action == "sell" and self._current_position == "long":
            # Close long position
            result = await self.execute_tool(
                "place_order",
                instrument=instrument,
                side="sell",
                order_type="market",
                size=size,
            )
            
            if result.success:
                # Calculate P&L
                pnl = (price - self._entry_price) * size if self._entry_price else 0
                
                if pnl > 0:
                    self._win_count += 1
                    outcome = "profit"
                else:
                    self._loss_count += 1
                    outcome = "loss"
                
                logger.info(f"✅ Closed LONG position: ${pnl:+.2f} ({outcome})")
                
                await self.remember(
                    f"Closed LONG: {size:.4f} {instrument} @ ${price:.2f}. "
                    f"P&L: ${pnl:+.2f} ({outcome})",
                    memory_type=MemoryType.OUTCOME,
                    metadata={
                        "pnl": pnl,
                        "entry_price": self._entry_price,
                        "exit_price": price,
                        "outcome": outcome,
                    },
                )
                
                self._current_position = None
                self._entry_price = None
            else:
                logger.error(f"❌ Failed to place order: {result.error}")
    
    async def on_market_data(self, data: Dict[str, Any]) -> None:
        """Handle incoming market data updates."""
        # Log significant price moves
        price_change = data.get("price_change_pct", 0)
        
        if abs(price_change) > 3:  # More than 3% move
            await self.remember(
                f"Significant price move: {price_change:+.1f}% for {data.get('instrument')}",
                memory_type=MemoryType.OBSERVATION,
                metadata={"importance": 0.7},
            )
    
    async def on_signal(self, signal: Dict[str, Any]) -> None:
        """Handle incoming trading signals."""
        logger.info(f"📡 Received signal: {signal}")
        
        if signal.get("strength", 0) > 0.8:
            await self.remember(
                f"Strong signal received: {signal.get('direction')} {signal.get('instrument')}",
                memory_type=MemoryType.OBSERVATION,
                metadata={"signal": signal, "importance": 0.9},
            )
    
    async def on_message(self, message: AgentMessage) -> None:
        """Handle messages from other agents."""
        logger.info(f"📬 Message from {message.from_agent}: {message.message_type.value}")
        
        if message.message_type == MessageType.ALERT:
            # Handle alerts (e.g., risk warnings from other agents)
            alert = message.payload.get("message", "Unknown alert")
            
            await self.remember(
                f"Alert from {message.from_agent}: {alert}",
                memory_type=MemoryType.OBSERVATION,
                metadata={"importance": 1.0, "source": message.from_agent},
            )


# =============================================================================
# Main Entry Point
# =============================================================================

async def main():
    """Run the RSI Mean Reversion Agent."""
    print("""
╔═══════════════════════════════════════════════════════════════╗
║           🌊 Neleus Agent Orchestrator Service 🌊              ║
║              Make Your Agent Trade Smarter                     ║
╠═══════════════════════════════════════════════════════════════╣
║  Agent: RSI Mean Reversion                                     ║
║  Strategy: Buy oversold, sell overbought                       ║
╚═══════════════════════════════════════════════════════════════╝
    """)
    
    # Load configuration from project directory
    project_path = Path(__file__).parent
    
    try:
        config = AgentConfig.from_project(project_path)
    except FileNotFoundError:
        # Create default config for standalone run
        logger.warning("No personality.yaml found, using default configuration")
        
        config = AgentConfig(
            personality=PersonalityConfig(
                name="RSI Mean Reversion Agent",
                description="An AI agent that trades RSI mean reversion",
                trading_style="balanced",
                risk_tolerance="medium",
                decision_speed="adaptive",
                traits=["analytical", "data-driven", "risk-aware"],
                use_technical_analysis=True,
                use_fundamental_analysis=False,
                use_sentiment_analysis=False,
            ),
            info=InfoConfig(
                llm_provider="openai",
                llm_model="gpt-4o",
                tools=["get_market_data", "get_analysis", "place_order", "get_portfolio"],
                instruments=["BTC-PERP", "ETH-PERP"],
                venues=["hyperliquid"],
                max_position_size=0.10,
                max_daily_loss=0.05,
                decision_interval_seconds=60,
            ),
        )
    
    # Create and start the agent
    agent = RSIMeanReversionAgent(config)
    
    try:
        await agent.start()
        
        logger.info("🏁 Starting decision loop...")
        logger.info("Press Ctrl+C to stop")
        
        # Main decision loop
        instruments = config.info.instruments or ["BTC-PERP"]
        
        while agent.is_running:
            try:
                for instrument in instruments:
                    context = {
                        "instrument": instrument,
                        "timestamp": datetime.now().isoformat(),
                    }
                    
                    decision = await agent.decide(context)
                    
                    logger.info(
                        f"📊 {instrument}: {decision['action'].upper()} "
                        f"(confidence: {decision.get('confidence', 'unknown')})"
                    )
                
                # Wait for next decision interval
                await asyncio.sleep(config.info.decision_interval_seconds)
                
            except KeyboardInterrupt:
                logger.info("🛑 Stopping agent...")
                break
            except Exception as e:
                logger.error(f"❌ Error in decision loop: {e}")
                await agent.on_error(e)
                await asyncio.sleep(10)  # Back off on error
    
    finally:
        await agent.stop()
        logger.info("👋 Agent stopped. Goodbye!")


if __name__ == "__main__":
    asyncio.run(main())
