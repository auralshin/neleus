"""
Neleus AI Agent Framework

Infrastructure for AI trading agents with:
- Memory persistence (short-term, long-term, vector) - Rust core
- Data formatting for LLM consumption - Rust core
- Tool/Action framework - Rust core
- LLM provider abstraction - Python (HTTP client)
- Agent-to-agent communication - Rust core

Motto: "Make Your Agent Trade Smarter"

Architecture:
- Heavy lifting in Rust (via neleus_core)
- Python as thin wrapper for CLI and user-facing APIs
- LLM calls remain in Python (HTTP client simplicity)
"""

# Import from Rust core (via PyO3 bindings) when available
try:
    from neleus_core import (
        # Memory
        MemoryType as RustMemoryType,
        MemoryEntry as RustMemoryEntry,
        MemoryManager as RustMemoryManager,
        # Communication
        MessageType as RustMessageType,
        MessageBus as RustMessageBus,
        # Formatters
        MarketDataFormatter as RustMarketDataFormatter,
        SignalFormatter as RustSignalFormatter,
        PortfolioFormatter as RustPortfolioFormatter,
        AnalysisFormatter as RustAnalysisFormatter,
        # Tools
        ToolRegistry as RustToolRegistry,
    )
    _RUST_AVAILABLE = True
except ImportError:
    _RUST_AVAILABLE = False

# Python implementations (used as fallback or for features not in Rust)
from .agent import AIAgent, AgentConfig, PersonalityConfig, InfoConfig, AgentState
from .memory import MemoryStore, LocalMemoryStore, CompositeMemoryStore, MemoryEntry, MemoryType
from .tools import (
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
)
from .llm import (
    LLMProvider,
    OpenAIProvider,
    AnthropicProvider,
    OllamaProvider,
    Message,
    Role as MessageRole,
    CompletionResult,
    ToolCall,
    create_provider,
)
from .communication import MessageBus, LocalMessageBus, AgentMessage, MessageType, MessagePriority
from .formatters import (
    MarketDataFormatter,
    SignalFormatter,
    PortfolioFormatter,
    AnalysisFormatter,
)

__all__ = [
    # Core Agent
    "AIAgent",
    "AgentConfig",
    "PersonalityConfig",
    "InfoConfig",
    "AgentState",
    # Memory
    "MemoryStore",
    "LocalMemoryStore",
    "CompositeMemoryStore",
    "MemoryEntry",
    "MemoryType",
    # Tools
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
    # LLM
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
