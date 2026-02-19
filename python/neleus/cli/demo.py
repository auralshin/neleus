"""
Neleus CLI Demo Commands

Interactive demos for showcasing Neleus capabilities:
- AI Agent demo with Ollama
- Visual trading dashboard
- Multi-agent competition

All demos use real data from Hyperliquid via Rust core.
"""

import asyncio
import logging
from typing import Optional, List
from pathlib import Path

import typer

logger = logging.getLogger(__name__)
from rich.console import Console
from rich.prompt import Prompt, Confirm, IntPrompt
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

console = Console()

demo_app = typer.Typer(
    name="demo",
    help="Interactive demos and visualizations",
    no_args_is_help=True,
)


def _check_ollama() -> bool:
    """Check if Ollama is running."""
    try:
        import urllib.request
        import json
        with urllib.request.urlopen("http://localhost:11434/api/tags", timeout=2.0) as response:
            return response.status == 200
    except Exception as e:
        logger.debug("Ollama not reachable: %s", e)
        return False


def _get_available_models() -> List[str]:
    """Get list of available Ollama models."""
    try:
        import urllib.request
        import json
        with urllib.request.urlopen("http://localhost:11434/api/tags", timeout=5.0) as response:
            if response.status == 200:
                data = json.loads(response.read().decode())
                return [m["name"] for m in data.get("models", [])]
    except Exception as e:
        logger.debug("Could not fetch Ollama models: %s", e)
    return []


def _get_available_symbols() -> List[str]:
    """Get available trading symbols from Hyperliquid."""
    try:
        from neleus_core import HyperliquidClient
        client = HyperliquidClient(testnet=False)
        meta = client.fetch_meta()
        return meta.symbol_names()
    except Exception as e:
        logger.debug("Could not fetch Hyperliquid symbols: %s", e)
        return ["BTC", "ETH", "SOL"]  # Fallback


def _print_demo_header(title: str):
    """Print a demo header."""
    console.print()
    console.print("═" * 70, style="bold blue")
    console.print(f"🚀 {title}", style="bold yellow", justify="center")
    console.print("═" * 70, style="bold blue")
    console.print()


@demo_app.command("agent")
def agent_demo(
    model: Optional[str] = typer.Option(
        None, "--model", "-m",
        help="Ollama model to use (e.g., llama3.1:8b, mistral, codellama)"
    ),
    interactive: bool = typer.Option(
        False, "--interactive", "-i",
        help="Run in interactive chat mode"
    ),
    reasoning: bool = typer.Option(
        False, "--reasoning", "-r",
        help="Show detailed reasoning output"
    ),
    charts: bool = typer.Option(
        False, "--charts", "-c",
        help="Show visual trading dashboard"
    ),
):
    """
    Run AI trading agent demo with Ollama.
    
    Examples:
        neleus demo agent --model llama3.1:8b --reasoning
        neleus demo agent --charts
        neleus demo agent --interactive
    """
    _print_demo_header("NELEUS AI TRADING AGENT DEMO")
    
    # Check Ollama
    console.print("[yellow]Checking Ollama availability...[/yellow]")
    if not _check_ollama():
        console.print("[red]✗ Ollama is not running![/red]")
        console.print()
        console.print("Please start Ollama first:")
        console.print("  [cyan]ollama serve[/cyan]")
        console.print()
        console.print("Then pull a model:")
        console.print("  [cyan]ollama pull llama3.1:8b[/cyan]")
        raise typer.Exit(1)
    
    console.print("[green]✓ Ollama is running[/green]")
    
    # Model selection
    if model is None:
        available_models = _get_available_models()
        if available_models:
            console.print()
            console.print("[bold]Available models:[/bold]")
            for i, m in enumerate(available_models[:10], 1):
                console.print(f"  {i}. [cyan]{m}[/cyan]")
            console.print()
            model = Prompt.ask(
                "Select model",
                default=available_models[0] if available_models else "llama3.1:8b"
            )
        else:
            model = Prompt.ask("Enter model name", default="llama3.1:8b")
    
    console.print(f"[green]✓ Using model: {model}[/green]")
    
    # Mode selection if not specified
    if not any([interactive, reasoning, charts]):
        console.print()
        console.print("[bold]Select demo mode:[/bold]")
        console.print("  1. [cyan]Reasoning[/cyan] - Detailed AI reasoning output")
        console.print("  2. [cyan]Charts[/cyan]    - Visual trading dashboard")
        console.print("  3. [cyan]Interactive[/cyan] - Chat with the agent")
        console.print()
        choice = Prompt.ask("Choose mode", choices=["1", "2", "3"], default="1")
        
        if choice == "1":
            reasoning = True
        elif choice == "2":
            charts = True
        else:
            interactive = True
    
    console.print()
    
    # Run the selected demo
    if charts:
        console.print("[bold magenta]Starting Visual Charts Demo...[/bold magenta]")
        from neleus.ai.rich_ui import run_visual_demo_with_ui
        asyncio.run(run_visual_demo_with_ui(model=model))
    elif reasoning:
        console.print("[bold magenta]Starting Reasoning Demo...[/bold magenta]")
        from neleus.ai.demo_runner import run_reasoning_demo
        asyncio.run(run_reasoning_demo(model=model))
    else:
        console.print("[bold magenta]Starting Interactive Demo...[/bold magenta]")
        from neleus.ai.ollama_demo import run_interactive_demo
        asyncio.run(run_interactive_demo(model=model))


@demo_app.command("competition")
def competition_demo(
    symbols: Optional[str] = typer.Option(
        None, "--symbols", "-s",
        help="Trading symbols, comma-separated (e.g., 'BTC,ETH,SOL')"
    ),
    rounds: int = typer.Option(
        20, "--rounds", "-r",
        help="Number of competition rounds"
    ),
    backtest: bool = typer.Option(
        False, "--backtest", "-b",
        help="Use historical data for faster results"
    ),
    delay: float = typer.Option(
        2.0, "--delay", "-d",
        help="Delay between rounds in seconds"
    ),
):
    """
    Run multi-agent trading competition.
    
    Three AI agents compete with different strategies:
    - MomentumMax (trend following)
    - ReversionRex (mean reversion)
    - VolatilityViper (volatility adaptive)
    
    Examples:
        neleus demo competition --backtest
        neleus demo competition -s BTC,ETH,SOL --rounds 30
        neleus demo competition --symbols BTC,ETH,SOL,ARB,DOGE --backtest
    """
    _print_demo_header("NELEUS AI AGENT COMPETITION")
    
    # Parse symbols if provided as comma/space separated string
    symbol_list: Optional[List[str]] = None
    if symbols is not None:
        # Support both comma and space separation
        symbol_list = [s.strip().upper() for s in symbols.replace(",", " ").split() if s.strip()]
    
    # Symbol selection
    if symbol_list is None:
        available = _get_available_symbols()
        console.print(f"[dim]Available markets: {len(available)} symbols[/dim]")
        console.print()
        
        # Show popular symbols
        popular = ["BTC", "ETH", "SOL", "ARB", "DOGE", "OP", "AVAX", "MATIC"]
        popular_available = [s for s in popular if s in available]
        
        console.print("[bold]Popular symbols:[/bold]")
        console.print(f"  [cyan]{', '.join(popular_available)}[/cyan]")
        console.print()
        
        symbols_input = Prompt.ask(
            "Enter symbols (comma or space-separated)",
            default="BTC,ETH,SOL"
        )
        symbol_list = [s.strip().upper() for s in symbols_input.replace(",", " ").split() if s.strip()]
    
    # Validate symbols
    available = _get_available_symbols()
    valid_symbols = [s for s in symbol_list if s in available]
    if not valid_symbols:
        console.print("[red]No valid symbols found![/red]")
        raise typer.Exit(1)
    
    if len(valid_symbols) < len(symbol_list):
        invalid = set(symbol_list) - set(valid_symbols)
        console.print(f"[yellow]Warning: Invalid symbols ignored: {invalid}[/yellow]")
    
    final_symbols = valid_symbols
    console.print(f"[green]✓ Trading symbols: {', '.join(final_symbols)}[/green]")
    
    # Mode selection - default to live if not specified
    if not backtest:
        console.print()
        console.print("[dim]Tip: Use --backtest for faster historical simulation[/dim]")
        backtest = Confirm.ask(
            "Use backtest mode? (7 days historical data)",
            default=False  # Default to live mode
        )
    
    # Rounds configuration
    console.print()
    console.print(f"[dim]Rounds: {rounds} | Delay: {delay}s | Mode: {'Backtest' if backtest else 'Live'}[/dim]")
    
    if Confirm.ask("Adjust settings?", default=False):
        rounds = IntPrompt.ask("Number of rounds", default=rounds)
        delay = float(Prompt.ask("Delay between rounds (seconds)", default=str(delay)))
    
    console.print()
    console.print("[bold magenta]Starting Competition...[/bold magenta]")
    console.print()
    
    # Run competition
    from neleus.ai.agent_competition import run_competition
    asyncio.run(run_competition(
        symbols=final_symbols,
        rounds=rounds,
        delay=delay,
        backtest=backtest,
    ))


@demo_app.command("markets")
def markets_demo():
    """
    Show live market data from Hyperliquid.
    
    Displays real-time prices, volatility, and market regime.
    """
    _print_demo_header("NELEUS LIVE MARKET DATA")
    
    try:
        from neleus_core import HyperliquidClient
    except ImportError:
        console.print("[red]Rust core not available![/red]")
        raise typer.Exit(1)
    
    console.print("[yellow]Fetching market data...[/yellow]")
    
    try:
        client = HyperliquidClient(testnet=False)
        meta = client.fetch_meta()
        symbols = meta.symbol_names()
        
        console.print(f"[green]✓ Found {len(symbols)} markets[/green]")
        console.print()
        
        # Fetch data for top symbols
        from datetime import datetime
        import numpy as np
        
        end_time = int(datetime.now().timestamp() * 1000)
        start_time = end_time - (24 * 3600000)
        
        table = Table(title="📊 Live Market Data", show_header=True, header_style="bold cyan")
        table.add_column("Symbol", style="cyan", width=8)
        table.add_column("Price", justify="right", width=12)
        table.add_column("24h Change", justify="right", width=12)
        table.add_column("Volatility", justify="right", width=10)
        table.add_column("Regime", justify="center", width=10)
        
        for symbol in symbols[:20]:  # Top 20
            try:
                candles = client.fetch_candles(symbol, "1h", start_time, end_time)
                if candles and len(candles) >= 10:
                    closes = [c.close for c in candles]
                    returns = np.diff(np.log(closes))
                    volatility = np.std(returns) * np.sqrt(24 * 365) * 100
                    change = (closes[-1] - closes[0]) / closes[0] * 100
                    
                    # Determine regime
                    if volatility > 80:
                        regime = "[red]EXTREME[/red]"
                    elif volatility > 50:
                        regime = "[yellow]HIGH[/yellow]"
                    elif volatility > 30:
                        regime = "[white]NORMAL[/white]"
                    else:
                        regime = "[green]LOW[/green]"
                    
                    change_style = "green" if change >= 0 else "red"
                    
                    table.add_row(
                        symbol,
                        f"${closes[-1]:,.2f}" if closes[-1] > 1 else f"${closes[-1]:.4f}",
                        f"[{change_style}]{change:+.2f}%[/{change_style}]",
                        f"{volatility:.0f}%",
                        regime,
                    )
            except Exception as e:
                logger.debug("Skipping symbol in market data display: %s", e)

        console.print(table)
        console.print()
        console.print(f"[dim]Data source: Hyperliquid Mainnet | Updated: {datetime.now().strftime('%H:%M:%S')}[/dim]")
        
    except Exception as e:
        console.print(f"[red]Error fetching data: {e}[/red]")
        raise typer.Exit(1)


@demo_app.command("volatility")
def volatility_demo(
    symbol: str = typer.Argument("BTC", help="Symbol to analyze"),
    hours: int = typer.Option(24, "--hours", "-h", help="Lookback hours"),
):
    """
    Analyze volatility for a specific symbol.
    
    Examples:
        neleus demo volatility BTC
        neleus demo volatility ETH --hours 48
    """
    _print_demo_header(f"VOLATILITY ANALYSIS: {symbol.upper()}")
    
    from neleus.ai.demo_tools import MonitorVolatilityTool, validate_symbol
    
    symbol = validate_symbol(symbol)
    console.print(f"[cyan]Analyzing {symbol} volatility ({hours}h lookback)...[/cyan]")
    console.print()
    
    tool = MonitorVolatilityTool()
    result = asyncio.run(tool.execute(instrument=symbol, lookback_hours=hours))
    
    if result.success:
        data = result.output
        
        # Create display
        vol_data = data.get("volatility", {})
        
        table = Table(show_header=False, box=None)
        table.add_column("Metric", style="dim")
        table.add_column("Value", style="bold")
        
        table.add_row("Current Price", f"${data.get('current_price', 0):,.2f}")
        table.add_row("24h Change", f"{data.get('price_change_24h_pct', 0):+.2f}%")
        table.add_row("", "")
        table.add_row("Realized Volatility (Ann.)", f"{vol_data.get('realized_annualized_pct', 0):.1f}%")
        table.add_row("Parkinson Volatility (Ann.)", f"{vol_data.get('parkinson_annualized_pct', 0):.1f}%")
        table.add_row("Recent vs Historical", f"{vol_data.get('recent_vs_historical_change_pct', 0):+.1f}%")
        table.add_row("", "")
        
        regime = data.get("regime", "unknown")
        regime_style = {
            "extreme": "bold red",
            "high": "yellow",
            "normal": "white",
            "low": "green",
        }.get(regime, "white")
        
        table.add_row("Volatility Regime", f"[{regime_style}]{regime.upper()}[/{regime_style}]")
        table.add_row("Risk Level", data.get("risk_level", "unknown").upper())
        
        if "forecast" in data:
            forecast = data["forecast"]
            table.add_row("", "")
            table.add_row("Forecasted Volatility", f"{forecast.get('ewma_forecast_pct', 0):.1f}%")
            table.add_row("Expected Range", forecast.get("expected_range", "N/A"))
        
        console.print(Panel(table, title=f"[cyan]{symbol} Volatility Analysis[/cyan]"))
        
        # Recommendations
        if "recommendations" in data:
            console.print()
            console.print("[bold]Recommendations:[/bold]")
            for rec in data["recommendations"]:
                console.print(f"  • {rec}")
    else:
        console.print(f"[red]Error: {result.error}[/red]")


@demo_app.command("backtest")
def backtest_demo(
    symbol: str = typer.Argument("BTC", help="Symbol to backtest"),
    strategy: str = typer.Option(
        "momentum", "--strategy", "-s",
        help="Strategy type: momentum, mean_reversion, breakout, rsi_based"
    ),
    days: int = typer.Option(30, "--days", "-d", help="Backtest period in days"),
    capital: float = typer.Option(100000, "--capital", "-c", help="Initial capital"),
):
    """
    Run a quick backtest on a strategy.
    
    Examples:
        neleus demo backtest BTC --strategy momentum --days 30
        neleus demo backtest ETH -s mean_reversion -d 60
    """
    from datetime import datetime, timedelta
    
    _print_demo_header(f"BACKTEST: {strategy.upper()} on {symbol.upper()}")
    
    from neleus.ai.demo_tools import RunBacktestTool, validate_symbol
    
    symbol = validate_symbol(symbol)
    
    end_date = datetime.now().strftime("%Y-%m-%d")
    start_date = (datetime.now() - timedelta(days=days)).strftime("%Y-%m-%d")
    
    console.print(f"[cyan]Running {strategy} backtest on {symbol}[/cyan]")
    console.print(f"[dim]Period: {start_date} to {end_date} | Capital: ${capital:,.0f}[/dim]")
    console.print()
    
    tool = RunBacktestTool()
    result = asyncio.run(tool.execute(
        instrument=symbol,
        strategy=strategy,
        start_date=start_date,
        end_date=end_date,
        initial_capital=capital,
    ))
    
    if result.success:
        data = result.output
        
        table = Table(show_header=False, box=None)
        table.add_column("Metric", style="dim", width=20)
        table.add_column("Value", style="bold", width=20)
        
        pnl = data.get("total_pnl", 0)
        pnl_style = "green" if pnl >= 0 else "red"
        
        table.add_row("Strategy", data.get("strategy", strategy))
        table.add_row("Period", data.get("period", f"{start_date} to {end_date}"))
        table.add_row("", "")
        table.add_row("Initial Capital", f"${data.get('initial_capital', capital):,.2f}")
        table.add_row("Final Balance", f"${data.get('final_balance', capital):,.2f}")
        table.add_row("Total P&L", f"[{pnl_style}]${pnl:+,.2f}[/{pnl_style}]")
        table.add_row("Return", f"[{pnl_style}]{data.get('return_pct', 0):+.2f}%[/{pnl_style}]")
        table.add_row("", "")
        table.add_row("Sharpe Ratio", f"{data.get('sharpe_ratio', 0):.2f}")
        table.add_row("Sortino Ratio", f"{data.get('sortino_ratio', 0):.2f}")
        table.add_row("Max Drawdown", f"{data.get('max_drawdown_pct', 0):.2f}%")
        table.add_row("", "")
        table.add_row("Total Trades", str(data.get("total_trades", 0)))
        table.add_row("Win Rate", f"{data.get('win_rate', 0):.1f}%")
        table.add_row("Profit Factor", f"{data.get('profit_factor', 0):.2f}")
        
        console.print(Panel(table, title=f"[cyan]Backtest Results[/cyan]"))
        console.print()
        console.print(f"[dim]Data source: {data.get('execution_source', 'unknown')}[/dim]")
    else:
        console.print(f"[red]Error: {result.error}[/red]")


@demo_app.callback()
def demo_callback():
    """
    Interactive demos and visualizations.

    Showcase Neleus capabilities with real market data.
    """
    pass


__all__ = ["demo_app"]
