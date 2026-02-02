# Neleus Usage Guide

> **Agent Orchestrator Service · Make Your Agent Trade Smarter**

Comprehensive guide to using Neleus for building AI-powered trading systems.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Creating AI Trading Agents](#creating-ai-trading-agents)
- [Creating Trading Projects](#creating-trading-projects)
- [Using the Rust Core](#using-the-rust-core)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Examples](#examples)
- [Best Practices](#best-practices)

---

## Installation

### Prerequisites

- Python 3.9+ (Python 3.13 recommended)
- Rust 1.70+ (for building from source)
- Git

### Install from Source

```bash
# Clone repository
git clone https://github.com/auralshin/neleus.git
cd neleus

# Create virtual environment
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install Python dependencies
pip install -r python/requirements.txt

# Build Rust core
cd crates/pybridge
maturin develop --release
cd ../..

# Install CLI
pip install -e python/
```

### Verify Installation

```bash
neleus --version
# Output: Neleus v0.1.0

neleus info
# Shows ASCII art and project info
```

---

## Quick Start

### 1. Create Your First AI Agent

```bash
# Create an AI trading agent project
neleus new-agent my_first_agent

# Navigate to the project
cd my_first_agent

# Set your OpenAI API key
export OPENAI_API_KEY="sk-..."

# Validate the agent
neleus agent validate .
```

Your agent project structure:
```
my_first_agent/
├── personality.yaml    # Agent behavior & trading style
├── info.yaml          # Capabilities & risk limits
├── main.py            # Agent implementation
├── README.md          # Project documentation
├── requirements.txt   # Python dependencies
├── data/             # Market data cache
├── logs/             # Agent logs
└── memory/           # Persistent memory storage
```

### 2. Customize Your Agent

**Edit `personality.yaml`:**
```yaml
name: "Momentum Trader"
description: "Aggressive momentum trading agent"
trading_style: "aggressive"
risk_tolerance: "high"
decision_speed: "fast"

traits:
  - "data-driven"
  - "quick to act"
  - "trend-following"

prefer_momentum: true
use_technical_analysis: true
verbose_reasoning: true
```

**Edit `info.yaml`:**
```yaml
version: "1.0.0"
llm_provider: "openai"
llm_model: "gpt-4o"
temperature: 0.7

instruments: ["BTC-PERP", "ETH-PERP"]
venues: ["hyperliquid"]

max_position_size: 0.1
max_daily_loss: 0.05
max_leverage: 5.0
```

### 3. Run Your Agent

```bash
# Run the agent
python main.py

# Or use the CLI
neleus agent run .
```

---

## Creating AI Trading Agents

### Agent Architecture

Neleus agents use a Rust-first architecture with Python wrappers:

- **Rust Core**: High-performance memory, communication, and tools
- **Python Layer**: LLM integration, CLI, and orchestration

### Agent Components

#### 1. Memory System (Rust Core)

The agent maintains persistent memory across sessions:

```python
from neleus_core import MemoryManager, MemoryType

# Initialize memory manager
memory = MemoryManager()  # Uses in-memory storage
# Or with persistence:
memory = MemoryManager(db_path="./memory/agent.db")

# Store memories
memory.remember(
    agent_id="my-agent",
    content="BTC showing bullish momentum above 95k",
    memory_type=MemoryType.observation(),
    importance=0.8
)

# Recall memories
memories = memory.recall("my-agent", limit=10)
for mem in memories:
    print(f"{mem.content} (importance: {mem.importance})")

# Count memories
count = memory.count("my-agent")
print(f"Agent has {count} memories")
```

**Memory Types:**
- `observation()` - Market observations
- `decision()` - Trading decisions made
- `action()` - Actions executed
- `outcome()` - Results of actions
- `learning()` - Lessons learned
- `context()` - Environmental context
- `conversation()` - Conversations with users/agents

#### 2. Communication System (Rust Core)

Agents can communicate via a message bus:

```python
from neleus_core import MessageBus, MessageType

# Initialize message bus
bus = MessageBus()

# Register agents
bus.register("agent-alpha")
bus.register("agent-beta")

# Subscribe to topics
bus.subscribe("agent-beta", "market_signals")

# Broadcast messages
bus.broadcast(
    from_agent="agent-alpha",
    topic="market_signals",
    message_type=MessageType.signal_share(),
    payload={"symbol": "BTC", "signal": "buy", "confidence": 0.85}
)

# Check pending messages
pending = bus.pending_count("agent-beta")
print(f"Agent has {pending} pending messages")
```

**Message Types:**
- `data_request()` - Request data
- `data_response()` - Data response
- `signal_share()` - Share trading signals
- `alert()` - Urgent notifications
- `status()` - Status updates

#### 3. Tool System (Rust Core)

Agents have built-in tools for trading operations:

```python
from neleus_core import ToolRegistry

# Initialize tool registry
tools = ToolRegistry()

# List available tools
print(tools.list_tools())
# Output: ['place_order', 'get_market_data', 'get_portfolio', 'get_signals', 'get_analysis']

# Execute a tool
result = tools.execute("get_market_data", {
    "symbol": "BTC-PERP",
    "timeframe": "1h"
})

if result["success"]:
    print(f"Market data: {result['output']}")
    print(f"Executed in {result['execution_time_ms']}ms")
```

**Built-in Tools:**
- `get_market_data` - Fetch market data
- `place_order` - Place trading orders
- `get_portfolio` - Get portfolio status
- `get_signals` - Retrieve signals
- `get_analysis` - Run technical analysis

#### 4. Data Formatters (Rust Core)

Format market data for LLM consumption:

```python
from neleus_core import (
    MarketDataFormatter,
    SignalFormatter,
    PortfolioFormatter,
    AnalysisFormatter
)

# Format ticker data
ticker = {
    "symbol": "BTC-PERP",
    "last_price": 95000.0,
    "bid": 94995.0,
    "ask": 95005.0,
    "volume": 12500.0
}
formatted = MarketDataFormatter.format_ticker(ticker)
print(formatted)

# Format signals
signal = {
    "symbol": "BTC-PERP",
    "type": "buy",
    "confidence": 0.85,
    "reasoning": "Strong momentum breakout"
}
formatted = SignalFormatter.format_signal(signal)
print(formatted)
```

### Advanced Agent Example

```python
from neleus.ai import AIAgent, AgentConfig
from neleus_core import MemoryManager, MessageBus, ToolRegistry
import asyncio

class MyTradingAgent(AIAgent):
    """Custom trading agent with memory and communication."""
    
    def __init__(self, config: AgentConfig):
        super().__init__(config)
        
        # Initialize Rust core components
        self.memory = MemoryManager(db_path=f"./memory/{self.agent_id}.db")
        self.message_bus = MessageBus()
        self.tools = ToolRegistry()
        
        # Register with message bus
        self.message_bus.register(self.agent_id)
        self.message_bus.subscribe(self.agent_id, "market_updates")
    
    async def on_start(self):
        """Called when agent starts."""
        self.logger.info("Agent starting...")
        
        # Recall past decisions
        memories = self.memory.recall(
            self.agent_id,
            memory_type=MemoryType.decision(),
            limit=5
        )
        
        self.logger.info(f"Recalled {len(memories)} past decisions")
    
    async def on_market_data(self, data: dict):
        """Process incoming market data."""
        # Store observation
        self.memory.remember(
            self.agent_id,
            content=f"Price: {data['price']}, Volume: {data['volume']}",
            memory_type=MemoryType.observation(),
            importance=0.7
        )
        
        # Analyze with tools
        analysis = self.tools.execute("get_analysis", {
            "symbol": data["symbol"],
            "timeframe": "1h"
        })
        
        # Make decision
        if analysis["success"]:
            decision = await self.make_decision(data, analysis["output"])
            
            # Store decision
            self.memory.remember(
                self.agent_id,
                content=f"Decision: {decision['action']} - {decision['reason']}",
                memory_type=MemoryType.decision(),
                importance=0.9
            )
            
            # Share signal
            if decision["action"] != "hold":
                self.message_bus.broadcast(
                    self.agent_id,
                    "trade_signals",
                    MessageType.signal_share(),
                    decision
                )

# Run the agent
if __name__ == "__main__":
    config = AgentConfig.from_project(".")
    agent = MyTradingAgent(config)
    asyncio.run(agent.run())
```

---

## Creating Trading Projects

### Traditional Trading Project

For strategy-based trading (non-AI):

```bash
# Create project
neleus new my_trading_bot
cd my_trading_bot

# Project structure
# my_trading_bot/
# ├── .env.example
# ├── README.md
# ├── neleus.toml
# ├── strategies/
# │   ├── momentum_strategy.py
# │   └── mean_reversion_strategy.py
# ├── config/
# │   └── venues.py
# ├── backtests/
# ├── data/
# └── logs/
```

### Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Edit .env
HYPERLIQUID_WALLET=0x...
HYPERLIQUID_PRIVATE_KEY=...
```

### List Strategies

```bash
neleus strategy list

# Output:
# ┏━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
# ┃ Name                    ┃ File                              ┃
# ┡━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
# │ momentum_strategy       │ strategies/momentum_strategy.py   │
# │ mean_reversion_strategy │ strategies/mean_reversion_strategy│
# └─────────────────────────┴───────────────────────────────────┘
```

### Run Backtest

```bash
# Backtest a strategy
neleus backtest -s momentum_strategy --symbol BTC-PERP --timeframe 1h --capital 100000

# With date range
neleus backtest -s momentum_strategy \
  --start 2024-01-01 \
  --end 2024-12-31 \
  --symbol ETH-PERP
```

### Build & Validate

```bash
# Validate project
neleus build

# Output:
# 🔨 Building project...
#   ✓ Validating neleus.toml
#   ✓ Checking strategies
#   ✓ Validating configuration
# Build successful!
```

### Live Trading

```bash
# Paper trading (simulated)
neleus live --strategy momentum_strategy --paper

# Live trading (real money)
neleus live --strategy momentum_strategy --venue hyperliquid
```

---

## Using the Rust Core

### Python API

All Rust core functionality is exposed via PyO3 bindings:

```python
# Import Rust core
from neleus_core import (
    # Memory
    MemoryType,
    MemoryEntry,
    MemoryManager,
    # Communication
    MessageType,
    MessageBus,
    # Formatters
    MarketDataFormatter,
    SignalFormatter,
    PortfolioFormatter,
    AnalysisFormatter,
    # Tools
    ToolRegistry,
)

# Check if Rust core is available
from neleus.ai import _RUST_AVAILABLE
if _RUST_AVAILABLE:
    print("Using high-performance Rust core")
else:
    print("Falling back to Python implementations")
```

### Performance Benefits

The Rust core provides significant performance improvements:

- **Memory Operations**: 10-100x faster than pure Python
- **Message Passing**: Zero-copy message bus
- **Concurrent Access**: Lock-free data structures
- **Low Latency**: Sub-millisecond tool execution

### Direct Rust Usage

For advanced users, you can use the Rust crates directly:

```rust
// Cargo.toml
[dependencies]
agent-memory = { path = "crates/agent-memory" }
agent-comm = { path = "crates/agent-comm" }
agent-core = { path = "crates/agent-core" }

// main.rs
use agent_memory::{MemoryManager, MemoryType, MemoryConfig};
use agent_comm::{LocalMessageBus, MessageBus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize memory
    let config = MemoryConfig {
        db_path: Some("./memory.db".to_string()),
        ..Default::default()
    };
    let memory = MemoryManager::new(config)?;
    
    // Store memory
    memory.remember(
        "agent-1",
        "BTC trending up".to_string(),
        MemoryType::Observation,
        Some(0.8),
        None,
        None
    ).await?;
    
    Ok(())
}
```

---

## CLI Reference

### Global Options

```bash
neleus --version              # Show version
neleus --help                 # Show help
```

### Project Commands

```bash
# Create new trading project
neleus new <name>

# Initialize in current directory
neleus init

# Show project info
neleus info

# Build/validate project
neleus build
```

### Agent Commands

```bash
# Create AI agent project
neleus new-agent <name>

# Run agent
neleus agent run <path>

# List agents
neleus agent list <directory>

# Validate agent config
neleus agent validate <path>
```

### Trading Commands

```bash
# Backtest
neleus backtest \
  --strategy <name> \
  --symbol <symbol> \
  --timeframe <tf> \
  --start <date> \
  --end <date> \
  --capital <amount>

# Live trading
neleus live \
  --strategy <name> \
  --paper              # Paper trading mode
  --venue <venue>      # hyperliquid, lighter, etc.

# Strategy management
neleus strategy list   # List strategies
```

### Agent Orchestration

```bash
# Deploy agent
neleus deploy <name> \
  --config <config.yaml> \
  --strategy <strategy> \
  --venue <venue> \
  --instruments <list> \
  --capital <amount> \
  --mode paper|live

# Deploy batch
neleus deploy batch <batch_config.yaml>

# Generate template
neleus deploy template > agent_config.yaml

# Manage agents
neleus agents list                    # List deployed agents
neleus agents status <agent_id>       # Agent status
neleus agents start <agent_id>        # Start agent
neleus agents stop <agent_id>         # Stop agent
neleus agents pause <agent_id>        # Pause agent
neleus agents resume <agent_id>       # Resume agent
neleus agents restart <agent_id>      # Restart agent
neleus agents delete <agent_id>       # Delete agent
neleus agents logs <agent_id>         # View logs
```

### Signal Management

```bash
# Send signal
neleus signals send \
  --source <source> \
  --symbol <symbol> \
  --type <buy|sell> \
  --confidence <0-1> \
  --metadata <json>

# List signals
neleus signals list --limit 10

# Test connectivity
neleus signals test

# Subscribe agent to signals
neleus signals subscribe <agent_id> <source>

# List signal sources
neleus signals sources
```

### Metrics & Monitoring

```bash
# Get agent metrics
neleus metrics get <agent_id>

# Summary for all agents
neleus metrics summary

# Historical metrics
neleus metrics history <agent_id> --days 7

# Export metrics
neleus metrics export <agent_id> --format csv|json

# View alerts
neleus metrics alerts

# Terminal dashboard
neleus metrics dashboard
```

### UI Dashboard

```bash
# Start web dashboard
neleus ui

# Custom port
neleus ui --port 8080

# Open in browser
neleus ui --open
```

---

## Configuration

### Agent Configuration Files

#### personality.yaml

Defines agent behavior and trading style:

```yaml
name: "Momentum Trader"
description: "Aggressive momentum-based trading agent"

# Trading characteristics
trading_style: "aggressive"     # aggressive, conservative, balanced
risk_tolerance: "high"          # low, medium, high
decision_speed: "fast"          # fast, deliberate, adaptive

# Behavioral traits
traits:
  - "data-driven"
  - "trend-following"
  - "quick to act"

# Strategy preferences
prefer_momentum: true
prefer_mean_reversion: false
use_fundamental_analysis: false
use_technical_analysis: true
use_sentiment_analysis: false

# Communication style
verbose_reasoning: true
explain_decisions: true

# Custom system prompt (optional)
system_prompt: |
  You are an aggressive momentum trader focused on capturing
  strong trends in cryptocurrency markets.
```

#### info.yaml

Defines capabilities and limits:

```yaml
version: "1.0.0"

# LLM configuration
llm_provider: "openai"          # openai, anthropic, ollama
llm_model: "gpt-4o"
temperature: 0.7
max_tokens: 4096

# Enabled tools
tools:
  - "get_market_data"
  - "place_order"
  - "get_portfolio"
  - "get_analysis"

# Trading instruments
instruments:
  - "BTC-PERP"
  - "ETH-PERP"
  - "SOL-PERP"

# Supported venues
venues:
  - "hyperliquid"

# Data feeds
data_feeds:
  - "hyperliquid"
  - "coingecko"

# Memory configuration
memory_backend: "sqlite"        # local, sqlite, postgres
vector_store: null              # null, chromadb, pinecone

# Risk limits
max_position_size: 0.1          # 10% of capital per position
max_daily_loss: 0.05            # 5% daily loss limit
max_leverage: 5.0               # 5x max leverage

# Timing constraints
decision_interval_seconds: 60   # Minimum time between decisions
max_decisions_per_hour: 60      # Rate limit
```

### Project Configuration

#### neleus.toml

Main project configuration:

```toml
[project]
name = "my_trading_bot"
version = "0.1.0"
description = "My trading bot"

[engine]
initial_balance = 100000.0
commission_rate = 0.001
slippage_model = "realistic"

[venues]
default = "hyperliquid"

[venues.hyperliquid]
testnet = true
wallet_address = "${HYPERLIQUID_WALLET}"

[strategies]
default = "momentum_strategy"

[logging]
level = "INFO"
file = "logs/neleus.log"
```

---

## Examples

### Example 1: Simple Memory Agent

```python
from neleus_core import MemoryManager, MemoryType
import time

# Create memory manager
memory = MemoryManager()

# Store observations
for i in range(5):
    memory.remember(
        "price-watcher",
        f"BTC price observation #{i}",
        MemoryType.observation(),
        importance=0.5 + (i * 0.1)
    )
    time.sleep(0.1)

# Recall all observations
memories = memory.recall("price-watcher")
print(f"Recalled {len(memories)} memories:")
for mem in memories:
    print(f"  - {mem.content} (importance: {mem.importance:.2f})")

# Count total memories
count = memory.count("price-watcher")
print(f"\nTotal memories: {count}")
```

### Example 2: Multi-Agent Communication

```python
from neleus_core import MessageBus, MessageType
import json

# Initialize bus
bus = MessageBus()

# Register agents
bus.register("analyzer")
bus.register("trader")

# Analyzer subscribes to market data
bus.subscribe("analyzer", "market_updates")

# Trader subscribes to signals
bus.subscribe("trader", "trade_signals")

# Broadcast market update
bus.broadcast(
    "data_feed",
    "market_updates",
    MessageType.data_response(),
    {
        "symbol": "BTC-PERP",
        "price": 95000.0,
        "volume": 1250.0
    }
)

# Analyzer sends trading signal
bus.broadcast(
    "analyzer",
    "trade_signals",
    MessageType.signal_share(),
    {
        "symbol": "BTC-PERP",
        "signal": "buy",
        "confidence": 0.85,
        "reasoning": "Strong momentum breakout"
    }
)

# Check messages
print(f"Analyzer has {bus.pending_count('analyzer')} pending messages")
print(f"Trader has {bus.pending_count('trader')} pending messages")
```

### Example 3: Tool Execution

```python
from neleus_core import ToolRegistry

# Initialize tools
tools = ToolRegistry()

# List available tools
print("Available tools:", tools.list_tools())

# Execute market data tool
result = tools.execute("get_market_data", {
    "symbol": "BTC-PERP",
    "timeframe": "1h",
    "limit": 100
})

if result["success"]:
    print(f"Execution time: {result['execution_time_ms']}ms")
    print(f"Output: {result['output']}")
else:
    print(f"Error: {result['error']}")

# Get tool schemas for LLM
openai_schemas = tools.openai_schemas()
anthropic_schemas = tools.anthropic_schemas()
print(f"OpenAI schemas: {openai_schemas}")
```

---

## Best Practices

### 1. Memory Management

- **Use appropriate memory types** for different kinds of information
- **Set importance scores** to prioritize critical memories
- **Query efficiently** with filters and limits
- **Clean up old memories** periodically

```python
# Good: Specific query with filters
memories = memory.recall(
    "agent-1",
    memory_type=MemoryType.decision(),
    limit=10
)

# Bad: Retrieving all memories
memories = memory.recall("agent-1", limit=10000)
```

### 2. Agent Communication

- **Subscribe to relevant topics** only
- **Use appropriate message types** for clarity
- **Handle message priorities** for critical alerts
- **Unregister agents** when shutting down

```python
# Good: Specific topic subscription
bus.subscribe("agent-1", "btc_signals")

# Bad: Too broad
bus.subscribe("agent-1", "all_signals")
```

### 3. Tool Usage

- **Validate inputs** before tool execution
- **Handle errors gracefully** with try-catch
- **Monitor execution time** for performance
- **Cache results** when appropriate

```python
# Good: Error handling
try:
    result = tools.execute("get_market_data", params)
    if result["success"]:
        process_data(result["output"])
    else:
        log_error(result["error"])
except Exception as e:
    log_exception(e)
```

### 4. Risk Management

- **Always set position limits** in info.yaml
- **Use stop losses** on all positions
- **Monitor daily P&L** against limits
- **Test in paper mode** first

### 5. LLM Integration

- **Use formatters** for consistent data presentation
- **Provide context** from memory system
- **Validate LLM outputs** before execution
- **Monitor token usage** and costs

### 6. Performance

- **Use Rust core** when available (10-100x faster)
- **Batch operations** where possible
- **Profile bottlenecks** with timing
- **Scale horizontally** with multiple agents

---

## Troubleshooting

### Common Issues

**Problem**: `ImportError: cannot import name 'MemoryType' from 'neleus_core'`

**Solution**: Rebuild the Rust core:
```bash
cd crates/pybridge
maturin develop --release
```

**Problem**: Agent not making decisions

**Solution**: Check LLM API key and logs:
```bash
export OPENAI_API_KEY="sk-..."
tail -f logs/agent.log
```

**Problem**: Permission denied when listing agents

**Solution**: Fixed in latest version. Skip inaccessible directories automatically.

---

## Resources

- **Documentation**: [docs/](../docs/)
- **Examples**: [examples/](../examples/)
- **Architecture**: [docs/ARCHITECTURE.md](ARCHITECTURE.md)
- **API Reference**: [docs/API_REFERENCE.md](API_REFERENCE.md)
- **GitHub**: https://github.com/auralshin/neleus

---

## Support

For issues, questions, or contributions:

1. Check the [documentation](../docs/)
2. Search [existing issues](https://github.com/auralshin/neleus/issues)
3. Create a [new issue](https://github.com/auralshin/neleus/issues/new)
4. Join our community discussions

---

**Version**: 0.1.0  
**Last Updated**: February 2, 2026  
**License**: Apache-2.0
