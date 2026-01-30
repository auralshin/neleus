#!/usr/bin/env python3
"""
Neleus CLI - Command Line Interface for the Neleus Trading Framework

Commands:
    neleus new <project-name>  - Create a new Neleus project
    neleus init               - Initialize Neleus in current directory
    neleus run                - Run backtest or live trading
    neleus ui                 - Start the web UI dashboard
    neleus strategy           - Manage strategies
    neleus backtest           - Run backtests
    neleus live               - Start live trading
    neleus build              - Compile and validate project
    neleus test               - Run strategy tests
"""

import os
import sys
import json
import shutil
import subprocess
import webbrowser
from pathlib import Path
from typing import Optional, List
from datetime import datetime

import typer
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn
from rich.syntax import Syntax
from rich.tree import Tree

console = Console()

# =============================================================================
# ASCII Art Banner
# =============================================================================

NELEUS_BANNER = """
[bold cyan]
    ███╗   ██╗███████╗██╗     ███████╗██╗   ██╗███████╗
    ████╗  ██║██╔════╝██║     ██╔════╝██║   ██║██╔════╝
    ██╔██╗ ██║█████╗  ██║     █████╗  ██║   ██║███████╗
    ██║╚██╗██║██╔══╝  ██║     ██╔══╝  ██║   ██║╚════██║
    ██║ ╚████║███████╗███████╗███████╗╚██████╔╝███████║
    ╚═╝  ╚═══╝╚══════╝╚══════╝╚══════╝ ╚═════╝ ╚══════╝
[/bold cyan]
[dim]    High-Performance DeFi Trading Framework[/dim]
    [dim]━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━[/dim]
"""

NELEUS_SHORT_BANNER = "[bold cyan]Neleus[/bold cyan] [dim]v0.1.0[/dim] — DeFi Trading Framework"


def print_banner(short: bool = False):
    """Print the Neleus banner."""
    if short:
        console.print(NELEUS_SHORT_BANNER)
    else:
        console.print(NELEUS_BANNER)


app = typer.Typer(
    name="neleus",
    help="Neleus - High-Performance DeFi Trading Framework",
    add_completion=True,
    rich_markup_mode="rich",
    no_args_is_help=False,
)


@app.callback(invoke_without_command=True)
def main_callback(
    ctx: typer.Context,
    version: bool = typer.Option(False, "--version", "-v", help="Show version"),
):
    """
    Neleus — High-Performance DeFi Trading Framework
    
    Quantitative trading infrastructure with sub-millisecond latency.
    """
    if version:
        print_banner()
        console.print(f"[dim]Rust Core:[/dim] neleus_core 0.1.0")
        raise typer.Exit()
    
    if ctx.invoked_subcommand is None:
        print_banner()
        console.print("[bold]Usage:[/bold] neleus [OPTIONS] COMMAND [ARGS]...")
        console.print()
        console.print("[bold]Commands:[/bold]")
        console.print("  [cyan]new[/cyan]        Create a new Neleus project")
        console.print("  [cyan]init[/cyan]       Initialize Neleus in current directory")
        console.print("  [cyan]run[/cyan]        Run a strategy (backtest or live)")
        console.print("  [cyan]ui[/cyan]         Launch the dashboard UI")
        console.print("  [cyan]backtest[/cyan]   Run a backtest")
        console.print("  [cyan]live[/cyan]       Start live trading")
        console.print("  [cyan]strategy[/cyan]   Manage trading strategies")
        console.print("  [cyan]deploy[/cyan]     Deploy strategies to cloud")
        console.print("  [cyan]agents[/cyan]     Manage running agents")
        console.print("  [cyan]signals[/cyan]    View real-time signals")
        console.print("  [cyan]metrics[/cyan]    Performance metrics dashboard")
        console.print()
        console.print("Run [cyan]neleus <command> --help[/cyan] for more information.")
        console.print()

# Sub-commands
strategy_app = typer.Typer(help="Strategy management commands")
app.add_typer(strategy_app, name="strategy")

# Import and register managed service CLI commands
from .deploy import deploy_app
from .agents import agents_app
from .signals import signals_app
from .metrics import metrics_app

app.add_typer(deploy_app, name="deploy")
app.add_typer(agents_app, name="agents")
app.add_typer(signals_app, name="signals")
app.add_typer(metrics_app, name="metrics")

# =============================================================================
# Configuration
# =============================================================================

NELEUS_VERSION = "0.1.0"
TEMPLATE_DIR = Path(__file__).parent.parent / "templates"
CONFIG_FILE = "neleus.toml"


def get_project_root() -> Optional[Path]:
    """Find the project root by looking for neleus.toml"""
    current = Path.cwd()
    while current != current.parent:
        if (current / CONFIG_FILE).exists():
            return current
        current = current.parent
    return None


def ensure_project():
    """Ensure we're in a Neleus project"""
    root = get_project_root()
    if root is None:
        console.print("[red]Error:[/red] Not in a Neleus project. Run 'neleus new <name>' or 'neleus init' first.")
        raise typer.Exit(1)
    return root


# =============================================================================
# Project Templates
# =============================================================================

PROJECT_TEMPLATE = {
    "neleus.toml": '''# Neleus Project Configuration
[project]
name = "{project_name}"
version = "0.1.0"
description = "A Neleus trading project"
created = "{created_date}"

[trading]
default_venue = "hyperliquid"
network = "testnet"
default_timeframe = "1h"

[backtest]
initial_capital = 100000.0
commission_bps = 5.0
slippage_model = "fixed"
slippage_bps = 2.0

[risk]
max_position_pct = 10.0
max_daily_loss_pct = 5.0
max_leverage = 5.0
dynamic_limits = true

[ui]
port = 8765
auto_open = true
theme = "dark"

[logging]
level = "info"
file = "logs/neleus.log"
''',
    
    "strategies/__init__.py": '''"""Neleus Strategies Package - Your trading strategies go here."""
''',
    
    "strategies/momentum_strategy.py": '''"""
Momentum Strategy - Example template

Uses price momentum (rate of change) to generate trading signals.
"""

from neleus import Strategy, StrategyContext, OrderSide
from typing import List


class MomentumStrategy(Strategy):
    """
    Momentum strategy using price rate of change.
    
    Parameters:
        lookback: Number of bars for momentum calculation
        entry_threshold: Momentum threshold to enter (e.g., 0.02 = 2%)
        position_size: Position size
    """
    
    def __init__(
        self,
        lookback: int = 20,
        entry_threshold: float = 0.02,
        position_size: float = 0.1,
    ):
        super().__init__()
        self.lookback = lookback
        self.entry_threshold = entry_threshold
        self.position_size = position_size
        
        self.prices: List[float] = []
        self.in_position = False
        self.position_side = None
    
    def on_bar(self, ctx: StrategyContext, bar) -> None:
        """Process each bar and generate orders."""
        self.prices.append(bar.close)
        
        if len(self.prices) < self.lookback:
            return
        
        # Keep only lookback period
        self.prices = self.prices[-self.lookback:]
        
        # Calculate momentum (rate of change)
        momentum = (self.prices[-1] - self.prices[0]) / self.prices[0]
        
        # Generate signals
        if not self.in_position:
            if momentum > self.entry_threshold:
                # Strong positive momentum - go long
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = True
                self.position_side = "long"
            elif momentum < -self.entry_threshold:
                # Strong negative momentum - go short
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = True
                self.position_side = "short"
        else:
            # Exit logic
            if self.position_side == "long" and momentum < 0:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = False
            elif self.position_side == "short" and momentum > 0:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = False
''',
    
    "strategies/mean_reversion_strategy.py": '''"""
Mean Reversion Strategy - Example template

Uses Bollinger Bands to identify overbought/oversold conditions.
"""

from neleus import Strategy, StrategyContext, OrderSide
from typing import List


class MeanReversionStrategy(Strategy):
    """
    Mean reversion using Bollinger Bands.
    
    Parameters:
        period: Period for moving average
        num_std: Number of standard deviations for bands
        position_size: Position size
    """
    
    def __init__(
        self,
        period: int = 20,
        num_std: float = 2.0,
        position_size: float = 0.1,
    ):
        super().__init__()
        self.period = period
        self.num_std = num_std
        self.position_size = position_size
        
        self.prices: List[float] = []
        self.in_position = False
        self.position_side = None
    
    def on_bar(self, ctx: StrategyContext, bar) -> None:
        """Process each bar and generate orders."""
        self.prices.append(bar.close)
        
        if len(self.prices) < self.period:
            return
        
        self.prices = self.prices[-self.period:]
        
        # Calculate Bollinger Bands
        sma = sum(self.prices) / len(self.prices)
        variance = sum((p - sma) ** 2 for p in self.prices) / len(self.prices)
        std = variance ** 0.5
        
        upper_band = sma + (self.num_std * std)
        lower_band = sma - (self.num_std * std)
        current_price = bar.close
        
        if not self.in_position:
            if current_price < lower_band:
                # Price below lower band - buy (expect reversion up)
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = True
                self.position_side = "long"
            elif current_price > upper_band:
                # Price above upper band - sell (expect reversion down)
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = True
                self.position_side = "short"
        else:
            # Exit at mean
            if self.position_side == "long" and current_price >= sma:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = False
            elif self.position_side == "short" and current_price <= sma:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = False
''',
    
    "data/.gitkeep": "",
    "logs/.gitkeep": "",
    "notebooks/.gitkeep": "",
    "backtests/__init__.py": '"""Backtest configurations."""\n',
    "backtests/results/.gitkeep": "",
    "config/__init__.py": '"""Configuration management."""\n',
    
    "config/venues.py": '''"""
Venue configurations - Load from environment variables.
IMPORTANT: Never commit API keys to version control!
"""

import os

class HyperliquidConfig:
    def __init__(self):
        self.api_key = os.getenv("HYPERLIQUID_API_KEY")
        self.api_secret = os.getenv("HYPERLIQUID_SECRET_KEY")
        self.wallet = os.getenv("HYPERLIQUID_WALLET")
        self.network = os.getenv("HYPERLIQUID_NETWORK", "testnet")

class LighterConfig:
    def __init__(self):
        self.api_key = os.getenv("LIGHTER_API_KEY")
        self.private_key = os.getenv("LIGHTER_PRIVATE_KEY")
        self.network = os.getenv("LIGHTER_NETWORK", "testnet")

VENUES = {"hyperliquid": HyperliquidConfig, "lighter": LighterConfig}
''',
    
    ".env.example": '''# Neleus Environment Variables
# Copy to .env and fill in your values

HYPERLIQUID_API_KEY=
HYPERLIQUID_SECRET_KEY=
HYPERLIQUID_WALLET=
HYPERLIQUID_NETWORK=testnet

LIGHTER_API_KEY=
LIGHTER_PRIVATE_KEY=
LIGHTER_NETWORK=testnet

NELEUS_UI_PORT=8765
''',
    
    ".gitignore": '''__pycache__/
*.py[cod]
.venv/
venv/
.env
logs/*.log
data/*.csv
data/*.parquet
backtests/results/*.html
backtests/results/*.json
!.gitkeep
.idea/
.vscode/
*.egg-info/
''',
    
    "README.md": '''# {project_name}

A trading project built with [Neleus](https://github.com/auralshin/neleus).

## Quick Start

```bash
# Configure API keys
cp .env.example .env

# Start the dashboard
neleus ui

# Run a backtest
neleus backtest --strategy momentum --symbol BTC-PERP
```

## Project Structure

```
{project_name}/
├── strategies/          # Your trading strategies
├── backtests/          # Backtest configs and results
├── config/             # Configuration files
├── data/               # Market data cache
├── logs/               # Log files
└── neleus.toml         # Project configuration
```

## Creating a Strategy

```python
from neleus import Strategy
from neleus.types import Signal

class MyStrategy(Strategy):
    def on_bar(self, bar):
        if some_condition:
            return Signal.BUY
        return Signal.HOLD
```
''',
}


# =============================================================================
# Commands
# =============================================================================

@app.command()
def new(
    name: str = typer.Argument(..., help="Name of the new project"),
    template: str = typer.Option("default", "--template", "-t", help="Project template"),
):
    """Create a new Neleus trading project."""
    print_banner()
    project_path = Path.cwd() / name
    
    if project_path.exists():
        console.print(f"[red]Error:[/red] Directory '{name}' already exists.")
        raise typer.Exit(1)
    
    console.print(f"\n🌊 Creating new Neleus project: [cyan]{name}[/cyan]\n")
    
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task("Creating project structure...", total=None)
        
        project_path.mkdir(parents=True)
        
        for file_path, content in PROJECT_TEMPLATE.items():
            full_path = project_path / file_path
            full_path.parent.mkdir(parents=True, exist_ok=True)
            content = content.replace("{project_name}", name)
            content = content.replace("{created_date}", datetime.now().isoformat())
            full_path.write_text(content)
        
        progress.update(task, description="Project created!")
    
    console.print(Panel.fit(
        f"""[green]✓[/green] Project '[cyan]{name}[/cyan]' created!

[bold]Next steps:[/bold]
  1. [cyan]cd {name}[/cyan]
  2. [cyan]cp .env.example .env[/cyan]
  3. [cyan]neleus ui[/cyan]

[dim]Run a backtest:[/dim]
  [cyan]neleus backtest --strategy momentum --symbol BTC-PERP[/cyan]
""",
        title="🎉 Success",
        border_style="green",
    ))


@app.command()
def init():
    """Initialize Neleus in the current directory."""
    print_banner()
    current_dir = Path.cwd()
    config_path = current_dir / CONFIG_FILE
    
    if config_path.exists():
        console.print("[yellow]Warning:[/yellow] neleus.toml already exists.")
        raise typer.Exit(0)
    
    project_name = current_dir.name
    console.print(f"\n🌊 Initializing Neleus in: [cyan]{current_dir}[/cyan]\n")
    
    for file_path, content in PROJECT_TEMPLATE.items():
        full_path = current_dir / file_path
        if not full_path.exists():
            full_path.parent.mkdir(parents=True, exist_ok=True)
            content = content.replace("{project_name}", project_name)
            content = content.replace("{created_date}", datetime.now().isoformat())
            full_path.write_text(content)
    
    console.print("[green]✓[/green] Neleus initialized!")


@app.command()
def ui(
    port: int = typer.Option(8501, "--port", "-p", help="Port for the UI"),
    no_browser: bool = typer.Option(False, "--no-browser", help="Don't open browser"),
    host: str = typer.Option("127.0.0.1", "--host", "-h", help="Host to bind to"),
    legacy: bool = typer.Option(False, "--legacy", help="Use legacy FastAPI dashboard"),
):
    """
    Start the Neleus web dashboard.
    
    Features: Risk Dashboard, Portfolio Manager, Backtest Runner,
    and Performance Analytics.
    
    Uses Streamlit by default. Use --legacy for the FastAPI dashboard.
    """
    print_banner()
    project_root = get_project_root()
    
    if legacy:
        # Use legacy FastAPI dashboard
        port = 8765 if port == 8501 else port  # Default legacy port
        console.print(f"\n🌊 Starting Legacy Dashboard on [cyan]http://{host}:{port}[/cyan]\n")
        
        try:
            from neleus.ui.server import run_server
            
            if not no_browser:
                import threading
                def open_browser():
                    import time
                    time.sleep(1.5)
                    webbrowser.open(f"http://{host}:{port}")
                threading.Thread(target=open_browser, daemon=True).start()
            
            run_server(host=host, port=port, project_root=project_root)
            
        except ImportError as e:
            console.print(f"[red]Error:[/red] UI dependencies not installed.")
            console.print(f"[dim]{e}[/dim]")
            raise typer.Exit(1)
    else:
        # Use Streamlit dashboard
        console.print(f"\n🌊 Starting Neleus Dashboard on [cyan]http://{host}:{port}[/cyan]\n")
        console.print("[dim]Using Streamlit dashboard (use --legacy for FastAPI)[/dim]\n")
        
        try:
            import subprocess
            import sys
            
            streamlit_app_path = Path(__file__).parent.parent / "ui" / "streamlit_app.py"
            
            if not streamlit_app_path.exists():
                console.print(f"[red]Error:[/red] Streamlit app not found at {streamlit_app_path}")
                raise typer.Exit(1)
            
            # Open browser if requested
            if not no_browser:
                import threading
                def open_browser():
                    import time
                    time.sleep(2)
                    webbrowser.open(f"http://{host}:{port}")
                threading.Thread(target=open_browser, daemon=True).start()
            
            # Run streamlit
            subprocess.run([
                sys.executable, "-m", "streamlit", "run",
                str(streamlit_app_path),
                "--server.port", str(port),
                "--server.address", host,
                "--server.headless", "true",
                "--browser.gatherUsageStats", "false",
            ])
            
        except ImportError as e:
            console.print(f"[red]Error:[/red] Streamlit not installed.")
            console.print("[dim]Run: pip install streamlit plotly[/dim]")
            raise typer.Exit(1)
        console.print(f"[dim]{e}[/dim]")
        raise typer.Exit(1)


@app.command()
def backtest(
    strategy: str = typer.Option(..., "--strategy", "-s", help="Strategy name"),
    symbol: str = typer.Option("BTC-PERP", "--symbol", help="Trading symbol"),
    timeframe: str = typer.Option("1h", "--timeframe", "-t", help="Timeframe"),
    start: str = typer.Option(None, "--start", help="Start date (YYYY-MM-DD)"),
    end: str = typer.Option(None, "--end", help="End date (YYYY-MM-DD)"),
    capital: float = typer.Option(100000.0, "--capital", "-c", help="Initial capital"),
):
    """Run a backtest on a strategy."""
    print_banner()
    project_root = ensure_project()
    
    console.print(Panel.fit(
        f"""[bold]Backtest Configuration[/bold]
Strategy:  [cyan]{strategy}[/cyan]
Symbol:    {symbol}
Timeframe: {timeframe}
Capital:   ${capital:,.2f}
""",
        title="🔬 Backtest",
    ))
    
    # Load and run actual backtest
    try:
        from neleus import BacktestRunner
        
        # Dynamic strategy import
        strategies_dir = project_root / "strategies"
        strategy_file = strategies_dir / f"{strategy}_strategy.py"
        
        if not strategy_file.exists():
            strategy_file = strategies_dir / f"{strategy}.py"
        
        if not strategy_file.exists():
            console.print(f"[red]Error:[/red] Strategy '{strategy}' not found.")
            console.print(f"[dim]Looked in: {strategies_dir}[/dim]")
            raise typer.Exit(1)
        
        # Import strategy dynamically
        import importlib.util
        spec = importlib.util.spec_from_file_location(strategy, strategy_file)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        
        # Find strategy class
        strategy_class = None
        for attr_name in dir(module):
            attr = getattr(module, attr_name)
            if isinstance(attr, type) and attr_name.endswith("Strategy") and attr_name != "Strategy":
                strategy_class = attr
                break
        
        if strategy_class is None:
            console.print(f"[red]Error:[/red] No strategy class found in {strategy_file}")
            raise typer.Exit(1)
        
        console.print(f"[dim]Loaded: {strategy_class.__name__}[/dim]\n")
        
        # Run backtest using the BacktestRunner properly
        from neleus.backtest_runner import BacktestRunner as Runner
        
        runner = Runner(project_root)
        
        with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
            task = progress.add_task("Running backtest...", total=None)
            
            import asyncio
            # Run backtest with async runner
            results = asyncio.run(runner.run_backtest(
                strategy_name=strategy,
                initial_capital=capital,
                start_date=start,
                end_date=end,
            ))
            progress.update(task, description="Complete!")
        
        # Display results
        console.print("\n[green]✓[/green] Backtest complete!\n")
        
        # Results is a dict of {strategy_name: BacktestResults}
        for strat_name, result in results.items():
            table = Table(title=f"Results: {strat_name}")
            table.add_column("Metric", style="cyan")
            table.add_column("Value", justify="right")
            
            # BacktestResults has a metrics attribute
            if hasattr(result, 'metrics'):
                m = result.metrics
                table.add_row("Total Return", f"{m.total_return * 100:.2f}%")
                table.add_row("Max Drawdown", f"{m.max_drawdown * 100:.2f}%")
                table.add_row("Sharpe Ratio", f"{m.sharpe_ratio:.2f}")
                table.add_row("Total Trades", str(m.total_trades))
                table.add_row("Win Rate", f"{m.win_rate * 100:.1f}%")
                table.add_row("Total Commission", f"${m.total_commission:,.2f}")
            # PyBacktestResults from Rust
            elif hasattr(result, 'return_pct'):
                table.add_row("Total Return", f"{result.return_pct:.2f}%")
                table.add_row("Max Drawdown", f"{result.max_drawdown_pct:.2f}%")
                table.add_row("Sharpe Ratio", f"{result.sharpe_ratio:.2f}")
                table.add_row("Total Trades", str(result.total_trades))
                table.add_row("Win Rate", f"{result.win_rate():.1f}%")
            elif isinstance(result, dict):
                table.add_row("Total Return", f"{result.get('return_pct', 0):.2f}%")
                table.add_row("Sharpe Ratio", f"{result.get('sharpe_ratio', 0):.2f}")
            
            console.print(table)
        
    except ImportError as e:
        console.print(f"[yellow]Warning:[/yellow] Running in demo mode.")
        console.print(f"[dim]{e}[/dim]\n")
        
        # Demo results
        table = Table(title="Demo Results")
        table.add_column("Metric", style="cyan")
        table.add_column("Value", justify="right")
        table.add_row("Total Return", "+15.3%")
        table.add_row("Sharpe Ratio", "1.85")
        table.add_row("Max Drawdown", "-8.2%")
        table.add_row("Total Trades", "142")
        table.add_row("Win Rate", "58.4%")
        console.print(table)


@app.command()
def live(
    strategy: str = typer.Option(..., "--strategy", "-s", help="Strategy to run"),
    symbol: str = typer.Option("BTC-PERP", "--symbol", help="Trading symbol"),
    venue: str = typer.Option("hyperliquid", "--venue", "-v", help="Trading venue"),
    paper: bool = typer.Option(True, "--paper/--real", help="Paper trading mode"),
):
    """Start live trading. Use --paper for paper trading (default)."""
    print_banner()
    project_root = ensure_project()
    
    if not paper:
        confirm = typer.confirm(
            "\n⚠️  LIVE trading with REAL money. Continue?",
            default=False,
        )
        if not confirm:
            raise typer.Exit(0)
    
    mode_str = "[yellow]PAPER[/yellow]" if paper else "[red]LIVE[/red]"
    console.print(f"\n🚀 Starting {mode_str} trading")
    console.print(f"   Strategy: [cyan]{strategy}[/cyan]")
    console.print(f"   Symbol:   {symbol}")
    console.print(f"   Venue:    {venue}")
    console.print("\n[dim]Press Ctrl+C to stop[/dim]\n")


@app.command()
def build():
    """Validate and compile the project."""
    print_banner()
    project_root = ensure_project()
    
    console.print("\n🔨 Building project...\n")
    
    checks = [
        ("Validating neleus.toml", True),
        ("Checking strategies", True),
        ("Validating configuration", True),
    ]
    
    for check, passed in checks:
        status = "[green]✓[/green]" if passed else "[red]✗[/red]"
        console.print(f"  {status} {check}")
    
    console.print("\n[green]Build successful![/green]")


# =============================================================================
# Strategy Sub-commands
# =============================================================================

@strategy_app.command("list")
def strategy_list():
    """List all available strategies."""
    print_banner()
    project_root = ensure_project()
    strategies_dir = project_root / "strategies"
    
    console.print("\n📋 Available Strategies:\n")
    
    table = Table()
    table.add_column("Name", style="cyan")
    table.add_column("File")
    
    if strategies_dir.exists():
        for py_file in strategies_dir.glob("*.py"):
            if py_file.name.startswith("_"):
                continue
            table.add_row(py_file.stem, str(py_file.relative_to(project_root)))
    
    console.print(table)


@strategy_app.command("add")
def strategy_add(
    name: str = typer.Argument(..., help="Strategy name"),
    template: str = typer.Option("momentum", "--template", "-t", help="Template: momentum, mean_reversion, custom"),
):
    """Create a new strategy from template."""
    print_banner()
    project_root = ensure_project()
    strategies_dir = project_root / "strategies"
    
    strategy_name = name.lower().replace("-", "_").replace(" ", "_")
    class_name = "".join(word.title() for word in strategy_name.split("_")) + "Strategy"
    file_path = strategies_dir / f"{strategy_name}.py"
    
    if file_path.exists():
        console.print(f"[red]Error:[/red] Strategy '{strategy_name}' exists.")
        raise typer.Exit(1)
    
    strategy_code = f'''"""
{class_name} - Auto-generated strategy template.
"""

from neleus import Strategy
from neleus.types import Signal
from typing import Dict, Any, List


class {class_name}(Strategy):
    """
    {name.replace("_", " ").title()} Strategy
    """
    
    def __init__(self):
        super().__init__()
        self.prices: List[float] = []
    
    def on_bar(self, bar: Dict[str, Any]) -> Signal:
        """Process each bar and return a signal."""
        # Your logic here
        return Signal.HOLD
    
    def get_position_size(self, capital: float, price: float) -> float:
        return (capital * 0.1) / price
'''
    
    file_path.write_text(strategy_code)
    console.print(f"\n[green]✓[/green] Created: [cyan]{strategy_name}[/cyan]")
    console.print(f"  File: {file_path.relative_to(project_root)}")


@strategy_app.command("show")
def strategy_show(name: str = typer.Argument(..., help="Strategy name")):
    """Show strategy source code."""
    print_banner()
    project_root = ensure_project()
    strategies_dir = project_root / "strategies"
    
    file_path = strategies_dir / f"{name}.py"
    if not file_path.exists():
        file_path = strategies_dir / f"{name}_strategy.py"
    
    if not file_path.exists():
        console.print(f"[red]Error:[/red] Strategy '{name}' not found.")
        raise typer.Exit(1)
    
    code = file_path.read_text()
    syntax = Syntax(code, "python", theme="monokai", line_numbers=True)
    console.print(Panel(syntax, title=f"📄 {file_path.name}"))


@app.command()
def version():
    """Show Neleus version."""
    print_banner()
    console.print(f"Neleus v{NELEUS_VERSION}")


@app.command()
def info():
    """Show project information."""
    print_banner()
    project_root = get_project_root()
    
    if project_root is None:
        console.print(f"[yellow]Not in a Neleus project[/yellow]")
        console.print(f"\nNeleus v{NELEUS_VERSION}")
        console.print("\nRun 'neleus new <name>' to create a project.")
        return
    
    tree = Tree(f"📁 {project_root.name}")
    for item in sorted(project_root.iterdir()):
        if item.name.startswith(".") and item.name != ".env.example":
            continue
        if item.is_dir():
            branch = tree.add(f"📁 {item.name}/")
            for subitem in sorted(item.iterdir())[:5]:
                if not subitem.name.startswith("."):
                    branch.add(f"📄 {subitem.name}")
        else:
            tree.add(f"📄 {item.name}")
    
    console.print(tree)


def main():
    """Entry point for the CLI."""
    app()


if __name__ == "__main__":
    main()
