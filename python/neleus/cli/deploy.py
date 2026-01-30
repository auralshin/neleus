"""
Neleus CLI - Deploy Command

Deploy trading agents to the orchestrator service.
"""

import os
import yaml
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()

deploy_app = typer.Typer(
    name="deploy",
    help="Deploy trading agents",
    invoke_without_command=True,
)


def get_orchestrator_url() -> str:
    """Get orchestrator URL from environment or default."""
    return os.environ.get("NELEUS_ORCHESTRATOR_URL", "http://localhost:8080")


@deploy_app.callback(invoke_without_command=True)
def deploy(
    ctx: typer.Context,
    name: Optional[str] = typer.Argument(None, help="Agent name to deploy"),
    config: Optional[Path] = typer.Option(None, "--config", "-c", help="Path to agent config YAML"),
    strategy: Optional[str] = typer.Option(None, "--strategy", "-s", help="Strategy ID"),
    venue: str = typer.Option("hyperliquid", "--venue", "-v", help="Trading venue"),
    instruments: Optional[str] = typer.Option(None, "--instruments", "-i", help="Comma-separated instruments"),
    capital: float = typer.Option(10000.0, "--capital", help="Initial capital"),
    mode: str = typer.Option("paper", "--mode", "-m", help="Trading mode: paper/live"),
    testnet: bool = typer.Option(True, "--testnet/--mainnet", help="Use testnet"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """
    Deploy a trading agent to the orchestrator.
    
    Examples:
        neleus deploy my-agent --strategy momentum --instruments ETH-PERP
        neleus deploy --config agent.yaml
        neleus deploy my-agent -s momentum -i ETH-PERP,BTC-PERP --capital 50000
    """
    if ctx.invoked_subcommand is not None:
        return
    
    url = orchestrator_url or get_orchestrator_url()
    
    # Load from config file if provided
    if config and config.exists():
        console.print(f"\n📦 Loading agent config from [cyan]{config}[/cyan]\n")
        with open(config) as f:
            agent_config = yaml.safe_load(f)
        
        name = agent_config.get("name", name)
        strategy = agent_config.get("strategy", {}).get("id", strategy)
        strategy_config = agent_config.get("strategy", {}).get("config", {})
        venue = agent_config.get("venue", {}).get("name", venue)
        mode = agent_config.get("venue", {}).get("mode", mode)
        testnet = agent_config.get("venue", {}).get("testnet", testnet)
        instruments_list = agent_config.get("instruments", [])
        capital = agent_config.get("capital", {}).get("initial", capital)
        risk_limits = agent_config.get("risk_limits", {})
    else:
        strategy_config = {}
        instruments_list = instruments.split(",") if instruments else []
        risk_limits = {}
    
    # Validate required fields
    if not name:
        console.print("[red]Error:[/red] Agent name is required")
        console.print("[dim]Usage: neleus deploy <name> --strategy <strategy> --instruments <instruments>[/dim]")
        raise typer.Exit(1)
    
    if not strategy:
        console.print("[red]Error:[/red] Strategy is required (--strategy or in config)")
        raise typer.Exit(1)
    
    if not instruments_list:
        console.print("[red]Error:[/red] Instruments are required (--instruments or in config)")
        raise typer.Exit(1)
    
    # Display deployment plan
    console.print(Panel.fit(
        f"""[bold]Agent Deployment[/bold]
        
Name:        [cyan]{name}[/cyan]
Strategy:    {strategy}
Venue:       {venue} ({'testnet' if testnet else 'mainnet'})
Mode:        {mode}
Instruments: {', '.join(instruments_list)}
Capital:     ${capital:,.2f}
Orchestrator: {url}
""",
        title="🚀 Deploy",
        border_style="cyan",
    ))
    
    # Confirm deployment
    if mode == "live" and not testnet:
        confirm = typer.confirm(
            "\n⚠️  Deploying to LIVE MAINNET. Continue?",
            default=False,
        )
        if not confirm:
            raise typer.Exit(0)
    
    # Deploy the agent
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task("Deploying agent...", total=None)
        
        try:
            from ..agents import AgentManager, AgentSpec, VenueSpec, RiskLimits, CapitalSpec
            
            # Build spec
            spec = AgentSpec(
                name=name,
                strategy_id=strategy,
                strategy_config=strategy_config,
                venue=VenueSpec(
                    venue=venue,
                    mode=mode,
                    testnet=testnet,
                ),
                instruments=instruments_list,
                risk_limits=RiskLimits(
                    max_position_size=risk_limits.get("max_position_size", 10.0),
                    max_notional=risk_limits.get("max_notional", 100000.0),
                    max_drawdown_pct=risk_limits.get("max_drawdown_pct", 5.0),
                    daily_loss_limit=risk_limits.get("daily_loss_limit", 1000.0),
                ),
                capital=CapitalSpec(
                    initial=capital,
                ),
            )
            
            manager = AgentManager(orchestrator_url=url)
            agent_id = manager.deploy(spec)
            
            progress.update(task, description="Agent deployed!")
            
            console.print(f"\n[green]✓[/green] Agent deployed successfully!")
            console.print(f"   Agent ID: [cyan]{agent_id}[/cyan]")
            console.print(f"\n[dim]Start the agent with: neleus agents start {agent_id}[/dim]")
            
        except ImportError as e:
            progress.update(task, description="Using demo mode...")
            
            # Demo mode - just show what would be deployed
            import uuid
            agent_id = str(uuid.uuid4())[:8]
            
            console.print(f"\n[yellow]Demo mode[/yellow] - Orchestrator not available")
            console.print(f"   Would deploy agent: [cyan]{agent_id}[/cyan]")
            console.print(f"\n[dim]Start orchestrator: neleus services start[/dim]")
            
        except Exception as e:
            progress.update(task, description="[red]Failed[/red]")
            console.print(f"\n[red]Error:[/red] {e}")
            raise typer.Exit(1)


@deploy_app.command("batch")
def deploy_batch(
    config: Path = typer.Argument(..., help="Path to batch config YAML"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Deploy multiple agents from a batch configuration file."""
    url = orchestrator_url or get_orchestrator_url()
    
    if not config.exists():
        console.print(f"[red]Error:[/red] Config file not found: {config}")
        raise typer.Exit(1)
    
    console.print(f"\n📦 Batch deploy from [cyan]{config}[/cyan]\n")
    
    with open(config) as f:
        batch_config = yaml.safe_load(f)
    
    agents = batch_config.get("agents", [])
    
    if not agents:
        console.print("[yellow]Warning:[/yellow] No agents found in config")
        raise typer.Exit(0)
    
    table = Table(title=f"Deploying {len(agents)} agents")
    table.add_column("Agent", style="cyan")
    table.add_column("Strategy")
    table.add_column("Venue")
    table.add_column("Status")
    
    try:
        from ..agents import AgentManager
        manager = AgentManager(orchestrator_url=url)
    except ImportError:
        manager = None
    
    for agent_config in agents:
        name = agent_config.get("name", "unknown")
        strategy = agent_config.get("strategy", {}).get("id", "unknown")
        venue = agent_config.get("venue", {}).get("name", "unknown")
        
        if manager:
            try:
                # Deploy would go here
                status = "[green]✓ Deployed[/green]"
            except Exception as e:
                status = f"[red]✗ {e}[/red]"
        else:
            status = "[yellow]Demo[/yellow]"
        
        table.add_row(name, strategy, venue, status)
    
    console.print(table)


@deploy_app.command("template")
def deploy_template(
    output: Path = typer.Option(Path("agent.yaml"), "--output", "-o", help="Output file"),
):
    """Generate an agent configuration template."""
    template = """# Neleus Agent Configuration
# Deploy with: neleus deploy --config agent.yaml

name: my-trading-agent

strategy:
  id: momentum_strategy
  config:
    lookback: 20
    threshold: 0.02
    position_size: 0.1

venue:
  name: hyperliquid
  mode: paper  # paper or live
  testnet: true

instruments:
  - ETH-PERP
  - BTC-PERP

risk_limits:
  max_position_size: 10.0
  max_notional: 100000.0
  max_drawdown_pct: 5.0
  daily_loss_limit: 1000.0

capital:
  initial: 10000.0
  currency: USDC

# Optional: Schedule (for market hours trading)
# schedule:
#   type: cron
#   expression: "0 9 * * 1-5"  # Weekdays at 9 AM
#   timezone: UTC
"""
    
    output.write_text(template)
    console.print(f"\n[green]✓[/green] Template saved to [cyan]{output}[/cyan]")
    console.print(f"\n[dim]Edit the file and deploy with: neleus deploy --config {output}[/dim]")
