#!/usr/bin/env python3
"""
Neleus CLI

Minimal Hyperliquid-first CLI surface:
- market analysis
- backtesting
- project scaffolding
- strategy runtime (one-shot or daemon)
"""

from __future__ import annotations

import asyncio
from datetime import datetime
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console
from rich.live import Live
from rich.syntax import Syntax

from .. import __version__
from ..backtest_runner import BacktestRunner
from ..config import discover_strategies, get_db_config, get_hyperliquid_credentials, load_project_config
from ..market import (
    analyze_market,
    list_markets,
    open_l2_book_stream,
    resolve_market_entry,
    scan_markets,
)
from ..runtime import run_project_daemon, run_project_once
from .ui import (
    DaemonDashboard,
    render_about_panel,
    render_backtest_result,
    render_brand_banner,
    render_l2_book,
    render_market_analysis,
    render_market_catalog,
    render_market_scan,
    render_project_created,
    render_project_info,
    render_runtime_result,
    render_strategy_list,
)

console = Console()

app = typer.Typer(
    name="neleus",
    help="Hyperliquid-first trading CLI for market analysis, backtesting, and strategy runtimes.",
    add_completion=True,
    no_args_is_help=False,
)
market_app = typer.Typer(help="Search, analyze, scan, and monitor Hyperliquid markets.")
strategy_app = typer.Typer(help="Manage project strategy files.")
db_app = typer.Typer(help="Database adapter management (status, schema init).")
app.add_typer(market_app, name="market")
app.add_typer(strategy_app, name="strategy")
app.add_typer(db_app, name="db")

CONFIG_FILE = "neleus.toml"

PROJECT_TEMPLATE = {
    ".gitignore": """# Local secrets
.env

# Python cache
__pycache__/
strategies/__pycache__/
configs/__pycache__/
""",
    "neleus.toml": """[project]
name = "{project_name}"
version = "0.1.0"
created_at = "{created_at}"

[hyperliquid]
testnet = false

[market]
symbol = "BTC-PERP"
timeframe = "1h"
lookback_bars = 200

[backtest]
initial_capital = 10000.0
maker_fee_bps = 2.0
taker_fee_bps = 5.0
slippage_bps = 5.0

[runtime]
mode = "once"
poll_interval_seconds = 60

# ---------------------------------------------------------------------------
# Database adapter
# ---------------------------------------------------------------------------
# backend: "none" | "postgres" | "timescale"
#   none      - no persistence (default)
#   postgres  - PostgreSQL event store + trade monitoring
#   timescale - TimescaleDB hypertables for market data + trade monitoring
#
# dsn: PostgreSQL connection string.
#   Keep this empty in committed config if it contains credentials.
#   Prefer storing secrets in a local .env via NELEUS_DB_DSN.
#
# trade_monitoring: when true, strategy-generated orders are automatically
#   recorded to hl_orders / hl_fills tables via the TradeMonitor adapter.
# ---------------------------------------------------------------------------
[database]
backend = "{db_backend}"
dsn = "{db_dsn}"
pool_size = 4
batch_size = 1000
flush_interval_ms = 100
trade_monitoring = {trade_monitoring}
""",
    ".env.example": """# Hyperliquid
# Public market data uses the /info API and does not require credentials.
HYPERLIQUID_TESTNET=false

# Signed trading actions use /exchange with a wallet or API-wallet signature.
# Required for live order execution (neleus run --live).
# Copy this file to .env and fill in your values — never commit .env.
HYPERLIQUID_ACCOUNT_ADDRESS=0x...
HYPERLIQUID_SIGNER_PRIVATE_KEY=0x...

# Database adapter
# Override the database.backend from neleus.toml
NELEUS_DB_BACKEND=none

# PostgreSQL / TimescaleDB connection string.
# Prefer storing real credentials in a local .env file, not in neleus.toml.
# If set, this takes precedence over database.dsn in neleus.toml.
# Example: postgresql://user:password@localhost:5432/neleus
NELEUS_DB_DSN=
""",
    "main.py": '''from pathlib import Path

from neleus.runtime import run_project_once


if __name__ == "__main__":
    result = run_project_once(Path(__file__).parent)
    print(result.to_dict())
''',
    "strategies/__init__.py": '"""Project strategies."""\n',
    "strategies/momentum.py": '''from neleus import OrderSide, Strategy, StrategyContext, Bar


class MomentumStrategy(Strategy):
    def __init__(self, lookback: int = 20):
        super().__init__("momentum")
        self.lookback = lookback
        self.prices: list[float] = []

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(float(bar.close))
        if len(self.prices) < self.lookback:
            return

        window = self.prices[-self.lookback:]
        baseline = sum(window[:-1]) / max(len(window) - 1, 1)
        if bar.close > baseline * 1.01:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
''',
}

STRATEGY_TEMPLATE = '''from neleus import OrderSide, Strategy, StrategyContext, Bar


class {class_name}(Strategy):
    def __init__(self):
        super().__init__("{strategy_name}")
        self.prices: list[float] = []

    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(float(bar.close))
        if len(self.prices) < 20:
            return

        average_price = sum(self.prices[-20:]) / 20
        if bar.close > average_price * 1.01:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.01)
'''


def get_project_root(start: Optional[Path] = None) -> Optional[Path]:
    current = (start or Path.cwd()).resolve()
    while current != current.parent:
        if (current / CONFIG_FILE).exists():
            return current
        current = current.parent
    return None


def ensure_project() -> Path:
    root = get_project_root()
    if root is None:
        console.print(f"[red]No {CONFIG_FILE} found in the current directory tree.[/red]")
        raise typer.Exit(1)
    return root


def create_project(
    project_path: Path,
    project_name: str,
    db_backend: str = "none",
    db_dsn: str = "",
    trade_monitoring: bool = False,
    private_key: str = "",
    account_address: str = "",
) -> None:
    project_path.mkdir(parents=True, exist_ok=False)
    for relative_path, content in PROJECT_TEMPLATE.items():
        full_path = project_path / relative_path
        full_path.parent.mkdir(parents=True, exist_ok=True)
        full_path.write_text(
            content.format(
                project_name=project_name,
                created_at=datetime.utcnow().isoformat(),
                db_backend=db_backend,
                db_dsn="",
                trade_monitoring=str(trade_monitoring).lower(),
            )
        )

    _write_local_env(project_path, db_dsn=db_dsn, private_key=private_key, account_address=account_address)


def _write_local_env(
    project_path: Path,
    db_dsn: str = "",
    private_key: str = "",
    account_address: str = "",
) -> None:
    """Write secrets to .env (never committed). Only writes non-empty values."""
    lines: list[str] = []
    if private_key:
        lines.append(f"HYPERLIQUID_SIGNER_PRIVATE_KEY={private_key}\n")
    if account_address:
        lines.append(f"HYPERLIQUID_ACCOUNT_ADDRESS={account_address}\n")
    if db_dsn:
        lines.append(f"NELEUS_DB_DSN={db_dsn}\n")

    if not lines:
        return

    env_path = project_path / ".env"
    if env_path.exists():
        existing = env_path.read_text()
        separator = "" if not existing or existing.endswith("\n") else "\n"
        additions = []
        for line in lines:
            key = line.split("=")[0]
            if f"{key}=" not in existing:
                additions.append(line)
        if additions:
            env_path.write_text(existing + separator + "".join(additions))
        return

    env_path.write_text(
        "# Local secrets — do not commit this file.\n" + "".join(lines)
    )


@app.callback(invoke_without_command=True)
def app_callback(ctx: typer.Context) -> None:
    if ctx.invoked_subcommand is None:
        console.print(render_brand_banner())
        console.print(ctx.get_help())
        raise typer.Exit()


@app.command("new")
def new_project(
    name: str = typer.Argument(..., help="Project directory name."),
    db_backend: str = typer.Option(
        "none",
        "--db-backend",
        help="Storage adapter: none, postgres, or timescale.",
    ),
    db_dsn: str = typer.Option(
        "",
        "--db-dsn",
        help="PostgreSQL DSN, e.g. postgresql://user:pass@localhost/db. "
             "Leave empty to configure via NELEUS_DB_DSN env var later.",
    ),
    trade_monitoring: bool = typer.Option(
        False,
        "--trade-monitoring/--no-trade-monitoring",
        help="Enable automatic order/fill recording to the database.",
    ),
    private_key: str = typer.Option(
        "",
        "--private-key",
        help="Hyperliquid signer private key (hex). Written to .env, never to neleus.toml.",
        envvar="HYPERLIQUID_SIGNER_PRIVATE_KEY",
    ),
    account_address: str = typer.Option(
        "",
        "--account-address",
        help="Hyperliquid account/wallet address (0x...). Written to .env.",
        envvar="HYPERLIQUID_ACCOUNT_ADDRESS",
    ),
):
    """Create a new Neleus project.

    Examples
    --------
    # Minimal (no database)
    neleus new my_bot

    # With live trading credentials
    neleus new my_bot --private-key 0xabc... --account-address 0xdef...

    # With PostgreSQL + trade monitoring
    neleus new my_bot --db-backend postgres --db-dsn postgresql://user:pass@localhost/neleus --trade-monitoring
    """
    if db_backend not in ("none", "postgres", "timescale"):
        console.print(f"[red]--db-backend must be none, postgres, or timescale (got '{db_backend}')[/red]")
        raise typer.Exit(1)

    project_path = Path.cwd() / name
    if project_path.exists():
        console.print(f"[red]Directory already exists:[/red] {project_path}")
        raise typer.Exit(1)

    create_project(
        project_path, name,
        db_backend=db_backend, db_dsn=db_dsn, trade_monitoring=trade_monitoring,
        private_key=private_key, account_address=account_address,
    )
    console.print(render_project_created(name, db_backend=db_backend, trade_monitoring=trade_monitoring))


@app.command("init")
def init_project(
    db_backend: str = typer.Option(
        "none",
        "--db-backend",
        help="Storage adapter: none, postgres, or timescale.",
    ),
    db_dsn: str = typer.Option(
        "",
        "--db-dsn",
        help="PostgreSQL DSN. Leave empty to configure via NELEUS_DB_DSN env var later.",
    ),
    trade_monitoring: bool = typer.Option(
        False,
        "--trade-monitoring/--no-trade-monitoring",
        help="Enable automatic order/fill recording to the database.",
    ),
    private_key: str = typer.Option(
        "",
        "--private-key",
        help="Hyperliquid signer private key (hex). Written to .env, never to neleus.toml.",
        envvar="HYPERLIQUID_SIGNER_PRIVATE_KEY",
    ),
    account_address: str = typer.Option(
        "",
        "--account-address",
        help="Hyperliquid account/wallet address (0x...). Written to .env.",
        envvar="HYPERLIQUID_ACCOUNT_ADDRESS",
    ),
):
    """Initialize the current directory as a Neleus project."""
    if db_backend not in ("none", "postgres", "timescale"):
        console.print(f"[red]--db-backend must be none, postgres, or timescale (got '{db_backend}')[/red]")
        raise typer.Exit(1)

    project_root = Path.cwd()
    config_path = project_root / CONFIG_FILE
    if config_path.exists():
        console.print(f"[yellow]{CONFIG_FILE} already exists.[/yellow]")
        raise typer.Exit(0)

    for relative_path, content in PROJECT_TEMPLATE.items():
        full_path = project_root / relative_path
        if full_path.exists():
            continue
        full_path.parent.mkdir(parents=True, exist_ok=True)
        full_path.write_text(
            content.format(
                project_name=project_root.name,
                created_at=datetime.utcnow().isoformat(),
                db_backend=db_backend,
                db_dsn="",
                trade_monitoring=str(trade_monitoring).lower(),
            )
        )

    _write_local_env(project_root, db_dsn=db_dsn, private_key=private_key, account_address=account_address)

    console.print(render_project_created(project_root.name, initialized=True, db_backend=db_backend, trade_monitoring=trade_monitoring))


@market_app.command("analyze")
def market_analyze(
    symbol: str = typer.Argument(..., help="Hyperliquid symbol, e.g. BTC-PERP."),
    timeframe: str = typer.Option("1h", "--timeframe", "-t", help="1m, 5m, 15m, 1h, 4h, 1d"),
    lookback_bars: int = typer.Option(200, "--lookback-bars", "-n", min=20),
    scope: str = typer.Option("all-perps", "--scope", "-s", help="perps, hip3, spot, or all-perps"),
    dex: Optional[str] = typer.Option(None, "--dex", help="HIP-3 dex name, e.g. flx or xyz"),
    testnet: bool = typer.Option(False, "--testnet", help="Use Hyperliquid testnet."),
    output: str = typer.Option("table", "--output", "-o", help="table or json"),
):
    """Analyze a Hyperliquid market using recent candles and technical indicators."""
    resolved_market = resolve_market_entry(symbol, scope=scope, dex=dex, testnet=testnet)

    with console.status(
        f"[bold cyan]Fetching {resolved_market.name} candles from Hyperliquid ({'testnet' if testnet else 'mainnet'})..."
    ):
        analysis = analyze_market(
            symbol=resolved_market.name,
            timeframe=timeframe,
            lookback_bars=lookback_bars,
            testnet=testnet,
            dex=resolved_market.request_dex or resolved_market.dex,
            display_symbol=resolved_market.name,
        )

    if output == "json":
        console.print_json(data=analysis.to_dict())
        return

    console.print(render_market_analysis(analysis))


@market_app.command("list")
def market_list(
    scope: str = typer.Option("perps", "--scope", "-s", help="perps, hip3, spot, or all-perps"),
    dex: Optional[str] = typer.Option(None, "--dex", help="HIP-3 dex name, e.g. xyz"),
    search: Optional[str] = typer.Option(None, "--search", help="Filter market names."),
    testnet: bool = typer.Option(False, "--testnet", help="Use Hyperliquid testnet."),
    output: str = typer.Option("table", "--output", "-o", help="table or json"),
):
    """List Hyperliquid markets through the Rust bridge."""
    with console.status(
        f"[bold cyan]Loading {scope} markets from Hyperliquid ({'testnet' if testnet else 'mainnet'})..."
    ):
        catalog = list_markets(
            scope=scope,
            dex=dex,
            search=search,
            testnet=testnet,
        )

    if output == "json":
        console.print_json(data=catalog.to_dict())
        return

    console.print(render_market_catalog(catalog))


@market_app.command("search")
def market_search(
    query: str = typer.Argument(..., help="Market search text."),
    scope: str = typer.Option("all-perps", "--scope", "-s", help="perps, hip3, spot, or all-perps"),
    dex: Optional[str] = typer.Option(None, "--dex", help="HIP-3 dex name, e.g. xyz"),
    testnet: bool = typer.Option(False, "--testnet", help="Use Hyperliquid testnet."),
    output: str = typer.Option("table", "--output", "-o", help="table or json"),
):
    """Search the Hyperliquid market catalog without a project."""
    with console.status(
        f"[bold cyan]Searching {scope} markets for '{query}' on {'testnet' if testnet else 'mainnet'}..."
    ):
        catalog = list_markets(
            scope=scope,
            dex=dex,
            search=query,
            testnet=testnet,
        )

    if output == "json":
        console.print_json(data=catalog.to_dict())
        return

    console.print(render_market_catalog(catalog))


@market_app.command("scan")
def market_scan(
    symbols: Optional[str] = typer.Option(
        None,
        "--symbols",
        help="Comma-separated symbol list. Overrides catalog selection.",
    ),
    scope: str = typer.Option("perps", "--scope", "-s", help="perps, hip3, spot, or all-perps"),
    dex: Optional[str] = typer.Option(None, "--dex", help="HIP-3 dex name, e.g. xyz"),
    search: Optional[str] = typer.Option(None, "--search", help="Filter before scanning."),
    timeframe: str = typer.Option("1h", "--timeframe", "-t", help="1m, 5m, 15m, 1h, 4h, 1d"),
    lookback_bars: int = typer.Option(200, "--lookback-bars", "-n", min=50),
    max_markets: int = typer.Option(8, "--max-markets", help="How many markets to analyze."),
    limit: int = typer.Option(8, "--limit", help="How many ranked rows to display."),
    sort_by: str = typer.Option("score", "--sort", help="score, change, volatility, or rsi"),
    testnet: bool = typer.Option(False, "--testnet", help="Use Hyperliquid testnet."),
    output: str = typer.Option("table", "--output", "-o", help="table or json"),
):
    """Run a ranked TA scan without creating a project."""
    symbol_list = None
    if symbols:
        symbol_list = [item.strip() for item in symbols.split(",") if item.strip()]

    with console.status(
        f"[bold cyan]Scanning {scope if symbol_list is None else 'custom'} markets on {'testnet' if testnet else 'mainnet'}..."
    ):
        scan = scan_markets(
            scope=scope,
            dex=dex,
            search=search,
            symbols=symbol_list,
            timeframe=timeframe,
            lookback_bars=lookback_bars,
            max_markets=max_markets,
            limit=limit,
            sort_by=sort_by,
            testnet=testnet,
        )

    if output == "json":
        console.print_json(data=scan.to_dict())
        return

    console.print(render_market_scan(scan))


@market_app.command("book")
def market_book(
    symbol: str = typer.Argument(..., help="Hyperliquid symbol, e.g. BTC-PERP."),
    depth: int = typer.Option(12, "--depth", "-d", min=1, max=25, help="Levels to show per side."),
    poll_ms: int = typer.Option(800, "--poll-ms", help="Max wait between screen refreshes."),
    scope: str = typer.Option("all-perps", "--scope", "-s", help="perps, hip3, spot, or all-perps"),
    dex: Optional[str] = typer.Option(None, "--dex", help="HIP-3 dex name, e.g. flx or xyz"),
    testnet: bool = typer.Option(False, "--testnet", help="Use Hyperliquid testnet."),
):
    """Show a live Hyperliquid L2 order book in the terminal."""
    resolved_market = resolve_market_entry(symbol, scope=scope, dex=dex, testnet=testnet)

    with console.status(
        f"[bold cyan]Connecting live L2 book for {resolved_market.name} on {'testnet' if testnet else 'mainnet'}..."
    ):
        stream, first_update, subscribed_coin = open_l2_book_stream(
            resolved_market.name,
            testnet=testnet,
            timeout_ms=max(poll_ms * 3, 3000),
        )

    if stream is None or first_update is None:
        console.print(
            f"[red]Timed out waiting for the first order book update for {resolved_market.name}.[/red]"
        )
        raise typer.Exit(1)

    try:
        with Live(
            render_l2_book(
                first_update,
                depth=depth,
                network="testnet" if testnet else "mainnet",
                display_symbol=resolved_market.name,
                subscribed_symbol=subscribed_coin,
            ),
            console=console,
            refresh_per_second=4,
        ) as live:
            while True:
                update = stream.next_update(timeout_ms=max(poll_ms, 100))
                if update is None:
                    continue
                live.update(
                    render_l2_book(
                        update,
                        depth=depth,
                        network="testnet" if testnet else "mainnet",
                        display_symbol=resolved_market.name,
                        subscribed_symbol=subscribed_coin,
                    ),
                    refresh=True,
                )
    except KeyboardInterrupt:
        console.print("\n[yellow]Stopped live order book.[/yellow]")


@app.command("backtest")
def backtest(
    strategy: Optional[str] = typer.Option(None, "--strategy", "-s", help="Strategy name."),
    start: Optional[str] = typer.Option(None, "--start", help="YYYY-MM-DD"),
    end: Optional[str] = typer.Option(None, "--end", help="YYYY-MM-DD"),
    capital: Optional[float] = typer.Option(None, "--capital", "-c"),
):
    """Run a backtest for a project strategy."""
    project_root = ensure_project()
    runner = BacktestRunner(project_root)
    strategies = discover_strategies(project_root)
    if not strategies:
        console.print("[red]No strategies found in strategies/[/red]")
        raise typer.Exit(1)

    strategy_name = strategy or strategies[0]["name"]
    with console.status(f"[bold cyan]Running backtest for {strategy_name}..."):
        results = asyncio.run(
            runner.run_backtest(
                strategy_name=strategy_name,
                start_date=start,
                end_date=end,
                initial_capital=capital,
            )
        )

    if not results:
        console.print("[yellow]No backtest results returned.[/yellow]")
        raise typer.Exit(1)

    for result_name, result in results.items():
        console.print(render_backtest_result(result_name, result))


@app.command("run")
def run(
    mode: str = typer.Option("once", "--mode", help="once or daemon"),
    strategy: Optional[str] = typer.Option(None, "--strategy", "-s", help="Strategy name."),
    symbol: Optional[str] = typer.Option(None, "--symbol", help="Override project symbol."),
    timeframe: Optional[str] = typer.Option(None, "--timeframe", "-t"),
    lookback_bars: Optional[int] = typer.Option(None, "--lookback-bars", "-n"),
    interval_seconds: Optional[int] = typer.Option(None, "--interval-seconds", "-i"),
    testnet: Optional[bool] = typer.Option(None, "--testnet/--mainnet"),
    live_trading: bool = typer.Option(
        False,
        "--live",
        help="Execute orders on-chain via HyperliquidTrader. "
             "Requires HYPERLIQUID_SIGNER_PRIVATE_KEY in .env or environment.",
    ),
):
    """Run a project strategy once or keep it running in daemon mode."""
    project_root = ensure_project()

    # Resolve private key when live trading is requested
    private_key: Optional[str] = None
    if live_trading:
        project_config = load_project_config(project_root / CONFIG_FILE)
        creds = get_hyperliquid_credentials(project_config)
        private_key = creds.get("signer_private_key")
        if not private_key:
            console.print(
                "[red]--live requires a private key.[/red] "
                "Set HYPERLIQUID_SIGNER_PRIVATE_KEY in .env or pass --private-key to neleus new/init."
            )
            raise typer.Exit(1)
        console.print("[yellow]Live trading enabled — real orders will be submitted.[/yellow]")

    if mode == "once":
        with console.status("[bold cyan]Running project once..."):
            result = run_project_once(
                project_path=project_root,
                strategy_name=strategy,
                symbol=symbol,
                timeframe=timeframe,
                lookback_bars=lookback_bars,
                testnet=testnet,
                private_key=private_key,
            )
        console.print(render_runtime_result(result, mode="once"))
        return

    if mode != "daemon":
        console.print("[red]Mode must be 'once' or 'daemon'.[/red]")
        raise typer.Exit(1)

    project_config = load_project_config(project_root / CONFIG_FILE)
    market_config = project_config.get("market", {})
    runtime_config = project_config.get("runtime", {})
    hyperliquid_config = project_config.get("hyperliquid", {})
    dashboard = DaemonDashboard(
        strategy=strategy or "auto",
        symbol=symbol or market_config.get("symbol", "BTC-PERP"),
        timeframe=timeframe or market_config.get("timeframe", "1h"),
        poll_interval_seconds=interval_seconds or int(runtime_config.get("poll_interval_seconds", 60)),
        network="testnet"
        if (testnet if testnet is not None else hyperliquid_config.get("testnet", False))
        else "mainnet",
    )

    async def _log_iteration(result) -> None:
        if dashboard.strategy == "auto":
            dashboard.strategy = result.strategy
        dashboard.update(result)
        live.update(dashboard.render(), refresh=True)

    try:
        with Live(dashboard.render(), console=console, refresh_per_second=4) as live:
            asyncio.run(
                run_project_daemon(
                    project_path=project_root,
                    strategy_name=strategy,
                    symbol=symbol,
                    timeframe=timeframe,
                    lookback_bars=lookback_bars,
                    poll_interval_seconds=interval_seconds,
                    testnet=testnet,
                    on_iteration=_log_iteration,
                    private_key=private_key,
                )
            )
    except KeyboardInterrupt:
        console.print("\n[yellow]Stopped daemon runtime.[/yellow]")


@strategy_app.command("list")
def strategy_list():
    """List strategy files in the current project."""
    project_root = ensure_project()
    strategies = discover_strategies(project_root)
    if not strategies:
        console.print("[yellow]No strategies found.[/yellow]")
        return

    console.print(render_strategy_list(strategies))


@strategy_app.command("new")
def strategy_new(
    name: str = typer.Argument(..., help="Strategy name."),
):
    """Create a new strategy file."""
    project_root = ensure_project()
    strategy_name = name.lower().replace("-", "_").replace(" ", "_")
    class_name = "".join(part.title() for part in strategy_name.split("_"))
    file_path = project_root / "strategies" / f"{strategy_name}.py"
    if file_path.exists():
        console.print(f"[red]Strategy already exists:[/red] {file_path}")
        raise typer.Exit(1)

    file_path.parent.mkdir(parents=True, exist_ok=True)
    file_path.write_text(
        STRATEGY_TEMPLATE.format(
            class_name=class_name,
            strategy_name=strategy_name,
        )
    )
    console.print(f"[green]Created[/green] {file_path}")


@strategy_app.command("show")
def strategy_show(
    name: str = typer.Argument(..., help="Strategy name."),
):
    """Show strategy source code."""
    project_root = ensure_project()
    file_path = project_root / "strategies" / f"{name}.py"
    if not file_path.exists():
        console.print(f"[red]Strategy not found:[/red] {name}")
        raise typer.Exit(1)

    console.print(Syntax(file_path.read_text(), "python", line_numbers=True))


@app.command("info")
def info():
    """Show current project information."""
    project_root = get_project_root()
    if project_root is None:
        console.print(render_about_panel())
        raise typer.Exit()

    config = load_project_config(project_root / CONFIG_FILE)
    console.print(render_project_info(project_root, config, discover_strategies(project_root), CONFIG_FILE))


@app.command("about")
def about():
    """Show Neleus branding, links, and command overview."""
    console.print(render_about_panel())


@app.command("version")
def version():
    """Show CLI version."""
    console.print(render_brand_banner(compact=True))
    console.print(f"neleus {__version__}")


# ---------------------------------------------------------------------------
# Database adapter commands
# ---------------------------------------------------------------------------

@db_app.command("status")
def db_status():
    """Check the database connection configured in the current project."""
    project_root = ensure_project()
    config = load_project_config(project_root / CONFIG_FILE)
    db_cfg = get_db_config(config)

    if db_cfg.backend == "none":
        console.print(
            "[yellow]Database backend is set to 'none'.[/yellow]\n"
            "Run [bold]neleus db status --help[/bold] or edit [bold]neleus.toml[/bold] to configure one."
        )
        raise typer.Exit(0)

    dsn = db_cfg.dsn
    if not dsn:
        console.print(
            "[red]database.dsn is empty.[/red] Set it in neleus.toml or via NELEUS_DB_DSN."
        )
        raise typer.Exit(1)

    console.print(
        f"[cyan]Backend:[/cyan] {db_cfg.backend}\n"
        f"[cyan]DSN:[/cyan]     {_mask_dsn(dsn)}\n"
        f"[cyan]Pool:[/cyan]    {db_cfg.pool_size}\n"
        f"[cyan]Monitor:[/cyan] {'enabled' if db_cfg.trade_monitoring else 'disabled'}\n"
    )

    with console.status("[bold cyan]Connecting to database..."):
        try:
            _probe_db(dsn)
            console.print("[bold green]Connection OK.[/bold green]")
        except Exception as exc:
            console.print(f"[bold red]Connection FAILED:[/bold red] {exc}")
            raise typer.Exit(1)


@db_app.command("init")
def db_init():
    """Initialize database schema for the current project.

    Creates hl_orders and hl_fills tables (trade monitoring) and, when the
    backend is 'timescale', also creates the full TimescaleDB market-data
    hypertables.
    """
    project_root = ensure_project()
    config = load_project_config(project_root / CONFIG_FILE)
    db_cfg = get_db_config(config)

    if db_cfg.backend == "none":
        console.print(
            "[yellow]Database backend is 'none'. Nothing to initialize.[/yellow]\n"
            "Edit [bold]database.backend[/bold] in neleus.toml first."
        )
        raise typer.Exit(0)

    dsn = db_cfg.dsn
    if not dsn:
        console.print("[red]database.dsn is empty.[/red] Set it in neleus.toml or via NELEUS_DB_DSN.")
        raise typer.Exit(1)

    console.print(f"[cyan]Backend:[/cyan] {db_cfg.backend}   [cyan]DSN:[/cyan] {_mask_dsn(dsn)}")

    with console.status("[bold cyan]Initializing schema..."):
        try:
            _init_schema(db_cfg)
            console.print("[bold green]Schema initialized.[/bold green]")
        except Exception as exc:
            console.print(f"[bold red]Schema init FAILED:[/bold red] {exc}")
            raise typer.Exit(1)


# ---------------------------------------------------------------------------
# DB helpers (imported lazily so they don't block the CLI when DB is unused)
# ---------------------------------------------------------------------------

def _mask_dsn(dsn: str) -> str:
    """Redact the password portion of a DSN for display."""
    import re
    return re.sub(r"(://[^:]+:)[^@]+(@)", r"\1***\2", dsn)


def _probe_db(dsn: str) -> None:
    """Verify connectivity by constructing a TradeMonitor (schema init included)."""
    from ..types import TradeMonitor
    # The Rust constructor is blocking and raises on connection failure.
    TradeMonitor(connection_string=dsn, testnet=False, pool_size=1)


def _init_schema(db_cfg) -> None:
    """Initialize the project database schema using the configured adapter."""
    from ..types import TradeMonitor, TimescaleConfig, TimescaleStore

    # Trade-monitoring tables work with plain PostgreSQL (no TimescaleDB needed).
    TradeMonitor(
        connection_string=db_cfg.dsn,
        testnet=False,
        pool_size=db_cfg.pool_size,
    )

    if db_cfg.backend == "timescale":
        cfg = TimescaleConfig(
            connection_string=db_cfg.dsn,
            pool_size=db_cfg.pool_size,
            batch_size=db_cfg.batch_size,
            flush_interval_ms=db_cfg.flush_interval_ms,
        )
        TimescaleStore(config=cfg)


# ---------------------------------------------------------------------------

def main() -> None:
    app()


if __name__ == "__main__":
    main()
