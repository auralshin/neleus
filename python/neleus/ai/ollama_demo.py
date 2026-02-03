"""
Ollama Trading Agent Demo

A demonstration agent that uses Ollama (local LLM) to:
- Analyze market conditions
- Run backtests
- Monitor volatility
- Make trading decisions

This showcases how Neleus tools can be used by AI agents.
"""

from __future__ import annotations

import asyncio
import json
import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Dict, List, Optional, Callable
from pathlib import Path
import time

from .agent import AIAgent, AgentConfig, PersonalityConfig, InfoConfig, AgentState
from .llm import OllamaProvider, Message, CompletionResult
from .tools import Tool, ToolRegistry, ToolResult
from .memory import LocalMemoryStore, MemoryEntry, MemoryType
from .demo_tools import (
    ListMarketsTool,
    RunBacktestTool,
    MonitorVolatilityTool,
    GetMarketRegimeTool,
    CalculateRiskMetricsTool,
)

logger = logging.getLogger(__name__)


@dataclass
class AgentAction:
    """Record of an agent action for logging/visualization."""
    timestamp: datetime
    action_type: str  # "thinking", "tool_call", "decision", "observation"
    tool_name: Optional[str] = None
    input_data: Optional[Dict[str, Any]] = None
    output_data: Optional[Any] = None
    reasoning: Optional[str] = None
    duration_ms: float = 0.0
    success: bool = True
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "timestamp": self.timestamp.isoformat(),
            "action_type": self.action_type,
            "tool_name": self.tool_name,
            "input_data": self.input_data,
            "output_data": self.output_data,
            "reasoning": self.reasoning,
            "duration_ms": self.duration_ms,
            "success": self.success,
            "error": self.error,
        }


class ActionLogger:
    """
    Logs and visualizes agent actions for demo purposes.
    
    Provides:
    - Real-time console output with colors
    - JSON log files for later analysis
    - Action history for visualization
    """
    
    def __init__(
        self,
        agent_name: str,
        log_dir: Optional[Path] = None,
        console_output: bool = True,
        use_colors: bool = True,
    ):
        self.agent_name = agent_name
        self.log_dir = log_dir or Path("logs")
        self.console_output = console_output
        self.use_colors = use_colors
        self.actions: List[AgentAction] = []
        self.start_time = datetime.now()
        
        # Create log directory
        self.log_dir.mkdir(parents=True, exist_ok=True)
        
        # Log file
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.log_file = self.log_dir / f"agent_{agent_name}_{timestamp}.json"
        
        # Colors for terminal
        self.colors = {
            "reset": "\033[0m",
            "bold": "\033[1m",
            "thinking": "\033[94m",  # Blue
            "tool_call": "\033[92m",  # Green
            "decision": "\033[93m",  # Yellow
            "observation": "\033[96m",  # Cyan
            "error": "\033[91m",  # Red
            "success": "\033[92m",  # Green
            "header": "\033[95m",  # Magenta
        }
    
    def _color(self, text: str, color: str) -> str:
        """Apply color if enabled."""
        if self.use_colors:
            return f"{self.colors.get(color, '')}{text}{self.colors['reset']}"
        return text
    
    def _format_output(self, action: AgentAction) -> str:
        """Format action for console output."""
        lines = []
        
        # Header with timestamp
        elapsed = (action.timestamp - self.start_time).total_seconds()
        header = f"[{elapsed:6.1f}s] {action.action_type.upper()}"
        lines.append(self._color(f"\n{'─' * 60}", "header"))
        lines.append(self._color(f"│ {header}", "header"))
        lines.append(self._color(f"{'─' * 60}", "header"))
        
        # Content based on action type
        if action.action_type == "thinking":
            lines.append(self._color("🧠 Agent Reasoning:", "thinking"))
            if action.reasoning:
                for line in action.reasoning.split("\n")[:10]:  # Limit lines
                    lines.append(f"   {line[:100]}")
                    
        elif action.action_type == "tool_call":
            lines.append(self._color(f"🔧 Tool: {action.tool_name}", "tool_call"))
            if action.input_data:
                lines.append(f"   Input: {json.dumps(action.input_data, indent=2)[:200]}")
            if action.success:
                lines.append(self._color(f"   ✓ Success ({action.duration_ms:.1f}ms)", "success"))
                if action.output_data:
                    output_str = json.dumps(action.output_data, indent=2)[:300]
                    lines.append(f"   Output: {output_str}")
            else:
                lines.append(self._color(f"   ✗ Failed: {action.error}", "error"))
                
        elif action.action_type == "decision":
            lines.append(self._color("📊 Decision Made:", "decision"))
            if action.output_data:
                lines.append(f"   {json.dumps(action.output_data, indent=2)[:300]}")
                
        elif action.action_type == "observation":
            lines.append(self._color("👁 Observation:", "observation"))
            if action.output_data:
                lines.append(f"   {str(action.output_data)[:300]}")
        
        return "\n".join(lines)
    
    def log(self, action: AgentAction) -> None:
        """Log an action."""
        self.actions.append(action)
        
        # Console output
        if self.console_output:
            print(self._format_output(action))
        
        # Append to log file
        self._write_log()
    
    def log_thinking(self, reasoning: str) -> None:
        """Log agent thinking/reasoning."""
        self.log(AgentAction(
            timestamp=datetime.now(),
            action_type="thinking",
            reasoning=reasoning,
        ))
    
    def log_tool_call(
        self,
        tool_name: str,
        inputs: Dict[str, Any],
        result: ToolResult,
        duration_ms: float,
    ) -> None:
        """Log a tool execution."""
        self.log(AgentAction(
            timestamp=datetime.now(),
            action_type="tool_call",
            tool_name=tool_name,
            input_data=inputs,
            output_data=result.output if result.success else None,
            duration_ms=duration_ms,
            success=result.success,
            error=result.error,
        ))
    
    def log_decision(self, decision: Dict[str, Any], reasoning: str = "") -> None:
        """Log a decision made by the agent."""
        self.log(AgentAction(
            timestamp=datetime.now(),
            action_type="decision",
            output_data=decision,
            reasoning=reasoning,
        ))
    
    def log_observation(self, observation: Any) -> None:
        """Log an observation."""
        self.log(AgentAction(
            timestamp=datetime.now(),
            action_type="observation",
            output_data=observation,
        ))
    
    def _write_log(self) -> None:
        """Write actions to log file."""
        log_data = {
            "agent_name": self.agent_name,
            "start_time": self.start_time.isoformat(),
            "actions": [a.to_dict() for a in self.actions],
        }
        with open(self.log_file, "w") as f:
            json.dump(log_data, f, indent=2)
    
    def get_summary(self) -> Dict[str, Any]:
        """Get a summary of all actions."""
        tool_calls = [a for a in self.actions if a.action_type == "tool_call"]
        decisions = [a for a in self.actions if a.action_type == "decision"]
        
        return {
            "agent_name": self.agent_name,
            "total_actions": len(self.actions),
            "tool_calls": len(tool_calls),
            "decisions": len(decisions),
            "successful_tools": sum(1 for t in tool_calls if t.success),
            "failed_tools": sum(1 for t in tool_calls if not t.success),
            "total_duration_s": (datetime.now() - self.start_time).total_seconds(),
            "tools_used": list(set(t.tool_name for t in tool_calls if t.tool_name)),
        }
    
    def print_summary(self) -> None:
        """Print a summary to console."""
        summary = self.get_summary()
        print(self._color("\n" + "=" * 60, "header"))
        print(self._color("│ AGENT SESSION SUMMARY", "header"))
        print(self._color("=" * 60, "header"))
        print(f"  Agent: {summary['agent_name']}")
        print(f"  Duration: {summary['total_duration_s']:.1f}s")
        print(f"  Total Actions: {summary['total_actions']}")
        print(f"  Tool Calls: {summary['tool_calls']} ({summary['successful_tools']} successful)")
        print(f"  Decisions Made: {summary['decisions']}")
        print(f"  Tools Used: {', '.join(summary['tools_used'])}")
        print(self._color("=" * 60 + "\n", "header"))


class OllamaTradingAgent(AIAgent):
    """
    Trading agent powered by Ollama (local LLM).
    
    Demonstrates:
    - Market analysis using Neleus tools
    - Backtesting strategies
    - Volatility monitoring
    - Risk-aware decision making
    
    All heavy lifting is done by Rust core, agent orchestrates via Python.
    """
    
    def __init__(
        self,
        name: str = "OllamaTrader",
        model: str = "llama3.2",
        base_url: str = "http://localhost:11434",
        instruments: List[str] = None,
        log_actions: bool = True,
    ):
        # Create default config
        personality = PersonalityConfig(
            name=name,
            description="An AI trading agent powered by Ollama for market analysis and backtesting",
            trading_style="balanced",
            risk_tolerance="medium",
            decision_speed="deliberate",
            traits=["analytical", "data-driven", "risk-aware"],
            use_technical_analysis=True,
            use_fundamental_analysis=False,
            use_sentiment_analysis=False,
        )
        
        info = InfoConfig(
            version="1.0.0",
            tools=[
                "get_market_data",
                "get_analysis",
                "run_backtest",
                "monitor_volatility",
                "get_market_regime",
                "calculate_risk_metrics",
            ],
            instruments=instruments or ["BTC", "ETH"],
            venues=["hyperliquid"],
            llm_provider="ollama",
            llm_model=model,
            memory_backend="local",
            max_position_size=0.1,
            max_daily_loss=0.05,
        )
        
        config = AgentConfig(
            personality=personality,
            info=info,
            project_path=Path("."),
        )
        
        # Initialize LLM provider
        llm = OllamaProvider(
            model=model,
            base_url=base_url,
            temperature=0.7,
            max_tokens=4096,
        )
        
        # Initialize memory
        memory = LocalMemoryStore()
        
        super().__init__(
            config=config,
            llm_provider=llm,
            memory_store=memory,
        )
        
        # Action logger
        self.action_logger = ActionLogger(name) if log_actions else None
        
        # Demo tools
        self._demo_tools: Dict[str, Tool] = {}
        
        # Conversation history
        self._conversation: List[Message] = []
    
    async def on_start(self) -> None:
        """Initialize demo tools on startup."""
        # Register demo tools
        demo_tools = [
            ListMarketsTool(agent=self),
            RunBacktestTool(agent=self),
            MonitorVolatilityTool(agent=self),
            GetMarketRegimeTool(agent=self),
            CalculateRiskMetricsTool(agent=self),
        ]
        
        for tool in demo_tools:
            self._tools.register(tool)
            self._demo_tools[tool.name] = tool
        
        if self.action_logger:
            self.action_logger.log_observation(f"Agent {self.name} started with tools: {list(self._tools.names())}")
        
        logger.info(f"Ollama Trading Agent '{self.name}' initialized with {len(self._tools.names())} tools")
    
    async def on_stop(self) -> None:
        """Cleanup on stop."""
        if self.action_logger:
            self.action_logger.print_summary()
    
    async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
        """Make a trading decision."""
        # This is called during autonomous operation
        # For demo, we use interactive mode via run_demo()
        return {"action": "observe", "reason": "Interactive mode"}
    
    async def think_and_act(
        self,
        user_prompt: str,
        max_iterations: int = 5,
    ) -> str:
        """
        Process a user prompt through thinking and tool use.
        
        This implements the agent loop:
        1. Think about the prompt
        2. Decide which tools to use
        3. Execute tools
        4. Synthesize results
        5. Provide response
        """
        # Build messages
        messages = [
            Message(
                role="system",
                content=self._build_system_prompt(),
            ),
        ]
        
        # Add conversation history (last few turns)
        messages.extend(self._conversation[-6:])
        
        # Add user prompt
        messages.append(Message(role="user", content=user_prompt))
        
        # Get available tools
        tools = list(self._tools.list())
        
        # Agent loop
        iteration = 0
        final_response = ""
        
        while iteration < max_iterations:
            iteration += 1
            
            try:
                # Call LLM
                start_time = time.time()
                response = await self._llm.complete(
                    messages=messages,
                    tools=tools if iteration < max_iterations else None,  # No tools on last iteration
                )
                llm_time = (time.time() - start_time) * 1000
                
                # Log thinking
                if response.content and self.action_logger:
                    self.action_logger.log_thinking(response.content[:500])
                
                # Check for tool calls
                if response.has_tool_calls:
                    # Execute tools
                    tool_results = []
                    
                    for tool_call in response.tool_calls:
                        tool = self._tools.get(tool_call.name)
                        if tool:
                            start_time = time.time()
                            result = await tool.execute(**tool_call.arguments)
                            exec_time = (time.time() - start_time) * 1000
                            
                            if self.action_logger:
                                self.action_logger.log_tool_call(
                                    tool_call.name,
                                    tool_call.arguments,
                                    result,
                                    exec_time,
                                )
                            
                            tool_results.append({
                                "tool": tool_call.name,
                                "result": result.to_dict(),
                            })
                            
                            # Store in memory
                            await self._memory.store(MemoryEntry(
                                memory_type=MemoryType.ACTION,
                                content=f"Used {tool_call.name}: {json.dumps(result.output)[:200]}",
                                metadata={"tool": tool_call.name},
                            ))
                    
                    # Add tool results to conversation
                    messages.append(Message(
                        role="assistant",
                        content=response.content or "",
                        tool_calls=[{
                            "id": tc.id,
                            "type": "function",
                            "function": {"name": tc.name, "arguments": json.dumps(tc.arguments)}
                        } for tc in response.tool_calls],
                    ))
                    
                    # Add tool results
                    for i, tr in enumerate(tool_results):
                        messages.append(Message(
                            role="tool",
                            content=json.dumps(tr["result"]),
                            tool_call_id=response.tool_calls[i].id,
                        ))
                    
                else:
                    # No tool calls, this is the final response
                    final_response = response.content
                    break
                    
            except Exception as e:
                logger.error(f"Error in agent loop: {e}")
                final_response = f"I encountered an error: {str(e)}"
                break
        
        # Store in conversation history
        self._conversation.append(Message(role="user", content=user_prompt))
        self._conversation.append(Message(role="assistant", content=final_response))
        
        # Log decision
        if self.action_logger and final_response:
            self.action_logger.log_decision(
                {"response": final_response[:300]},
                reasoning="Synthesized from tool results",
            )
        
        return final_response
    
    def _build_system_prompt(self) -> str:
        """Build the system prompt for the agent."""
        tools_desc = "\n".join([
            f"- {t.name}: {t.description}"
            for t in self._tools.list()
        ])
        
        return f"""You are {self.name}, an AI trading assistant powered by Neleus.

Your capabilities:
{tools_desc}

Your personality:
- Trading Style: {self.config.personality.trading_style}
- Risk Tolerance: {self.config.personality.risk_tolerance}
- Traits: {', '.join(self.config.personality.traits)}

Guidelines:
1. Always use tools to get real data before making recommendations
2. Consider risk metrics and volatility before suggesting trades
3. Explain your reasoning clearly
4. When running backtests, interpret the results meaningfully
5. Be cautious with position sizing - respect risk limits

When asked to analyze markets:
1. First check the current market regime
2. Monitor volatility levels
3. Run backtests if evaluating strategies
4. Calculate risk metrics for any positions

Respond in a helpful, analytical manner. Use the tools available to provide data-driven insights."""
    
    async def analyze_market(self, instrument: str) -> str:
        """Convenience method to analyze a market."""
        return await self.think_and_act(
            f"Analyze the current market conditions for {instrument}. "
            f"Check volatility, market regime, and provide trading recommendations."
        )
    
    async def backtest_strategy(
        self,
        instrument: str,
        strategy: str,
        start_date: str,
        end_date: str,
    ) -> str:
        """Convenience method to run and analyze a backtest."""
        return await self.think_and_act(
            f"Run a backtest for {instrument} using a {strategy} strategy "
            f"from {start_date} to {end_date}. Analyze the results and "
            f"tell me if this strategy is viable."
        )
    
    async def assess_risk(self, instrument: str, position_size: float) -> str:
        """Convenience method to assess risk for a position."""
        return await self.think_and_act(
            f"Assess the risk of taking a position of {position_size} in {instrument}. "
            f"Calculate VaR, check volatility, and provide position sizing recommendations."
        )


async def run_interactive_demo(
    model: str = "llama3.2",
    base_url: str = "http://localhost:11434",
) -> None:
    """
    Run an interactive demo session with the Ollama Trading Agent.
    
    This allows users to chat with the agent and see it use Neleus tools.
    """
    print("\n" + "=" * 60)
    print("  NELEUS AI AGENT DEMO - Powered by Ollama")
    print("=" * 60)
    print(f"\n  Model: {model}")
    print("  Commands: 'quit' to exit, 'summary' for session summary")
    print("=" * 60 + "\n")
    
    agent = OllamaTradingAgent(
        name="NeleusDemo",
        model=model,
        base_url=base_url,
        instruments=["BTC", "ETH", "SOL"],
        log_actions=True,
    )
    
    try:
        await agent.start()
        
        # Example prompts for demo
        example_prompts = [
            "Analyze the current volatility for BTC",
            "Run a momentum backtest on ETH from 2025-01-01 to 2025-12-31",
            "What's the current market regime?",
            "Assess the risk of a 1.0 BTC position",
        ]
        
        print("Example prompts you can try:")
        for i, prompt in enumerate(example_prompts, 1):
            print(f"  {i}. {prompt}")
        print()
        
        while True:
            try:
                user_input = input("\n🤖 You: ").strip()
                
                if not user_input:
                    continue
                
                if user_input.lower() == "quit":
                    break
                
                if user_input.lower() == "summary":
                    if agent.action_logger:
                        agent.action_logger.print_summary()
                    continue
                
                # Process the input
                response = await agent.think_and_act(user_input)
                
                print(f"\n💬 {agent.name}: {response}")
                
            except KeyboardInterrupt:
                print("\n\nInterrupted by user.")
                break
            except Exception as e:
                print(f"\n❌ Error: {e}")
                logger.exception("Error in demo loop")
        
    finally:
        await agent.stop()
        
        if agent.action_logger:
            print(f"\n📁 Log file saved to: {agent.action_logger.log_file}")


# Export
__all__ = [
    "OllamaTradingAgent",
    "ActionLogger",
    "AgentAction",
    "run_interactive_demo",
]
