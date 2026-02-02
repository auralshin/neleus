"""
Neleus - Agent Orchestrator Service
====================================

Make Your Agent Trade Smarter

Neleus is an AI agent orchestration platform that combines a high-performance
Rust core with an intelligent Python agent layer. Build, deploy, and manage
autonomous trading agents with memory, reasoning, and execution capabilities.

Quick Start - AI Agent:
    from neleus.ai import AIAgent, AgentConfig
    
    class MyTradingAgent(AIAgent):
        async def decide(self, context):
            # Think with LLM
            reasoning = await self.think("Analyze this market...")
            
            # Use tools
            data = await self.execute_tool("get_market_data", instrument="BTC-PERP")
            
            # Remember for future
            await self.remember("Important observation...", memory_type=MemoryType.OBSERVATION)
            
            return {"action": "hold", "reasoning": reasoning}
    
    # Run agent
    agent = MyTradingAgent(AgentConfig.from_project("./my_agent"))
    await agent.start()

Quick Start - Strategy:
    from neleus import Strategy, StrategyContext, Bar, OrderSide
    
    class MyStrategy(Strategy):
        def on_bar(self, ctx: StrategyContext, bar: Bar):
            if some_signal:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
    
    # Run backtest
    from neleus import backtest
    results = backtest(MyStrategy(), config)
    print(results.summary())

Architecture:
    - Rust core: Order management, position tracking, backtest engine
    - Python AI layer: Agent framework with memory, tools, and LLM integration
    - Python strategy layer: Strategy logic, configuration, analysis
    - PyO3 bridge: Zero-copy data passing between Rust and Python
"""

__version__ = "0.1.0"

# =============================================================================
# Core Types (from Rust or Python fallback)
# =============================================================================

from .types import (
    # Enums
    Venue,
    InstrumentType,
    OrderSide,
    OrderType,
    TimeInForce,
    OrderState,
    PositionSide,
    Network,
    FillModel,
    LatencyModel,
    SubscriptionType,
    Signal,
    # Market Data
    InstrumentId,
    TradeTick,
    QuoteTick,
    BookLevel,
    OrderBook,
    Bar,
    MarketData,
    # Trading
    Order,
    Fill,
    Position,
    OrderRequest,
    # Engine
    StrategyContext,
    BacktestResults,
    # Config
    HyperliquidConfig,
    LighterConfig,
    BacktestConfig,
    RiskConfig,
    # Algos
    TwapParams,
    VwapParams,
    IcebergParams,
    # Utils
    using_rust_types,
)

# =============================================================================
# Strategy Framework
# =============================================================================

from .strategy import (
    Strategy,
    Actor,
)

# =============================================================================
# Node & Execution (from node.py)
# =============================================================================

from .node import (
    Node,
    BacktestNode,
    PaperNode,
    LiveNode,
    backtest,
    # Hyperliquid-specific
    HyperliquidBacktestConfig,
    HyperliquidBacktestNode,
    CandleInterval,
    # Venue configs
    HyperliquidVenueConfig,
    LighterVenueConfig,
    SimulationConfig,
)

# =============================================================================
# Configuration
# =============================================================================

from .config import (
    load_project_config,
    load_strategy_config,
    save_config,
    validate_config,
    discover_strategies,
)

# =============================================================================
# Visualization
# =============================================================================

from .plots import (
    BacktestPlotter,
    LivePlotter,
    generate_html_report,
)

# =============================================================================
# Backtest Runner (for CLI)
# =============================================================================

from .backtest_runner import BacktestRunner

# =============================================================================
# UI Dashboard
# =============================================================================

from .ui import start_dashboard

# =============================================================================
# Managed Service - Signals (External AI/Quant Integration)
# =============================================================================

from .signals import (
    # Core types
    Signal as ExternalSignal,
    SignalType,
    SignalDirection,
    SignalPriority,
    SignalFeatures,
    SignalSubscription,
    # Client
    SignalClient,
    SignalListener,
    # Convenience functions
    entry_signal,
    exit_signal,
    risk_alert,
)

# =============================================================================
# Managed Service - Agents (CI/CD for Trading Bots)
# =============================================================================

from .agents import (
    # Specs
    AgentSpec,
    VenueSpec,
    RiskLimits,
    CapitalSpec,
    ScheduleSpec,
    # Stats
    AgentStats,
    # Management
    AgentManager,
    LocalAgentRunner,
)

# =============================================================================
# Managed Service - Deployment Configuration
# =============================================================================

from .config.deployment import (
    DeploymentEnvironment,
    DeploymentConfig,
    SecretRef,
    TEEConfig,
    local_dev_config,
    docker_config,
    kubernetes_config,
    tee_config,
)

# =============================================================================
# AI Agent Framework (New Agent Orchestrator)
# =============================================================================

from .ai import (
    # Core Agent
    AIAgent,
    AgentConfig,
    PersonalityConfig,
    InfoConfig,
    AgentState,
    # Memory System
    MemoryStore,
    LocalMemoryStore,
    CompositeMemoryStore,
    MemoryEntry,
    MemoryType,
    # Tool Framework
    Tool,
    ToolRegistry,
    ToolParameter,
    ToolResult,
    GetMarketDataTool,
    GetAnalysisTool,
    PlaceOrderTool,
    GetSignalsTool,
    QueryMemoryTool,
    GetPortfolioTool,
    SendMessageTool,
    # LLM Providers
    LLMProvider,
    OpenAIProvider,
    AnthropicProvider,
    OllamaProvider,
    Message,
    MessageRole,
    CompletionResult,
    ToolCall,
    create_provider,
    # Communication
    MessageBus,
    LocalMessageBus,
    AgentMessage,
    MessageType,
    MessagePriority,
    # Formatters
    MarketDataFormatter,
    SignalFormatter,
    PortfolioFormatter,
    AnalysisFormatter,
)


__all__ = [
    # Version
    "__version__",
    # Enums
    "Venue",
    "InstrumentType",
    "OrderSide",
    "OrderType",
    "TimeInForce",
    "OrderState",
    "PositionSide",
    "Network",
    "FillModel",
    "LatencyModel",
    "SubscriptionType",
    "Signal",
    # Market Data
    "InstrumentId",
    "TradeTick",
    "QuoteTick",
    "BookLevel",
    "OrderBook",
    "Bar",
    "MarketData",
    # Trading
    "Order",
    "Fill",
    "Position",
    "OrderRequest",
    # Engine
    "StrategyContext",
    "BacktestResults",
    # Config
    "HyperliquidConfig",
    "LighterConfig",
    "BacktestConfig",
    "RiskConfig",
    # Algos
    "TwapParams",
    "VwapParams",
    "IcebergParams",
    # Strategy
    "Strategy",
    "Actor",
    # Nodes
    "Node",
    "BacktestNode",
    "PaperNode",
    "LiveNode",
    "backtest",
    "HyperliquidBacktestConfig",
    "HyperliquidBacktestNode",
    "CandleInterval",
    "HyperliquidVenueConfig",
    "LighterVenueConfig",
    "SimulationConfig",
    # Config functions
    "load_project_config",
    "load_strategy_config",
    "save_config",
    "validate_config",
    "discover_strategies",
    # Visualization
    "BacktestPlotter",
    "LivePlotter",
    "generate_html_report",
    # Backtest Runner
    "BacktestRunner",
    # UI
    "start_dashboard",
    # Utils
    "using_rust_types",
    # ==========================================================================
    # Managed Service Components
    # ==========================================================================
    # External Signals (AI/Quant Integration)
    "ExternalSignal",
    "SignalType",
    "SignalDirection",
    "SignalPriority",
    "SignalFeatures",
    "SignalSubscription",
    "SignalClient",
    "SignalListener",
    "entry_signal",
    "exit_signal",
    "risk_alert",
    # Agent Management (CI/CD for Trading Bots)
    "AgentSpec",
    "VenueSpec",
    "RiskLimits",
    "CapitalSpec",
    "ScheduleSpec",
    "AgentStats",
    "AgentManager",
    "LocalAgentRunner",
    # Deployment Configuration
    "DeploymentEnvironment",
    "DeploymentConfig",
    "SecretRef",
    "TEEConfig",
    "local_dev_config",
    "docker_config",
    "kubernetes_config",
    "tee_config",
    # ==========================================================================
    # AI Agent Framework (Agent Orchestrator Service)
    # ==========================================================================
    # Core Agent
    "AIAgent",
    "AgentConfig",
    "PersonalityConfig",
    "InfoConfig",
    "AgentState",
    # Memory System
    "MemoryStore",
    "LocalMemoryStore",
    "CompositeMemoryStore",
    "MemoryEntry",
    "MemoryType",
    # Tool Framework
    "Tool",
    "ToolRegistry",
    "ToolParameter",
    "ToolResult",
    "GetMarketDataTool",
    "GetAnalysisTool",
    "PlaceOrderTool",
    "GetSignalsTool",
    "QueryMemoryTool",
    "GetPortfolioTool",
    "SendMessageTool",
    # LLM Providers
    "LLMProvider",
    "OpenAIProvider",
    "AnthropicProvider",
    "OllamaProvider",
    "Message",
    "MessageRole",
    "CompletionResult",
    "ToolCall",
    "create_provider",
    # Communication
    "MessageBus",
    "LocalMessageBus",
    "AgentMessage",
    "MessageType",
    "MessagePriority",
    # Formatters
    "MarketDataFormatter",
    "SignalFormatter",
    "PortfolioFormatter",
    "AnalysisFormatter",
]
