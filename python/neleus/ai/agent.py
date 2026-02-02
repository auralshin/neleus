"""
AI Agent Base Class

The core AIAgent class that provides:
- Lifecycle management (start, stop, pause, resume)
- Memory integration (Rust core)
- Tool execution (Rust core)
- LLM provider abstraction (Python HTTP client)
- Configuration from personality.yaml and info.yaml

Heavy lifting is done in Rust, Python provides the orchestration layer.
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Callable, Union
import yaml

# Try to use Rust core, fallback to Python implementations
try:
    from neleus_core import (
        MemoryType as RustMemoryType,
        MemoryManager as RustMemoryManager,
        MessageBus as RustMessageBus,
        MessageType as RustMessageType,
        ToolRegistry as RustToolRegistry,
    )
    _RUST_AVAILABLE = True
except ImportError:
    _RUST_AVAILABLE = False

from .memory import MemoryStore, MemoryEntry, MemoryType
from .tools import Tool, ToolRegistry, ToolResult
from .llm import LLMProvider, Message
from .communication import MessageBus, AgentMessage, MessageType


logger = logging.getLogger(__name__)


class AgentState(str, Enum):
    """Agent lifecycle states."""
    CREATED = "created"
    INITIALIZING = "initializing"
    READY = "ready"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPING = "stopping"
    STOPPED = "stopped"
    ERROR = "error"


@dataclass
class PersonalityConfig:
    """Agent personality configuration from personality.yaml"""
    name: str
    description: str
    trading_style: str  # e.g., "aggressive", "conservative", "balanced"
    risk_tolerance: str  # e.g., "low", "medium", "high"
    decision_speed: str  # e.g., "fast", "deliberate", "adaptive"
    
    # Behavioral traits
    traits: List[str] = field(default_factory=list)
    
    # System prompt for LLM
    system_prompt: Optional[str] = None
    
    # Decision-making preferences
    prefer_momentum: bool = False
    prefer_mean_reversion: bool = False
    use_fundamental_analysis: bool = True
    use_technical_analysis: bool = True
    use_sentiment_analysis: bool = False
    
    # Communication style
    verbose_reasoning: bool = True
    explain_decisions: bool = True
    
    @classmethod
    def from_yaml(cls, path: Union[str, Path]) -> "PersonalityConfig":
        """Load personality from YAML file."""
        with open(path, "r") as f:
            data = yaml.safe_load(f)
        return cls(**data)
    
    def to_system_prompt(self) -> str:
        """Generate system prompt from personality."""
        if self.system_prompt:
            return self.system_prompt
        
        traits_str = ", ".join(self.traits) if self.traits else "analytical, data-driven"
        
        prompt = f"""You are {self.name}, an AI trading agent.

Description: {self.description}

Trading Style: {self.trading_style}
Risk Tolerance: {self.risk_tolerance}
Decision Speed: {self.decision_speed}
Key Traits: {traits_str}

Analysis Capabilities:
- Technical Analysis: {"Enabled" if self.use_technical_analysis else "Disabled"}
- Fundamental Analysis: {"Enabled" if self.use_fundamental_analysis else "Disabled"}
- Sentiment Analysis: {"Enabled" if self.use_sentiment_analysis else "Disabled"}

Trading Preferences:
- Momentum Strategies: {"Preferred" if self.prefer_momentum else "Available"}
- Mean Reversion: {"Preferred" if self.prefer_mean_reversion else "Available"}

Communication:
- {"Provide detailed reasoning for all decisions" if self.verbose_reasoning else "Be concise"}
- {"Explain trade rationale" if self.explain_decisions else "Focus on actions"}

Always prioritize risk management and capital preservation. Never exceed position limits.
When uncertain, gather more data before acting."""
        
        return prompt


@dataclass
class InfoConfig:
    """Agent capabilities configuration from info.yaml"""
    version: str
    
    # Enabled tools
    tools: List[str] = field(default_factory=list)
    
    # Knowledge sources
    knowledge_sources: List[str] = field(default_factory=list)
    
    # Supported instruments
    instruments: List[str] = field(default_factory=list)
    
    # Supported venues
    venues: List[str] = field(default_factory=list)
    
    # Data access
    data_feeds: List[str] = field(default_factory=list)
    
    # Memory configuration
    memory_backend: str = "local"  # local, redis, postgres
    vector_store: Optional[str] = None  # None, chromadb, pinecone
    
    # LLM configuration
    llm_provider: str = "openai"
    llm_model: str = "gpt-4o"
    temperature: float = 0.7
    max_tokens: int = 4096
    
    # Risk limits
    max_position_size: float = 0.1
    max_daily_loss: float = 0.05
    max_leverage: float = 5.0
    
    # Timing
    decision_interval_seconds: int = 60
    max_decisions_per_hour: int = 60
    
    @classmethod
    def from_yaml(cls, path: Union[str, Path]) -> "InfoConfig":
        """Load info from YAML file."""
        with open(path, "r") as f:
            data = yaml.safe_load(f)
        return cls(**data)


@dataclass
class AgentConfig:
    """Complete agent configuration."""
    personality: PersonalityConfig
    info: InfoConfig
    project_path: Path
    
    @classmethod
    def from_project(cls, project_path: Union[str, Path]) -> "AgentConfig":
        """Load configuration from a project directory."""
        path = Path(project_path)
        
        personality_file = path / "personality.yaml"
        info_file = path / "info.yaml"
        
        if not personality_file.exists():
            raise FileNotFoundError(f"personality.yaml not found in {path}")
        if not info_file.exists():
            raise FileNotFoundError(f"info.yaml not found in {path}")
        
        return cls(
            personality=PersonalityConfig.from_yaml(personality_file),
            info=InfoConfig.from_yaml(info_file),
            project_path=path,
        )


class AIAgent(ABC):
    """
    Base class for AI trading agents.
    
    Provides:
    - Configuration loading from personality.yaml and info.yaml
    - Memory persistence
    - Tool execution framework
    - LLM integration
    - Lifecycle management
    - Agent-to-agent communication
    
    Example:
        class MyTradingAgent(AIAgent):
            async def on_market_data(self, data: Dict[str, Any]) -> None:
                analysis = await self.analyze(data)
                if analysis.should_trade:
                    await self.execute_tool("place_order", analysis.order_params)
            
            async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
                # Use LLM to make trading decision
                response = await self.think(
                    f"Given this market data: {context}, should I trade?"
                )
                return self.parse_decision(response)
    """
    
    def __init__(
        self,
        config: AgentConfig,
        llm_provider: Optional[LLMProvider] = None,
        memory_store: Optional[MemoryStore] = None,
        message_bus: Optional[MessageBus] = None,
    ):
        self.config = config
        self.id = f"{config.personality.name.lower().replace(' ', '-')}-{id(self)}"
        self.state = AgentState.CREATED
        
        # Core components (initialized in start())
        self._llm = llm_provider
        self._memory = memory_store
        self._message_bus = message_bus
        self._tools = ToolRegistry()
        
        # Runtime state
        self._started_at: Optional[datetime] = None
        self._last_decision: Optional[datetime] = None
        self._decision_count: int = 0
        self._error_count: int = 0
        
        # Event loop
        self._loop: Optional[asyncio.AbstractEventLoop] = None
        self._running = False
        
        logger.info(f"Agent {self.id} created with personality: {config.personality.name}")
    
    @property
    def name(self) -> str:
        """Agent name from personality."""
        return self.config.personality.name
    
    @property
    def is_running(self) -> bool:
        """Check if agent is currently running."""
        return self.state == AgentState.RUNNING
    
    @property
    def uptime(self) -> Optional[float]:
        """Get agent uptime in seconds."""
        if self._started_at:
            return (datetime.now() - self._started_at).total_seconds()
        return None
    
    # =========================================================================
    # Lifecycle Methods
    # =========================================================================
    
    async def start(self) -> None:
        """Start the agent."""
        if self.state not in (AgentState.CREATED, AgentState.STOPPED):
            raise RuntimeError(f"Cannot start agent in state {self.state}")
        
        self.state = AgentState.INITIALIZING
        logger.info(f"Starting agent {self.id}")
        
        try:
            # Initialize components
            await self._initialize_components()
            
            # Register default tools
            self._register_default_tools()
            
            # Call user initialization
            await self.on_start()
            
            self.state = AgentState.READY
            self._started_at = datetime.now()
            
            # Start main loop
            self._running = True
            self.state = AgentState.RUNNING
            
            logger.info(f"Agent {self.id} started successfully")
            
        except Exception as e:
            self.state = AgentState.ERROR
            logger.error(f"Failed to start agent {self.id}: {e}")
            raise
    
    async def stop(self) -> None:
        """Stop the agent gracefully."""
        if self.state not in (AgentState.RUNNING, AgentState.PAUSED, AgentState.ERROR):
            return
        
        self.state = AgentState.STOPPING
        logger.info(f"Stopping agent {self.id}")
        
        self._running = False
        
        try:
            await self.on_stop()
        except Exception as e:
            logger.error(f"Error in on_stop: {e}")
        
        self.state = AgentState.STOPPED
        logger.info(f"Agent {self.id} stopped")
    
    async def pause(self) -> None:
        """Pause the agent."""
        if self.state != AgentState.RUNNING:
            return
        
        self.state = AgentState.PAUSED
        await self.on_pause()
        logger.info(f"Agent {self.id} paused")
    
    async def resume(self) -> None:
        """Resume a paused agent."""
        if self.state != AgentState.PAUSED:
            return
        
        self.state = AgentState.RUNNING
        await self.on_resume()
        logger.info(f"Agent {self.id} resumed")
    
    async def _initialize_components(self) -> None:
        """Initialize agent components."""
        info = self.config.info
        
        # Initialize LLM provider if not provided
        if self._llm is None:
            from .llm import create_provider
            self._llm = create_provider(
                provider=info.llm_provider,
                model=info.llm_model,
                temperature=info.temperature,
                max_tokens=info.max_tokens,
            )
        
        # Initialize memory store if not provided
        if self._memory is None:
            from .memory import create_memory_store
            self._memory = create_memory_store(
                backend=info.memory_backend,
                vector_store=info.vector_store,
            )
        
        # Initialize message bus if not provided
        if self._message_bus is None:
            from .communication import create_message_bus
            self._message_bus = create_message_bus()
        
        # Subscribe to messages for this agent
        await self._message_bus.subscribe(
            agent_id=self.id,
            callback=self._handle_message,
        )
    
    def _register_default_tools(self) -> None:
        """Register default trading tools."""
        from .tools import (
            GetMarketDataTool,
            GetAnalysisTool,
            PlaceOrderTool,
            GetSignalsTool,
            QueryMemoryTool,
            GetPortfolioTool,
            SendMessageTool,
        )
        
        enabled_tools = self.config.info.tools
        
        tool_classes = {
            "get_market_data": GetMarketDataTool,
            "get_analysis": GetAnalysisTool,
            "place_order": PlaceOrderTool,
            "get_signals": GetSignalsTool,
            "query_memory": QueryMemoryTool,
            "get_portfolio": GetPortfolioTool,
            "send_message": SendMessageTool,
        }
        
        for tool_name in enabled_tools:
            if tool_name in tool_classes:
                self._tools.register(tool_classes[tool_name](agent=self))
                logger.debug(f"Registered tool: {tool_name}")
    
    async def _handle_message(self, message: AgentMessage) -> None:
        """Handle incoming agent messages."""
        try:
            await self.on_message(message)
        except Exception as e:
            logger.error(f"Error handling message: {e}")
    
    # =========================================================================
    # Core Agent Methods
    # =========================================================================
    
    async def think(
        self,
        prompt: str,
        context: Optional[Dict[str, Any]] = None,
        tools: Optional[List[str]] = None,
    ) -> str:
        """
        Use LLM to reason about a situation.
        
        Args:
            prompt: The question or situation to reason about
            context: Additional context to include
            tools: List of tool names available for this reasoning step
        
        Returns:
            LLM response text
        """
        # Build messages
        messages = [
            Message(role="system", content=self.config.personality.to_system_prompt()),
        ]
        
        # Add relevant memories
        memories = await self._memory.recall(
            query=prompt,
            limit=5,
            memory_type=MemoryType.DECISION,
        )
        if memories:
            memory_context = "\n".join([f"- {m.content}" for m in memories])
            messages.append(Message(
                role="system",
                content=f"Relevant past decisions:\n{memory_context}"
            ))
        
        # Add context if provided
        if context:
            import json
            messages.append(Message(
                role="system",
                content=f"Current context:\n{json.dumps(context, indent=2)}"
            ))
        
        # Add user prompt
        messages.append(Message(role="user", content=prompt))
        
        # Get available tools
        available_tools = None
        if tools:
            available_tools = [self._tools.get(t) for t in tools if self._tools.get(t)]
        
        # Call LLM
        response = await self._llm.complete(
            messages=messages,
            tools=available_tools,
        )
        
        return response.content
    
    async def execute_tool(self, tool_name: str, **kwargs) -> ToolResult:
        """
        Execute a registered tool.
        
        Args:
            tool_name: Name of the tool to execute
            **kwargs: Tool parameters
        
        Returns:
            ToolResult with output or error
        """
        tool = self._tools.get(tool_name)
        if not tool:
            return ToolResult(
                success=False,
                error=f"Tool '{tool_name}' not found",
            )
        
        try:
            result = await tool.execute(**kwargs)
            
            # Store tool usage in memory
            await self._memory.store(MemoryEntry(
                memory_type=MemoryType.ACTION,
                content=f"Executed {tool_name} with {kwargs}: {result.output}",
                metadata={"tool": tool_name, "params": kwargs},
            ))
            
            return result
        except Exception as e:
            logger.error(f"Tool execution error: {e}")
            return ToolResult(success=False, error=str(e))
    
    async def remember(
        self,
        content: str,
        memory_type: MemoryType = MemoryType.OBSERVATION,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Store something in memory."""
        await self._memory.store(MemoryEntry(
            memory_type=memory_type,
            content=content,
            metadata=metadata or {},
        ))
    
    async def recall(
        self,
        query: str,
        limit: int = 10,
        memory_type: Optional[MemoryType] = None,
    ) -> List[MemoryEntry]:
        """Recall memories matching a query."""
        return await self._memory.recall(
            query=query,
            limit=limit,
            memory_type=memory_type,
        )
    
    async def send_message(
        self,
        to_agent: str,
        message_type: MessageType,
        payload: Dict[str, Any],
    ) -> None:
        """Send a message to another agent."""
        await self._message_bus.publish(AgentMessage(
            from_agent=self.id,
            to_agent=to_agent,
            message_type=message_type,
            payload=payload,
        ))
    
    async def broadcast(
        self,
        message_type: MessageType,
        payload: Dict[str, Any],
    ) -> None:
        """Broadcast a message to all agents."""
        await self._message_bus.broadcast(AgentMessage(
            from_agent=self.id,
            to_agent="*",
            message_type=message_type,
            payload=payload,
        ))
    
    # =========================================================================
    # Abstract Methods - Must be implemented by subclasses
    # =========================================================================
    
    @abstractmethod
    async def on_start(self) -> None:
        """Called when agent starts. Override to initialize custom state."""
        pass
    
    @abstractmethod
    async def on_stop(self) -> None:
        """Called when agent stops. Override to cleanup."""
        pass
    
    @abstractmethod
    async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Make a trading decision given the current context.
        
        This is the main decision-making method that should use
        self.think() for LLM reasoning and self.execute_tool() for actions.
        
        Args:
            context: Current market context
        
        Returns:
            Decision dictionary with action and parameters
        """
        pass
    
    # =========================================================================
    # Optional Override Methods
    # =========================================================================
    
    async def on_pause(self) -> None:
        """Called when agent is paused."""
        pass
    
    async def on_resume(self) -> None:
        """Called when agent resumes."""
        pass
    
    async def on_market_data(self, data: Dict[str, Any]) -> None:
        """Called when new market data arrives."""
        pass
    
    async def on_signal(self, signal: Dict[str, Any]) -> None:
        """Called when a trading signal is received."""
        pass
    
    async def on_message(self, message: AgentMessage) -> None:
        """Called when a message from another agent arrives."""
        pass
    
    async def on_error(self, error: Exception) -> None:
        """Called when an error occurs."""
        self._error_count += 1
        logger.error(f"Agent {self.id} error: {error}")
    
    # =========================================================================
    # Utility Methods
    # =========================================================================
    
    def get_status(self) -> Dict[str, Any]:
        """Get agent status summary."""
        return {
            "id": self.id,
            "name": self.name,
            "state": self.state.value,
            "uptime": self.uptime,
            "decision_count": self._decision_count,
            "error_count": self._error_count,
            "last_decision": self._last_decision.isoformat() if self._last_decision else None,
            "config": {
                "personality": self.config.personality.name,
                "trading_style": self.config.personality.trading_style,
                "risk_tolerance": self.config.personality.risk_tolerance,
                "llm_model": self.config.info.llm_model,
                "tools": self.config.info.tools,
            }
        }
