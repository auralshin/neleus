<p align="center">
  <img src="logo.png" alt="Neleus logo" width="200">
  <h3 align="center"><strong>Neleus</strong></h3>
  <p align="center"><em>Make Your Agent Trade Smarter</em></p>
  <p align="center">Agent Orchestrator Service for AI Trading Agents</p>
</p>

---

Neleus is an **AI Agent Orchestrator Service** for building intelligent trading agents. It combines a high-performance Rust core with a Python AI framework featuring:

- 🧠 **LLM-Powered Reasoning** - OpenAI, Anthropic, Ollama integration
- 💾 **Persistent Memory** - Agents remember and learn from past decisions  
- 🔧 **Tool Framework** - Market data, analysis, execution, and more
- 📡 **Agent Communication** - Agents can share signals and coordinate
- 📊 **Data Formatting** - Optimal data presentation for LLM consumption

## 🚀 Quick Start - AI Agent

```bash
# Create a new AI trading agent
neleus new-agent my_trading_agent
cd my_trading_agent

# Configure your agent
# - Edit personality.yaml (trading style, risk tolerance)
# - Edit info.yaml (LLM provider, tools, instruments)

# Set your API key
export OPENAI_API_KEY="your-key-here"

# Run the agent
python main.py
```

### Example Agent

```python
from neleus.ai import AIAgent, AgentConfig, MemoryType

class MyTradingAgent(AIAgent):
    async def decide(self, context):
        # Get market data
        data = await self.execute_tool("get_market_data", instrument="BTC-PERP")
        
        # Think with LLM
        reasoning = await self.think(f"Analyze {data}. Should I buy, sell, or hold?")
        
        # Remember this decision
        await self.remember(
            f"Decided to {reasoning}",
            memory_type=MemoryType.DECISION
        )
        
        return {"action": "hold", "reasoning": reasoning}
```

## 📚 Documentation

**Complete documentation is now available:**
- 🚀 [**Getting Started**](./docs/GETTING_STARTED.md) - Installation, setup, and first strategy
- � [**Usage Guide**](./docs/USAGE.md) - **NEW!** Comprehensive usage documentation
- �📖 [**API Reference**](./docs/API_REFERENCE.md) - Complete API documentation
- ⌨️ [**CLI Reference**](./docs/CLI_REFERENCE.md) - Command-line tools
- 💡 [**Examples**](./docs/EXAMPLES.md) - Strategy examples and tutorials
- ⚙️ [**Configuration**](./docs/CONFIGURATION.md) - Setup and configuration
- ⚡ [**Quick Reference**](./docs/QUICK_REFERENCE.md) - Cheat sheet
- 🏗️ [**Architecture**](./docs/ARCHITECTURE.md) - System design

**[📖 View Full Documentation Index](./docs/INDEX.md)**


### Installation

#### Prerequisites
- Python 3.8+
- Rust toolchain (for building the extension)

#### Install from Source

```bash
# Clone the repository
git clone https://github.com/auralshin/neleus.git
cd neleus

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install maturin (builds Rust extensions for Python)
pip install maturin

# Build and install the Rust extension
cd crates/pybridge
maturin develop --release
cd ../..

# Install Python dependencies
pip install -e python/
```

The Rust extension (`neleus_core`) is **required** - Neleus does not have a Python fallback.

### Create a Project

```bash
neleus create my_trading_project
cd my_trading_project
```

### Configure Credentials

Edit `.env` with your API keys:

```bash
HYPERLIQUID_PRIVATE_KEY=your_key_here
HYPERLIQUID_TESTNET=true
```

### Create a Strategy

```bash
neleus new strategy --name MyMomentum
```

### Edit Your Strategy

```python
# strategies/my_momentum.py
from neleus import Strategy, StrategyContext, Bar, OrderSide

class MyMomentum(Strategy):
    def __init__(self, lookback=20, threshold=0.02):
        super().__init__("MyMomentum")
        self.lookback = lookback
        self.threshold = threshold
        self.prices = []
    
    def on_data(self, ctx: StrategyContext, data):
        if isinstance(data, Bar):
            self.prices.append(float(data.close))
            if len(self.prices) >= self.lookback:
                momentum = (self.prices[-1] - self.prices[-self.lookback]) / self.prices[-self.lookback]
                if momentum > self.threshold:
                    ctx.market_order(data.instrument_id, OrderSide.BUY, 0.1)
```

### Run Backtest

```bash
neleus backtest
```

### Open Dashboard

```bash
neleus ui
```

Opens a web dashboard at `http://localhost:8080`:

- View price charts and orderbook
- Configure strategy parameters
- Run backtests with visual results
- Monitor positions and PnL

## Project Structure

After running `neleus create`:

```
my_project/
├── .env              # API keys (never commit!)
├── .env.example      # Example credentials
├── .gitignore        # Pre-configured
├── config.yaml       # Project configuration
├── run_backtest.py   # Backtest runner
├── strategies/       # Your strategies
├── configs/          # Strategy configs
├── data/             # Market data cache
├── logs/             # Application logs
└── reports/          # Backtest reports
```

## Configuration

### Project Config (`config.yaml`)

```yaml
project:
  name: "my_project"

venues:
  hyperliquid:
    enabled: true
    mode: paper  # backtest | paper | live

instruments:
  - symbol: BTC
    venue: hyperliquid
    type: perp

risk:
  max_position_size: 1.0
  max_drawdown_pct: 10.0
  daily_loss_limit: 500.0

backtest:
  start_date: "2024-01-01"
  end_date: "2024-12-31"
  initial_capital: 10000.0
```

### Environment Variables (`.env`)

```bash
# Hyperliquid
HYPERLIQUID_PRIVATE_KEY=
HYPERLIQUID_TESTNET=true

# Lighter
LIGHTER_API_KEY=
LIGHTER_API_SECRET=
LIGHTER_TESTNET=true

# Overrides
NELEUS_LOG_LEVEL=INFO
NELEUS_MAX_POSITION_SIZE=1.0
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `neleus create [name]` | Create a new project |
| `neleus new strategy --name NAME` | Create a new strategy |
| `neleus backtest` | Run backtest |
| `neleus backtest --strategy NAME` | Backtest specific strategy |
| `neleus run --mode paper` | Start paper trading |
| `neleus run --mode live` | Start live trading |
| `neleus ui` | Open web dashboard |
| `neleus config show` | Show configuration |
| `neleus config validate` | Validate configuration |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   AI Agent Layer                            │
│  LLM Reasoning • Memory • Tools • Communication            │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                     Python Layer                            │
│  Strategies • Indicators • Config • Research • Reporting   │
└─────────────────────────────────────────────────────────────┘
                              │
                    PyO3 Bridge (thin)
                              │
┌─────────────────────────────────────────────────────────────┐
│                      Rust Core                              │
│  Engine • Order State Machine • Risk • Positions • PnL    │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
         ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
         │Hyperliquid│   │ Lighter │    │  More   │
         │ Adapter   │   │ Adapter │    │ Venues  │
         └──────────┘    └─────────┘    └─────────┘
```

## Documentation

- [Python Package](python/README.md) - Strategy development guide
- [Architecture](docs/ARCHITECTURE.md) - System design and data flow
- [Status](docs/STATUS.md) - Component completion status and recent changes

## Supported Venues

| Venue | Market Data | Order Execution | Status |
|-------|-------------|-----------------|--------|
| Hyperliquid |  |  | Production Ready |
| Lighter |  |  | Production Ready |
| Polymarket |  |  | Production Ready |

## Key Features

### 🤖 AI Agent Framework
- **AIAgent Base Class** - Abstract base for building intelligent trading agents
- **Memory System** - Persistent memory with semantic search capabilities
- **Tool Framework** - Extensible tools for market data, analysis, execution
- **LLM Integration** - OpenAI, Anthropic, Ollama with provider abstraction
- **Agent Communication** - Message bus for multi-agent coordination

### ⚡ Trading Infrastructure
- **Rust Core Engine** - High-performance event loop and order matching
- **Python Strategy Layer** - Write strategies with NumPy, Pandas, SciPy
- **Unified Adapter System** - ExecutionClient and DataClient traits
- **Full Backtesting** - Position tracking, P&L calculation, performance metrics
- **Risk Management** - Dynamic limits, drawdown protection, leverage controls
- **Event Persistence** - TimescaleDB integration for replay and analysis

## License

MIT License
