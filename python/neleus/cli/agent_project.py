"""
Neleus CLI - Agent Project Scaffolding

Commands for creating and managing AI trading agent projects:
- neleus new-agent <name>  - Create a new agent project
- neleus agent run <name>  - Run an agent
- neleus agent list        - List agent projects
"""

import os
import sys
import shutil
from pathlib import Path
from datetime import datetime
from typing import Optional

import typer
from rich.console import Console
from rich.panel import Panel
from rich.syntax import Syntax
from rich.tree import Tree
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()

agent_project_app = typer.Typer(
    name="agent",
    help="Manage AI trading agent projects",
)


# =============================================================================
# Agent Project Templates
# =============================================================================

PERSONALITY_TEMPLATE = '''# Agent Personality Configuration
# This file defines how your agent thinks and behaves

name: "{agent_name}"
description: "An AI trading agent powered by Neleus"

# Trading Style
# Options: aggressive, balanced, conservative, scalping, swing, position
trading_style: balanced

# Risk Tolerance  
# Options: low, medium, high
risk_tolerance: medium

# Decision Speed
# Options: fast (quick decisions), deliberate (careful analysis), adaptive (context-dependent)
decision_speed: adaptive

# Behavioral Traits
# These traits influence the agent's decision-making process
traits:
  - analytical
  - data-driven
  - risk-aware
  - opportunistic

# Analysis Preferences
use_technical_analysis: true
use_fundamental_analysis: true
use_sentiment_analysis: false
prefer_momentum: false
prefer_mean_reversion: false

# Communication Style
verbose_reasoning: true
explain_decisions: true

# Custom System Prompt (optional)
# If provided, this overrides the auto-generated prompt
# system_prompt: |
#   You are a trading agent. Be careful with risk.
'''

INFO_TEMPLATE = '''# Agent Capabilities Configuration
# This file defines what your agent can do

version: "1.0.0"

# =============================================================================
# LLM Configuration
# =============================================================================

llm_provider: openai  # openai, anthropic, ollama
llm_model: gpt-4o
temperature: 0.7
max_tokens: 4096

# =============================================================================
# Available Tools
# =============================================================================

# These are the tools your agent can use
tools:
  - get_market_data    # Fetch current market prices and data
  - get_analysis       # Get technical analysis (RSI, MACD, Bollinger)
  - place_order        # Execute trades (market/limit orders)
  - get_signals        # Receive trading signals
  - query_memory       # Search agent's memory
  - get_portfolio      # View positions and P&L
  - send_message       # Communicate with other agents

# =============================================================================
# Trading Configuration
# =============================================================================

# Supported instruments (e.g., BTC-PERP, ETH-PERP)
instruments:
  - BTC-PERP
  - ETH-PERP

# Supported venues
venues:
  - hyperliquid

# Data feeds
data_feeds:
  - hyperliquid_candles
  - hyperliquid_trades

# =============================================================================
# Risk Limits
# =============================================================================

# Maximum position size as fraction of portfolio (0.1 = 10%)
max_position_size: 0.10

# Maximum daily loss as fraction of portfolio (0.05 = 5%)
max_daily_loss: 0.05

# Maximum leverage allowed
max_leverage: 5.0

# =============================================================================
# Timing Configuration
# =============================================================================

# How often to run the decision loop (seconds)
decision_interval_seconds: 60

# Maximum decisions per hour (rate limiting)
max_decisions_per_hour: 60

# =============================================================================
# Memory Configuration
# =============================================================================

# Memory backend: local (file), redis, postgres
memory_backend: local

# Vector store for semantic search (optional): chromadb, pinecone, none
vector_store: null

# =============================================================================
# Knowledge Sources
# =============================================================================

# External knowledge the agent can access
knowledge_sources:
  - historical_prices
  - technical_indicators
  # - news_feed
  # - social_sentiment
'''

MAIN_TEMPLATE = '''"""
{agent_name} - AI Trading Agent

Created with Neleus Agent Orchestrator
Motto: "Make Your Agent Trade Smarter"

This agent uses:
- Memory persistence for learning from past decisions
- Tool framework for market data, analysis, and execution
- LLM-powered reasoning for trading decisions
"""

import asyncio
import logging
from pathlib import Path
from typing import Any, Dict

from neleus.ai import (
    AIAgent,
    AgentConfig,
    MemoryType,
    MessageType,
    AgentMessage,
)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class {class_name}(AIAgent):
    """
    {agent_name} - Your custom AI trading agent.
    
    Customize the decision-making logic in the `decide` method.
    Use self.think() for LLM reasoning and self.execute_tool() for actions.
    """
    
    async def on_start(self) -> None:
        """Called when the agent starts. Initialize your custom state here."""
        logger.info(f"{{self.name}} starting up...")
        
        # Example: Load any saved state from memory
        past_decisions = await self.recall(
            "recent trading decisions",
            limit=5,
            memory_type=MemoryType.DECISION,
        )
        
        if past_decisions:
            logger.info(f"Loaded {{len(past_decisions)}} past decisions from memory")
        
        # Remember that we started
        await self.remember(
            f"Agent started at {{self._started_at}}",
            memory_type=MemoryType.CONTEXT,
        )
    
    async def on_stop(self) -> None:
        """Called when the agent stops. Cleanup here."""
        logger.info(f"{{self.name}} shutting down...")
        
        # Save any final state
        await self.remember(
            f"Agent stopped. Uptime: {{self.uptime:.0f}} seconds, Decisions: {{self._decision_count}}",
            memory_type=MemoryType.CONTEXT,
        )
    
    async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Make a trading decision given the current context.
        
        This is the heart of your agent. Use:
        - self.think() for LLM-powered reasoning
        - self.execute_tool() to take actions
        - self.remember() to store important observations
        - self.recall() to retrieve past experiences
        
        Args:
            context: Current market context (prices, signals, etc.)
        
        Returns:
            Decision dictionary with action and parameters
        """
        # Step 1: Gather current market data
        market_result = await self.execute_tool(
            "get_market_data",
            instrument=context.get("instrument", "BTC-PERP"),
            data_type="ticker",
        )
        
        if not market_result.success:
            logger.warning(f"Failed to get market data: {{market_result.error}}")
            return {{"action": "wait", "reason": "No market data"}}
        
        market_data = market_result.output
        
        # Step 2: Get technical analysis
        analysis_result = await self.execute_tool(
            "get_analysis",
            instrument=context.get("instrument", "BTC-PERP"),
            indicators=["rsi", "macd", "bollinger"],
        )
        
        analysis = analysis_result.output if analysis_result.success else {{}}
        
        # Step 3: Check current portfolio
        portfolio_result = await self.execute_tool("get_portfolio")
        portfolio = portfolio_result.output if portfolio_result.success else {{}}
        
        # Step 4: Recall similar past situations
        similar_situations = await self.recall(
            f"market conditions similar to RSI {{analysis.get('indicators', {{}}).get('rsi', 'unknown')}}",
            limit=3,
            memory_type=MemoryType.DECISION,
        )
        
        # Step 5: Use LLM to reason about the situation
        reasoning = await self.think(
            f\"\"\"
            Current market situation:
            - Instrument: {{context.get('instrument', 'BTC-PERP')}}
            - Price: ${{market_data.get('price', 'unknown')}}
            - RSI: {{analysis.get('indicators', {{}}).get('rsi', 'unknown')}}
            - MACD: {{analysis.get('indicators', {{}}).get('macd', {{}}).get('signal', 'unknown')}}
            - Overall signal: {{analysis.get('overall_signal', 'neutral')}}
            
            Current portfolio:
            - Equity: ${{portfolio.get('equity', 'unknown')}}
            - Unrealized P&L: ${{portfolio.get('unrealized_pnl', 'unknown')}}
            - Positions: {{len(portfolio.get('positions', []))}}
            
            Based on your trading style and risk tolerance, what action should you take?
            Options: buy, sell, hold, or wait
            
            Provide your reasoning and decision.
            \"\"\",
            tools=["place_order", "get_signals"],
        )
        
        logger.info(f"Agent reasoning: {{reasoning[:200]}}...")
        
        # Step 6: Parse the decision and remember it
        # (In a real implementation, you'd parse the LLM response more carefully)
        decision = {{
            "action": "hold",  # Default to hold
            "reasoning": reasoning,
            "market_data": market_data,
            "analysis": analysis,
        }}
        
        # Remember this decision
        await self.remember(
            f"Decision: {{decision['action']}} for {{context.get('instrument', 'BTC-PERP')}} "
            f"at ${{market_data.get('price', 'unknown')}}. Reasoning: {{reasoning[:100]}}",
            memory_type=MemoryType.DECISION,
            metadata={{
                "instrument": context.get("instrument"),
                "price": market_data.get("price"),
                "rsi": analysis.get("indicators", {{}}).get("rsi"),
            }},
        )
        
        self._decision_count += 1
        
        return decision
    
    async def on_market_data(self, data: Dict[str, Any]) -> None:
        """Called when new market data arrives."""
        # Store interesting observations
        if data.get("price_change_pct", 0) > 5:
            await self.remember(
                f"Large price move detected: {{data.get('price_change_pct'):.1f}}% for {{data.get('instrument')}}",
                memory_type=MemoryType.OBSERVATION,
                metadata={{"importance": 0.8}},
            )
    
    async def on_signal(self, signal: Dict[str, Any]) -> None:
        """Called when a trading signal is received."""
        logger.info(f"Received signal: {{signal}}")
        
        # Evaluate the signal
        if signal.get("strength", 0) > 0.7:
            await self.remember(
                f"Strong {{signal.get('direction')}} signal for {{signal.get('instrument')}}",
                memory_type=MemoryType.OBSERVATION,
                metadata={{"signal": signal}},
            )
    
    async def on_message(self, message: AgentMessage) -> None:
        """Called when a message from another agent arrives."""
        logger.info(f"Message from {{message.from_agent}}: {{message.message_type.value}}")
        
        # Handle different message types
        if message.message_type == MessageType.SIGNAL_SHARE:
            # Another agent shared a signal
            await self.on_signal(message.payload)
        
        elif message.message_type == MessageType.DATA_REQUEST:
            # Another agent is requesting data
            # Respond with data if we have it
            pass
        
        elif message.message_type == MessageType.ALERT:
            # Important alert from another agent
            await self.remember(
                f"Alert from {{message.from_agent}}: {{message.payload.get('message')}}",
                memory_type=MemoryType.OBSERVATION,
                metadata={{"importance": 0.9}},
            )


async def main():
    """Run the agent."""
    # Load configuration from project directory
    project_path = Path(__file__).parent
    config = AgentConfig.from_project(project_path)
    
    # Create and start the agent
    agent = {class_name}(config)
    
    try:
        await agent.start()
        
        # Main decision loop
        while agent.is_running:
            try:
                # Get context (in production, this would come from real data)
                context = {{
                    "instrument": "BTC-PERP",
                    "timestamp": asyncio.get_event_loop().time(),
                }}
                
                # Make a decision
                decision = await agent.decide(context)
                logger.info(f"Decision: {{decision.get('action')}}")
                
                # Wait for next decision interval
                await asyncio.sleep(config.info.decision_interval_seconds)
                
            except KeyboardInterrupt:
                break
            except Exception as e:
                logger.error(f"Error in decision loop: {{e}}")
                await agent.on_error(e)
                await asyncio.sleep(10)  # Back off on error
        
    finally:
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
'''

README_TEMPLATE = '''# {agent_name}

An AI trading agent built with [Neleus Agent Orchestrator](https://github.com/neleus-hq/neleus).

**Motto: "Make Your Agent Trade Smarter"**

## Quick Start

1. **Configure your agent**
   - Edit `personality.yaml` to define behavior and trading style
   - Edit `info.yaml` to configure tools, instruments, and limits

2. **Set up environment variables**
   ```bash
   export OPENAI_API_KEY="your-key-here"  # or ANTHROPIC_API_KEY
   ```

3. **Run the agent**
   ```bash
   # From the project directory
   python main.py
   
   # Or using the CLI
   neleus agent run {project_name}
   ```

## Configuration Files

### personality.yaml
Defines how your agent thinks and behaves:
- Trading style (aggressive, balanced, conservative)
- Risk tolerance (low, medium, high)
- Analysis preferences (technical, fundamental, sentiment)
- Communication style

### info.yaml
Defines what your agent can do:
- LLM provider and model
- Available tools
- Supported instruments and venues
- Risk limits
- Memory configuration

## Customization

Edit `main.py` to customize the agent's decision-making logic:

```python
async def decide(self, context: Dict[str, Any]) -> Dict[str, Any]:
    # Your custom trading logic here
    pass
```

## Tools Available

| Tool | Description |
|------|-------------|
| `get_market_data` | Fetch prices, candles, orderbook |
| `get_analysis` | Technical indicators (RSI, MACD, Bollinger) |
| `place_order` | Execute market/limit orders |
| `get_signals` | Receive trading signals |
| `query_memory` | Search past decisions |
| `get_portfolio` | View positions and P&L |
| `send_message` | Communicate with other agents |

## Memory System

Your agent automatically persists:
- **Observations**: Market conditions, price moves
- **Decisions**: Trading decisions with reasoning
- **Actions**: Tool executions and results
- **Outcomes**: Trade results and P&L
- **Learnings**: Insights derived from experience

## Learn More

- [Neleus Documentation](https://github.com/neleus-hq/neleus)
- [Agent Development Guide](https://github.com/neleus-hq/neleus/docs/AGENTS.md)
'''

REQUIREMENTS_TEMPLATE = '''# Agent Dependencies
neleus>=0.1.0
openai>=1.0.0
anthropic>=0.18.0
aiohttp>=3.9.0
pyyaml>=6.0.0
'''


# =============================================================================
# CLI Commands
# =============================================================================

def create_agent_project(
    name: str,
    output_dir: Optional[Path] = None,
) -> Path:
    """Create a new agent project with all necessary files."""
    
    # Sanitize name
    project_name = name.lower().replace(" ", "_").replace("-", "_")
    agent_name = name.title().replace("_", " ").replace("-", " ")
    class_name = "".join(word.title() for word in project_name.split("_")) + "Agent"
    
    # Determine output directory
    if output_dir is None:
        output_dir = Path.cwd()
    
    project_dir = output_dir / project_name
    
    if project_dir.exists():
        raise ValueError(f"Directory already exists: {project_dir}")
    
    # Create directory structure
    project_dir.mkdir(parents=True)
    (project_dir / "data").mkdir()
    (project_dir / "logs").mkdir()
    (project_dir / "memory").mkdir()
    
    # Write files
    files = {
        "personality.yaml": PERSONALITY_TEMPLATE.format(agent_name=agent_name),
        "info.yaml": INFO_TEMPLATE,
        "main.py": MAIN_TEMPLATE.format(
            agent_name=agent_name,
            class_name=class_name,
        ),
        "README.md": README_TEMPLATE.format(
            agent_name=agent_name,
            project_name=project_name,
        ),
        "requirements.txt": REQUIREMENTS_TEMPLATE,
        ".gitignore": "data/\nlogs/\nmemory/\n__pycache__/\n*.pyc\n.env\n",
    }
    
    for filename, content in files.items():
        (project_dir / filename).write_text(content)
    
    return project_dir


@agent_project_app.command("new")
def new_agent(
    name: str = typer.Argument(..., help="Name for the new agent project"),
    output_dir: Optional[str] = typer.Option(None, "--output", "-o", help="Output directory"),
):
    """
    Create a new AI trading agent project.
    
    This creates a complete agent project with:
    - personality.yaml (agent behavior configuration)
    - info.yaml (capabilities and limits)
    - main.py (agent implementation)
    - README.md (documentation)
    """
    console.print()
    console.print(Panel.fit(
        "[bold cyan]Neleus Agent Orchestrator[/bold cyan]\n"
        "[dim]Make Your Agent Trade Smarter[/dim]",
        border_style="cyan",
    ))
    console.print()
    
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
    ) as progress:
        task = progress.add_task("Creating agent project...", total=None)
        
        try:
            out = Path(output_dir) if output_dir else None
            project_dir = create_agent_project(name, out)
            progress.update(task, description="Done!")
        except ValueError as e:
            console.print(f"\n[red]Error:[/red] {e}")
            raise typer.Exit(1)
    
    # Show created structure
    console.print(f"\n[green]✓[/green] Created agent project: [cyan]{project_dir}[/cyan]\n")
    
    tree = Tree(f"📁 {project_dir.name}")
    tree.add("📄 personality.yaml [dim](agent behavior)[/dim]")
    tree.add("📄 info.yaml [dim](capabilities & limits)[/dim]")
    tree.add("🐍 main.py [dim](agent implementation)[/dim]")
    tree.add("📄 README.md")
    tree.add("📄 requirements.txt")
    tree.add("📁 data/ [dim](market data cache)[/dim]")
    tree.add("📁 logs/ [dim](agent logs)[/dim]")
    tree.add("📁 memory/ [dim](persistent memory)[/dim]")
    
    console.print(tree)
    
    console.print("\n[bold]Next steps:[/bold]")
    console.print(f"  1. cd {project_dir.name}")
    console.print("  2. Edit [cyan]personality.yaml[/cyan] to customize behavior")
    console.print("  3. Edit [cyan]info.yaml[/cyan] to configure tools and limits")
    console.print("  4. Set [cyan]OPENAI_API_KEY[/cyan] environment variable")
    console.print("  5. Run [cyan]python main.py[/cyan] or [cyan]neleus agent run .[/cyan]")
    console.print()


@agent_project_app.command("run")
def run_agent(
    path: str = typer.Argument(".", help="Path to agent project directory"),
    verbose: bool = typer.Option(False, "--verbose", "-v", help="Verbose output"),
):
    """
    Run an AI trading agent.
    
    The agent will load configuration from personality.yaml and info.yaml,
    then start its decision loop.
    """
    project_path = Path(path).resolve()
    
    # Validate project
    if not (project_path / "personality.yaml").exists():
        console.print(f"[red]Error:[/red] Not an agent project: {project_path}")
        console.print("[dim]Missing personality.yaml[/dim]")
        raise typer.Exit(1)
    
    if not (project_path / "info.yaml").exists():
        console.print(f"[red]Error:[/red] Not an agent project: {project_path}")
        console.print("[dim]Missing info.yaml[/dim]")
        raise typer.Exit(1)
    
    console.print()
    console.print(Panel.fit(
        "[bold cyan]Neleus Agent Orchestrator[/bold cyan]\n"
        "[dim]Make Your Agent Trade Smarter[/dim]",
        border_style="cyan",
    ))
    console.print()
    console.print(f"Starting agent from: [cyan]{project_path}[/cyan]")
    console.print()
    
    # Run the agent
    main_file = project_path / "main.py"
    
    if main_file.exists():
        import subprocess
        
        env = os.environ.copy()
        if verbose:
            env["LOG_LEVEL"] = "DEBUG"
        
        try:
            subprocess.run(
                [sys.executable, str(main_file)],
                cwd=str(project_path),
                env=env,
            )
        except KeyboardInterrupt:
            console.print("\n[yellow]Agent stopped by user[/yellow]")
    else:
        console.print(f"[red]Error:[/red] main.py not found in {project_path}")
        raise typer.Exit(1)


@agent_project_app.command("list")
def list_agents(
    directory: str = typer.Argument(".", help="Directory to search for agent projects"),
):
    """
    List all agent projects in a directory.
    """
    search_dir = Path(directory).resolve()
    
    console.print(f"\nSearching for agent projects in: [cyan]{search_dir}[/cyan]\n")
    
    agents = []
    
    for item in search_dir.iterdir():
        try:
            if item.is_dir():
                if (item / "personality.yaml").exists() and (item / "info.yaml").exists():
                    # Load personality name
                    import yaml
                    with open(item / "personality.yaml") as f:
                        personality = yaml.safe_load(f)
                    
                    agents.append({
                        "path": item,
                        "name": personality.get("name", item.name),
                        "style": personality.get("trading_style", "unknown"),
                        "risk": personality.get("risk_tolerance", "unknown"),
                    })
        except (PermissionError, OSError):
            # Skip directories we can't access
            continue
    
    if not agents:
        console.print("[dim]No agent projects found[/dim]")
        return
    
    from rich.table import Table
    
    table = Table(title="Agent Projects")
    table.add_column("Name", style="cyan")
    table.add_column("Directory")
    table.add_column("Trading Style")
    table.add_column("Risk Tolerance")
    
    for agent in agents:
        table.add_row(
            agent["name"],
            str(agent["path"].name),
            agent["style"],
            agent["risk"],
        )
    
    console.print(table)
    console.print()


@agent_project_app.command("validate")
def validate_agent(
    path: str = typer.Argument(".", help="Path to agent project"),
):
    """
    Validate an agent project configuration.
    """
    import yaml
    
    project_path = Path(path).resolve()
    
    console.print(f"\nValidating agent project: [cyan]{project_path}[/cyan]\n")
    
    errors = []
    warnings = []
    
    # Check personality.yaml
    personality_file = project_path / "personality.yaml"
    if not personality_file.exists():
        errors.append("Missing personality.yaml")
    else:
        with open(personality_file) as f:
            personality = yaml.safe_load(f)
        
        required = ["name", "trading_style", "risk_tolerance"]
        for field in required:
            if field not in personality:
                errors.append(f"personality.yaml: Missing required field '{field}'")
    
    # Check info.yaml
    info_file = project_path / "info.yaml"
    if not info_file.exists():
        errors.append("Missing info.yaml")
    else:
        with open(info_file) as f:
            info = yaml.safe_load(f)
        
        required = ["llm_provider", "llm_model", "tools"]
        for field in required:
            if field not in info:
                errors.append(f"info.yaml: Missing required field '{field}'")
        
        if not info.get("instruments"):
            warnings.append("info.yaml: No instruments configured")
        
        if info.get("max_position_size", 0) > 0.5:
            warnings.append(f"info.yaml: High max_position_size ({info.get('max_position_size')})")
    
    # Check main.py
    main_file = project_path / "main.py"
    if not main_file.exists():
        errors.append("Missing main.py")
    
    # Check environment
    api_key_providers = {
        "openai": "OPENAI_API_KEY",
        "anthropic": "ANTHROPIC_API_KEY",
    }
    
    if info_file.exists():
        with open(info_file) as f:
            info = yaml.safe_load(f)
        
        provider = info.get("llm_provider", "openai")
        env_var = api_key_providers.get(provider)
        
        if env_var and not os.environ.get(env_var):
            warnings.append(f"Environment variable {env_var} not set")
    
    # Report results
    if errors:
        console.print("[red]Errors:[/red]")
        for error in errors:
            console.print(f"  ✗ {error}")
        console.print()
    
    if warnings:
        console.print("[yellow]Warnings:[/yellow]")
        for warning in warnings:
            console.print(f"  ⚠ {warning}")
        console.print()
    
    if not errors and not warnings:
        console.print("[green]✓ Agent project is valid![/green]")
    elif not errors:
        console.print("[green]✓ Agent project is valid (with warnings)[/green]")
    else:
        console.print("[red]✗ Agent project has errors[/red]")
        raise typer.Exit(1)
